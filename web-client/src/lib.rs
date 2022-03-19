#![allow(unused_parens)]

use wasm_bindgen::prelude::*;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

mod wasm_instant;
mod ai;

use std::time::Duration;
use std::cell::RefCell;

use std::collections::VecDeque;
use wasm_instant::WasmInstant;
use serde::Deserialize;

use crossy_multi_core::*;
use crossy_multi_core::game::PlayerId;

struct ConsoleDebugLogger();
impl crossy_multi_core::DebugLogger for ConsoleDebugLogger {
    fn log(&self, logline: &str) {
        log!("{}", logline);
    }
}

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug)]
pub struct LocalPlayerInfo {
    player_id: game::PlayerId,
    buffered_input : Input,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Client {
    timeline: timeline::Timeline,
    server_start: WasmInstant,
    last_tick: u32,
    // The last server tick we received
    last_server_tick: Option<u32>,
    local_player_info : Option<LocalPlayerInfo>,
    ready_state : bool,

    // This seems like a super hacky solution
    trusted_rule_state : Option<crossy_ruleset::CrossyRulesetFST>,

    queued_server_messages : VecDeque<interop::ServerTick>,

    ai_agent : Option<RefCell<Box<dyn ai::AIAgent>>>,
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : &str, server_time_us : u32, estimated_latency : u32) -> Self {
        // Setup statics
        console_error_panic_hook::set_once();
        crossy_multi_core::set_debug_logger(Box::new(ConsoleDebugLogger()));

        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, vec![], crossy_ruleset::CrossyRulesetFST::start());

        // Estimate server start
        let server_start = WasmInstant::now() - Duration::from_micros((server_time_us + estimated_latency) as u64);

        log!("CONSTRUCTING : Estimated t0 {:?} server t1 {} estimated latency {}", server_start, server_time_us, estimated_latency);

