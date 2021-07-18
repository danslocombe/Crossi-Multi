use wasm_bindgen::prelude::*;
use web_sys::console;

use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};

use serde_json::json;

const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);

use crossy_multi_core::*;
use crossy_multi_core::game::PlayerId;

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
    server_start: Instant,
    last_tick: u32,
    local_player_info : Option<LocalPlayerInfo>,
}

/*
// Ugh dont want wasm-bindgen in the core package
#[wasm_bindgen]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerIdInterop(u32);

impl Into<PlayerId> for PlayerIdInterop
{
    fn into(self) -> PlayerId {
        PlayerId(self.0 as u8)
    }
}

#[wasm_bindgen]
pub struct PlayerStateInterop {
    player_state : PlayerState,
}

impl Into<PlayerState> for PlayerStateInterop
{
    fn into(self) -> PlayerState {
        PlayerState {
            
        }
    }
}
*/

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : u32, server_time_us : u32, estimated_latency : u32, player_count : u8) -> Self {
        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, vec![], player_count);

        // Estimate server start
        let server_start = Instant::now() - Duration::from_micros((server_time_us + estimated_latency) as u64);
        
        Client {
            timeline,
            last_tick : server_time_us,
            server_start,
            local_player_info : None,
        } 
    }

    pub fn joined(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            last_input : Input::None,
        })
    }

    pub fn set_local_input_json(&mut self, input_json : &str) {
        let input = serde_json::from_str(input_json).unwrap();
        self.set_local_input(input);
    }

    fn set_local_input(&mut self, input : Input) {
        self.local_player_info.as_mut().map(|x| {
            x.last_input = input;
        });
    }

    pub fn tick(&mut self) {
        let tick_start = Instant::now();
        let current_time = tick_start.saturating_duration_since(self.server_start);
        self.last_tick = current_time.as_micros() as u32;

        // Tick logic
        let mut player_inputs = self.timeline.get_last_player_inputs();

        self.local_player_info.as_ref().map(|x|
        {
            player_inputs.set(x.player_id, x.last_input);
        });

        self.timeline
            .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        let reader = flexbuffers::Reader::get_root(server_tick).unwrap();
        let tick = interop::ServerTick::deserialize(reader).unwrap();
        self.recv_internal(&tick);
    }

    fn recv_internal(&mut self, server_tick : &interop::ServerTick)
    {
        match self.local_player_info.as_ref()
        {
            Some(lpi) => {
                self.timeline.propagate_state(
                    &server_tick.latest,
                    server_tick.last_client_sent.get(lpi.player_id),
                    Some(lpi.player_id));
            }
            _ => {
                self.timeline.propagate_state(
                    &server_tick.latest,
                    None,
                    None);
            }
        }
        /*
        self.timeline.propagate_state(
            &server_tick.latest,
            server_tick.last_client_sent.get(self.local_player_id),
            self.local_player_id);
            */
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