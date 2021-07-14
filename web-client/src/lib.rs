use wasm_bindgen::prelude::*;
use web_sys::console;

use std::time::{Instant, Duration};

const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);

use crossy_multi_core::*;


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub struct LocalPlayerInfo {
    player_id: game::PlayerId,
}

pub struct Client {
    timeline: timeline::Timeline,
    server_start: Instant,
    last_tick: u32,
    local_player_info : Option<LocalPlayerInfo>,
}

impl Client {
    pub fn new(seed : u32, server_time_us : u32, estimated_latency : u32, player_states : Vec<PlayerState>, player_count : u8) -> Self {
        let timeline = timeline::Timeline::from_server_parts(seed, server_time_us, player_states, player_count);

        // Estimate server start
        let server_start = Instant::now() - Duration::from_micros((server_time_us + estimated_latency) as u64);
        
        Client {
            timeline,
            last_tick : server_time_us,
            server_start,
            local_player_info : None,
        } 
    }

    pub fn tick(&mut self) {
        let tick_start = Instant::now();
        let current_time = tick_start.saturating_duration_since(self.server_start);
        self.last_tick = current_time.as_micros() as u32;

        // Tick logic
        let mut player_inputs = self.timeline.get_last_player_inputs();

        /*
        self.local_player_info.as_ref().map(|x|
        {
            player_inputs.set(x.player_id, input);
        });
        */

        self.timeline
            .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);
    }

    pub fn recv(&mut self, server_tick : &interop::ServerTick)
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

    pub fn get_players(&self) -> Vec<PlayerState>
    {
        self.timeline.top_state().get_valid_player_states()
    }
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
    /*
    let mut client = client::Client::try_create(8089).expect("Could not create client");
    let mut tick = 0;
    let mut cur_pos = game::Pos::Coord(game::CoordPos{x: 0, y:0});
    let mut up = true;
    loop {
        let tick_start = Instant::now();

        let input = if tick % 50 == 25 { 
            up = !up;
            if up {
                game::Input::Up
            }
            else {
                game::Input::Down
            }
        }
        else {
            game::Input::None
        };

        client.tick(input);

        {
            let top_state = client.timeline.top_state();
            let pos = top_state.get_player(client.local_player_id).unwrap().pos;
            if cur_pos != pos
            {
                cur_pos = pos;
                console::log_1(&JsValue::from_str(&format!("T = {}", top_state.time_us)));
                console::log_1(&JsValue::from_str(&format!("Pos = {:?}", &cur_pos)));
            }
        }

        let now = Instant::now();
        let elapsed_time = now.saturating_duration_since(tick_start);

        if let Some(sleep_time) = DESIRED_TICK_TIME.checked_sub(elapsed_time)
        {
            std::thread::sleep(sleep_time);
        }

        tick += 1
    }
    */
}