        Client {
            timeline,
            last_tick : server_time_us,
            last_server_tick : None,
            server_start,
            local_player_info : None,
            // TODO proper ready state
            ready_state : false,
            trusted_rule_state: None,
            queued_server_messages: VecDeque::new(),
            ai_agent : None,
        } 
    }

    pub fn join(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            buffered_input : Input::None,
        })
    }

    pub fn get_ready_state(&self) -> bool {
        self.ready_state
    }

    pub fn set_ready_state(&mut self, state : bool) {
        self.ready_state = state;
    }

    pub fn buffer_input_json(&mut self, input_json : &str) {
        let input = serde_json::from_str(input_json).map_err(|e| log!("{} {:?}", input_json, e)).unwrap_or(Input::None);
        self.buffer_input(input);
    }

    fn buffer_input(&mut self, input : Input) {
        self.local_player_info.as_mut().map(|x| {
            //if input != Input::None {
            if input != Input::None && x.buffered_input == Input::None {
                x.buffered_input = input;
            }
        });
    }

    pub fn tick(&mut self) {
        let current_time = self.server_start.elapsed();
        self.last_tick = current_time.as_micros() as u32;

        // Move buffered input to input
        // awkward because of mut / immut borrowing
        let mut player_inputs = self.timeline.get_last_player_inputs();

        let mut can_move = false;
        let local_player_id = self.local_player_info.as_ref().map(|x| x.player_id);
        local_player_id.map(|id| {
            self.timeline.top_state().get_player(id).map(|player| {
                can_move = player.can_move();
            });
        });

        if (self.local_player_info.is_some())
        {
            let mut local_input = Input::None;

            if (can_move)
            {
                if let Some(ai_refcell) = &self.ai_agent {
                    let mut ai = ai_refcell.borrow_mut();
                    local_input = ai.think(&self.timeline.top_state(), &self.timeline.map);
                }
                else 
                {
                    let local_player_info = self.local_player_info.as_mut().unwrap();
                    if (local_player_info.buffered_input != Input::None)
                    {
                        local_input = local_player_info.buffered_input;
                        local_player_info.buffered_input = Input::None;
                    }
                }
            }

            player_inputs.set(self.local_player_info.as_ref().unwrap().player_id, local_input);
        }

        // Tick 
        let current_time_us = current_time.as_micros() as u32;
        self.timeline
            .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);

        // BIGGEST hack
        // dont have the energy to explain, but the timing is fucked and just want to demo something.
        let mut server_tick_it = None;
        while  {
            self.queued_server_messages.back().map(|x| x.latest.time_us < current_time_us).unwrap_or(false)
        }
        {server_tick_it = self.queued_server_messages.pop_back();}

        //if (self.queued_server_messages.len() > 0) {
        {
            //log!("DROPPED {} SERVER MESSAGES AS THEY ARE IN THE FUTURE, GOT TICK {}", self.queued_server_messages.len(), server_tick_it.is_some());
        }

        if let Some(server_tick) = server_tick_it {
            self.process_server_message(&server_tick);
        }

        if (self.timeline.top_state().frame_id.floor() as u32 % 15) == 0
        {
            //log!("{:?}", self.timeline.top_state().get_rule_state());
            //log!("{:?}", self.timeline.top_state());
        }
    }

    fn process_server_message(&mut self, server_tick : &interop::ServerTick)
    {
        // If we have had a "major change" instead of patching up the current state we perform a full reset
        // At the moment a major change is either:
        //   We have moved between game states (eg the round ended)
        //   A player has joined or left
        let mut should_reset = self.trusted_rule_state.as_ref().map(|x| !x.same_variant(&server_tick.rule_state)).unwrap_or(false);
        should_reset |= self.timeline.top_state().player_states.count_populated() != server_tick.latest.states.len();

        if (should_reset) {
            self.timeline = timeline::Timeline::from_server_parts_exact_seed(
                self.timeline.map.get_seed(),
                server_tick.latest.time_us,
                server_tick.latest.states.clone(),
                server_tick.rule_state.clone());
        }
        else
        {
            match self.local_player_info.as_ref()
            {
                Some(lpi) => {
                    if (self.timeline.top_state().get_player(lpi.player_id)).is_none()
                    {
                        // Edge case
                        // First tick with the player
                        // we need to take state from server
                        self.timeline.propagate_state(
                            &server_tick.latest,
                            Some(&server_tick.rule_state),
                            Some(&server_tick.latest),
                            None);
                    }
                    else
                    {
                        self.timeline.propagate_state(
                            &server_tick.latest,
                            Some(&server_tick.rule_state),
                            server_tick.last_client_sent.get(lpi.player_id),
                            Some(lpi.player_id));
                    }
                }
                _ => {
                    self.timeline.propagate_state(
                        &server_tick.latest,
                        Some(&server_tick.rule_state),
                        None,
                        None);
                }
            }
        }

        self.last_server_tick = Some(server_tick.latest.time_us);
        self.trusted_rule_state = Some(server_tick.rule_state.clone());
    }

    fn get_round_id(&self) -> u8 {
        self.trusted_rule_state.as_ref().map(|x| x.get_round_id()).unwrap_or(0)
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        if let Some(deserialized) = try_deserialize_server_tick(server_tick)
        {
            self.recv_internal(deserialized);
        }
    }

    fn recv_internal(&mut self, server_tick : interop::ServerTick)
    {
        self.queued_server_messages.push_front(server_tick);
    }

    pub fn get_client_message(&self) -> Vec<u8>
    {
        let message = self.get_client_message_internal();
        flexbuffers::to_vec(message).unwrap()

    }

    fn get_client_message_internal(&self) -> interop::CrossyMessage
    {
        //let input = self.local_player_info.as_ref().map(|x| x.input).unwrap_or(Input::None);
        //let mut input = self.timeline.top_state().
        let input = self.local_player_info.as_ref().map(|x| self.timeline.top_state().player_inputs.get(x.player_id)).unwrap_or(Input::None);
        interop::CrossyMessage::ClientTick(interop::ClientTick {
            time_us: self.last_tick,
            input: input,
            lobby_ready : self.ready_state,
        })
    }

    pub fn get_players_json(&self) -> String
    {
        let time_us = self.timeline.top_state().time_us;
        let players : Vec<_> = self.timeline.top_state().get_valid_player_states()
            .iter()
            .map(|x| x.to_public(self.get_round_id(), time_us, &self.timeline.map))
            .collect();
        serde_json::to_string(&players).unwrap()
    }

    // Return -1 if no local player
    pub fn get_local_player_id(&self) -> i32 {
        self.local_player_info.as_ref().map(|x| x.player_id.0 as i32).unwrap_or(-1)
    }

    pub fn get_rule_state_json(&self) -> String {
        match self.get_latest_server_rule_state() {
            Some(x) => {
                serde_json::to_string(x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_latest_server_rule_state(&self) -> Option<&crossy_ruleset::CrossyRulesetFST> {
        /*
        let us = self.last_server_tick? + 1;
        let state_before = self.timeline.get_state_before_eq_us(us)?;
        Some(state_before.get_rule_state())
        */
        self.trusted_rule_state.as_ref()
    }

    pub fn get_rows_json(&mut self) -> String {
        serde_json::to_string(&self.get_rows()).unwrap()
    }

    fn get_rows(&mut self) -> Vec<(i32, map::Row)> {
        let mut vec = Vec::with_capacity(32);
        let screen_y = self.trusted_rule_state.as_ref().map(|x| x.get_screen_y()).unwrap_or(0);
        let range_min = screen_y;
        let range_max = (screen_y + 160/8 + 6).min(160/8);
        for i in range_min..range_max {
            let y = i;
            vec.push((y as i32, self.timeline.map.get_row(self.get_round_id(), y).clone()));
        }
        vec
    }

    pub fn get_cars_json(&self) -> String {
        let cars = self.timeline.map.get_cars(self.get_round_id(), self.timeline.top_state().time_us);
        serde_json::to_string(&cars).unwrap()
    }

    pub fn get_lillipads_json(&self) -> String {
        let lillipads = self.timeline.map.get_lillipads(self.get_round_id(), self.timeline.top_state().time_us);
        serde_json::to_string(&lillipads).unwrap()
    }

    pub fn player_alive(&self, player_id : u32) -> bool {
        // We have to be careful here.
        // We dont want to tell the client a player is dead if they could possibly "come back alive".
        // For remote players we want to wait for confirmation from the server.
        // For local player we can probably make this decision earlier. (Weird edge case where player pushing you in gets interrupted before they can?)

        self.get_latest_server_rule_state().map(|x| {
            x.get_player_alive(PlayerId(player_id as u8))
        }).unwrap_or(false)
    }

    pub fn is_river(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::River(_) => true,
            _ => false,
        }
    }

    pub fn is_path(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::Path(_) => true,
            map::RowType::Stands() => true,
            map::RowType::StartingBarrier() => true,
            _ => false,
        }
    }

    pub fn set_ai(&mut self, ai_config : &str) {
        if (self.local_player_info.is_none())
        {
            log!("No local player to set ai on");
            return;
        }

        let local_player_id = self.local_player_info.as_ref().unwrap().player_id;

        let lower = ai_config.to_lowercase();
        match lower.as_str() {
            "none" => {
                log!("Setting ai agent to none");
                self.ai_agent = None;
            },
            "go_up" => {
                log!("Setting ai agent to 'go_up'");
                self.ai_agent = Some(RefCell::new(Box::new(ai::go_up::GoUpAI::new(local_player_id))));
            },
            _ => {
                log!("Unknown ai agent {}", ai_config);
            }
        }
    }

    fn get_ai_drawstate(&self) -> Option<ai::AIDrawState> {
        if let Some(x) = self.local_player_info.as_ref() {
            if (!self.player_alive(x.player_id.0 as u32)) {
                return None;
            }
        }

        self.ai_agent.as_ref().map(|x| x.borrow().get_drawstate().clone())
    }

    pub fn get_ai_drawstate_json(&self) -> String {
        match self.get_ai_drawstate() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_lilly_drawstate(&self) -> Option<Vec<LillyOverlay>> {
        self.local_player_info.as_ref().and_then(|x| {
            if (!self.player_alive(x.player_id.0 as u32)) {
                None
            }
            else {
                let top_state = self.timeline.top_state();
                top_state.get_player(x.player_id).and_then(|player| {
                    match &player.move_state {
                        player::MoveState::Stationary => {
                            let precise_coords = match &player.pos {
                                Pos::Coord(coord_pos) => {
                                    coord_pos.to_precise()
                                },
                                Pos::Lillipad(lilly_id) => {
                                    let x = self.timeline.map.get_lillipad_screen_x(top_state.time_us, &lilly_id);
                                    PreciseCoords {
                                        x,
                                        y : lilly_id.y,
                                    }
                                },
                            };

                            let lilly_moves = get_lilly_moves(&precise_coords, top_state.get_round_id(), top_state.time_us, &self.timeline.map);
                            Some(lilly_moves)

                        }
                        _ => {
                            None
                        }
                    }
                })
            }
        })
    }

    pub fn get_lilly_drawstate_json(&self) -> String {
        match self.get_lilly_drawstate() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
struct LillyOverlay {
    precise_coords : PreciseCoords,
    input : Input,
}

fn get_lilly_moves(initial_pos : &PreciseCoords, round_id : u8, time_us : u32, map : &map::Map) -> Vec<LillyOverlay>
{
    let mut moves = vec![];

    for input in &ALL_INPUTS {
        let applied = initial_pos.apply_input(*input);
        if let Some(lilly) = map.lillipad_at_pos(round_id, time_us, applied) {
            let screen_x = map.get_lillipad_screen_x(time_us, &lilly);
            moves.push(LillyOverlay {
                precise_coords: PreciseCoords {
                    x : screen_x,
                    y : applied.y,
                },
                input: *input,
            });
        }
    }

    moves
}

fn try_deserialize_server_tick(buffer : &[u8]) -> Option<interop::ServerTick>
{
    let reader = flexbuffers::Reader::get_root(buffer).map_err(|e| log!("{:?}", e)).ok()?;
    let message = interop::CrossyMessage::deserialize(reader).map_err(|e| log!("{:?}", e)).ok()?;
    match message {
        interop::CrossyMessage::ServerTick(tick) => Some(tick),
        _ => None
    }
}