use crossy_multi_core::client;
use crossy_multi_core::game;

use std::time::{Instant, Duration};

const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);

fn main() {
    let start = Instant::now();
    let mut client = client::Client::try_create(8080).expect("Could not create client");
    loop {
        println!("Tick");
        let tick_start = Instant::now();
        let input = game::Input::Up;
        client.tick(input);

        let now = Instant::now();
        let elapsed_time = now.saturating_duration_since(tick_start);

        match DESIRED_TICK_TIME.checked_sub(elapsed_time)
        {
            Some(sleep_time) => {
                //println!("Sleeping zzzz for {},", sleep_time.as_micros());
                std::thread::sleep(sleep_time);
            },
            None => {
                //println!("No time to waste!");
            }
        }
    }
}
