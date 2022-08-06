#![allow(unused_parens)]

use wasm_bindgen::prelude::*;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
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
use crossy_multi_core::map::river::RiverSpawnTimes;
use crossy_multi_core::crossy_ruleset::AliveState;

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
    client_start : WasmInstant,
    server_start: WasmInstant,
    estimated_latency_us : f32,

    timeline: timeline::Timeline,
    last_tick: u32,
    // The last server tick we received
    last_server_tick: Option<u32>,
    local_player_info : Option<LocalPlayerInfo>,
    ready_state : bool,

    // This seems like a super hacky solution
    trusted_rule_state : Option<crossy_ruleset::CrossyRulesetFST>,

    queued_server_messages : VecDeque<interop::ReceivedServerTick>,
    queued_time_info : Option<interop::TimeRequestEnd>,

    ai_agent : Option<RefCell<Box<dyn ai::AIAgent>>>,
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : &str, server_time_us : u32, estimated_latency_us : u32) -> Self {
        // Setup statics
        console_error_panic_hook::set_once();
        crossy_multi_core::set_debug_logger(Box::new(ConsoleDebugLogger()));

        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, vec![], crossy_ruleset::CrossyRulesetFST::start());

        // Estimate server start
        let client_start = WasmInstant::now();
        let server_start = client_start - Duration::from_micros((server_time_us + estimated_latency_us) as u64);

        log!("CONSTRUCTING : Estimated t0 {:?} server t1 {} estimated latency {}", server_start, server_time_us, estimated_latency_us);

        Client {
            timeline,
            last_tick : server_time_us,
            last_server_tick : None,
            client_start,
            server_start,
            estimated_latency_us : estimated_latency_us as f32,
            local_player_info : None,
            // TODO proper ready state
            ready_state : false,
            trusted_rule_state: None,
            queued_server_messages: Default::default(),
            queued_time_info: Default::default(),
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
        if (self.timeline.top_state().time_us > current_time_us)
        {
            log!("OH NO WE ARE IN THE PAST!");
        }
        else
        {
            self.timeline
                .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);
        }

        // BIGGEST hack
        // dont have the energy to explain, but the timing is fucked and just want to demo something.
        let mut server_tick_it = None;
        while  {
            self.queued_server_messages.back().map(|x| x.server_tick.latest.time_us < current_time_us).unwrap_or(false)
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

    fn process_server_message(&mut self, server_tick_rec : &interop::ReceivedServerTick)
    {
        let server_tick = &server_tick_rec.server_tick;
        // DAN HACK
        // from branch "latency-approximation"
        {
            //let lerp_target = 
            /*
            let current_approx_server_time_at_receive = received_time.duration_since(self.server_start).as_micros() as u32;

            let new_latency_measurement = received_time.saturating_duration_since(server_tick.exact_send_server_time_us);
            self.estimated_latency_us = dan_lerp(self.estimated_latency_us, new_latency_measurement, 50.);

            let delta_us = server_time_now_approx_us as i32 - estimated_server_time_prev_us as i32;

            let lerp_target = estimated_server_time_prev_us as f32 - server_tick.exact_send_server_time_us as f32;
            self.server_start = WasmInstant::now() - Duration::from_micros((server_tick.exact_send_server_time_us + self.estimated_latency_us as u32) as u64);
            //log!("Estimated latency {}ms | lerping_towards {}", self.estimated_latency_us / 1000., lerp_target as f32 / 1000.);
            */
        }

        if let Some(time_request_end) = self.queued_time_info.take() {
            //log!("Applying time ease {:?}", time_request_end);
            let t0 = time_request_end.client_send_time_us as i64;
            let t1 = time_request_end.server_receive_time_us as i64;
            let t2 = time_request_end.server_send_time_us as i64;
            let t3 = time_request_end.client_receive_time_us as i64;

            let total_time_in_flight = t3 - t0;
            let total_time_on_server = t2 - t1;
            let ed = (total_time_in_flight - total_time_on_server) / 2;
            //let ed = ((t1 - t0).abs() + (t3 - t2).abs()) / 2;
            //log!("ed {} - time_in_flight {} - time_on_server {}", ed, total_time_in_flight, total_time_on_server);
            self.estimated_latency_us = dan_lerp(self.estimated_latency_us, ed as f32, 50.);
            //log!("estimated latency {}us", self.estimated_latency_us);

            let estimated_server_time_us = server_tick.exact_send_server_time_us + self.estimated_latency_us as u32;

            let new_server_start = self.client_start + Duration::from_micros(server_tick_rec.client_receive_time_us as u64) - Duration::from_micros(estimated_server_time_us as u64);
            self.server_start = WasmInstant(dan_lerp(self.server_start.0 as f32, new_server_start.0 as f32, 50.) as i128);


            //self.server_start = WasmInstant::now() - Duration::from_micros((server_tick.exact_send_server_time_us + self.estimated_latency_us as u32) as u64);
            //log!("{:#?}", time_request_end);
            //let server_start = client_start - Duration::from_micros((server_time_us + estimated_latency_us) as u64);
        }


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

            // Reset ready state when we are not in the lobby
            match (server_tick.rule_state)
            {
                crossy_ruleset::CrossyRulesetFST::Lobby(_) => {},
                _ => {self.ready_state = false}
            }
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

    fn get_river_spawn_times(&self) -> &RiverSpawnTimes {
        self.trusted_rule_state.as_ref().map(|x| x.get_river_spawn_times()).unwrap_or(&crossy_multi_core::map::river::EMPTY_RIVER_SPAWN_TIMES)
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        if let Some(deserialized) = try_deserialize_message(server_tick)
        {
            self.recv_internal(deserialized);
        }
    }

    fn recv_internal(&mut self, message : interop::CrossyMessage)
    {
        let client_receive_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        match message {
            interop::CrossyMessage::TimeResponsePacket(time_info) => {
                self.queued_time_info = Some(interop::TimeRequestEnd {
                    client_receive_time_us,
                    client_send_time_us : time_info.client_send_time_us,
                    server_receive_time_us : time_info.server_receive_time_us,
                    server_send_time_us : time_info.server_send_time_us,
                });

                //log!("Got time response, {:#?}", self.queued_time_info);
            },
            interop::CrossyMessage::ServerTick(server_tick) => {
                let with_time = interop::ReceivedServerTick {
                    client_receive_time_us,
                    server_tick,
                };

                self.queued_server_messages.push_front(with_time);
            }
            _ => {},
        }
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

    pub fn get_time_request(&self) -> Vec<u8>
    {
        let message = self.get_time_request_internal();
        flexbuffers::to_vec(message).unwrap()

    }

    fn get_time_request_internal(&self) -> interop::CrossyMessage
    {
        let client_send_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        interop::CrossyMessage::TimeRequestPacket(interop::TimeRequestPacket {
            client_send_time_us,
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
        let lillipads = self.timeline.map.get_lillipads(self.get_round_id(), self.timeline.top_state().time_us, self.get_river_spawn_times());
        serde_json::to_string(&lillipads).unwrap()
    }

    pub fn player_alive_state_json(&self, player_id : u32) -> String {
        serde_json::to_string(&self.player_alive_state(player_id)).unwrap()
    }

    fn player_alive_state(&self, player_id : u32) -> AliveState
    {
        // We have to be careful here.
        // We dont want to tell the client a player is dead if they could possibly "come back alive".
        // For remote players we want to wait for confirmation from the server.
        // For local player we can probably make this decision earlier. (Weird edge case where player pushing you in gets interrupted before they can?)

        self.get_latest_server_rule_state().map(|x| {
            x.get_player_alive(PlayerId(player_id as u8))
        }).unwrap_or(AliveState::Unknown)
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
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
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
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
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

                            let lilly_moves = get_lilly_moves(&precise_coords, self.get_river_spawn_times(), top_state.get_round_id(), top_state.time_us, &self.timeline.map);
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

fn get_lilly_moves(initial_pos : &PreciseCoords, spawn_times : &RiverSpawnTimes, round_id : u8, time_us : u32, map : &map::Map) -> Vec<LillyOverlay>
{
    let mut moves = vec![];

    for input in &ALL_INPUTS {
        let applied = initial_pos.apply_input(*input);
        if let Some(lilly) = map.lillipad_at_pos(round_id, spawn_times, time_us, applied) {
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

fn try_deserialize_message(buffer : &[u8]) -> Option<interop::CrossyMessage>
{
    let reader = flexbuffers::Reader::get_root(buffer).map_err(|e| log!("{:?}", e)).ok()?;
    interop::CrossyMessage::deserialize(reader).map_err(|e| log!("{:?}", e)).ok()
}

fn dan_lerp(x0 : f32, x : f32, k : f32) -> f32 {
    (x0 * (k-1.0) + x) / k
}