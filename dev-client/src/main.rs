use crossy_multi_core::client;
use crossy_multi_core::game;

use std::time::{Instant, Duration};

const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);

fn main() {
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
                println!("Pos {:?}", &cur_pos);
            }
        }

        let now = Instant::now();
        let elapsed_time = now.saturating_duration_since(tick_start);

        match DESIRED_TICK_TIME.checked_sub(elapsed_time)
        {
            Some(sleep_time) => {
                std::thread::sleep(sleep_time);
            },
            None => {},
        }

        tick += 1
    }
}
