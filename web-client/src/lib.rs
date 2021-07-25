mod wasm_instant;

use wasm_bindgen::prelude::*;

use wasm_instant::WasmInstant;
use std::time::Duration;
use serde::Deserialize;

use crossy_multi_core::*;
use crossy_multi_core::game::PlayerId;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
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
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : u32, server_time_us : u32, estimated_latency : u32) -> Self {
        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, vec![], crossy_ruleset::CrossyRulesetFST::start());

        // Estimate server start
        let server_start = WasmInstant::now() - Duration::from_micros((server_time_us + estimated_latency) as u64);

        log!("CONSTRUCTING : Estamated t0 {:?} server t1 {} estimated latency {}", server_start, server_time_us, estimated_latency);
        
        Client {
            timeline,
            last_tick : server_time_us,
            last_server_tick : None,
            server_start,
            local_player_info : None,
            // TODO proper ready state
            ready_state : true,
        } 
    }

    pub fn join(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            buffered_input : Input::None,
        })
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


        self.local_player_info.as_mut().map(|local_player| {
            let mut input = Input::None;
            if (can_move && local_player.buffered_input != Input::None)
            {
                input = local_player.buffered_input;
                local_player.buffered_input = Input::None;
            }

            player_inputs.set(local_player.player_id, input);
        });

        // Tick 

        self.timeline
            .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);

        if (self.timeline.top_state().frame_id.floor() as u32 % 60) == 0
        {
            log!("{:?}", self.timeline.top_state());
        }
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        if let Some(deserialized) = try_deserialize_server_tick(server_tick)
        {
            self.recv_internal(&deserialized);
        }
    }

    fn recv_internal(&mut self, server_tick : &interop::ServerTick)
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

        self.last_server_tick = Some(server_tick.latest.time_us);
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
        let players = self.get_players();
        serde_json::to_string(&players).unwrap()
    }

    fn get_players(&self) -> Vec<PlayerState>
    {
        self.timeline.top_state().get_valid_player_states()
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
        let us = self.last_server_tick?;
        let state_before = self.timeline.get_state_before_eq_us(us)?;
        Some(state_before.get_rule_state())
    }

    pub fn get_rows_json(&mut self) -> String {
        serde_json::to_string(&self.get_rows()).unwrap()
    }

    fn get_rows(&mut self) -> Vec<(i32, map::Row)> {
        let mut vec = Vec::with_capacity(32);
        for i in 0..(160/8) {
            let y = i;
            vec.push((y as i32, self.timeline.map.get_row(y).clone()));
        }
        vec
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