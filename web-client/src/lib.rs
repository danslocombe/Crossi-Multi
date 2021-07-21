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
    last_input : Input
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Client {
    timeline: timeline::Timeline,
    server_start: WasmInstant,
    last_tick: u32,
    local_player_info : Option<LocalPlayerInfo>,
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : u32, server_time_us : u32, estimated_latency : u32, player_count : u8) -> Self {
        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, vec![], player_count);

        // Estimate server start
        let server_start = WasmInstant::now() - Duration::from_micros((server_time_us + estimated_latency) as u64);

        log!("CONSTRUCTING : Estamated t0 {:?} server t1 {} estimated latency {}", server_start, server_time_us, estimated_latency);
        
        Client {
            timeline,
            last_tick : server_time_us,
            server_start,
            local_player_info : None,
        } 
    }

    pub fn join(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            last_input : Input::None,
        })
    }

    pub fn set_local_input_json(&mut self, input_json : &str) {
        let input = serde_json::from_str(input_json).map_err(|e| log!("{} {:?}", input_json, e)).unwrap_or(Input::None);
        self.set_local_input(input);
    }

    fn set_local_input(&mut self, input : Input) {
        self.local_player_info.as_mut().map(|x| {
            x.last_input = input;
        });
    }

    pub fn tick(&mut self) {
        let current_time = self.server_start.elapsed();
        self.last_tick = current_time.as_micros() as u32;

        // Tick logic
        let mut player_inputs = self.timeline.get_last_player_inputs();

        self.local_player_info.as_ref().map(|x|
        {
            player_inputs.set(x.player_id, x.last_input);
        });

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
                        Some(&server_tick.latest),
                        None);
                }
                else
                {
                    self.timeline.propagate_state(
                        &server_tick.latest,
                        server_tick.last_client_sent.get(lpi.player_id),
                        Some(lpi.player_id));
                }
            }
            _ => {
                self.timeline.propagate_state(
                    &server_tick.latest,
                    None,
                    None);
            }
        }
    }

    pub fn get_client_message(&self) -> Vec<u8>
    {
        let message = self.get_client_message_internal();
        flexbuffers::to_vec(message).unwrap()

    }

    fn get_client_message_internal(&self) -> interop::CrossyMessage
    {
        let input = self.local_player_info.as_ref().map(|x| x.last_input).unwrap_or(Input::None);
        interop::CrossyMessage::ClientTick(interop::ClientTick {
            time_us: self.last_tick,
            input: input,
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