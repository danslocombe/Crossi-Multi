use std::hash::Hash;
use std::fmt::Debug;

use crossy_multi_core::{GameState, PlayerId, Input, CoordPos};
use crossy_multi_core::map::{Map, RowType};
use crossy_multi_core::player::MoveState;
use crossy_multi_core::rng::FroggyRng;

pub trait AIAgent : std::fmt::Debug
{
    fn think(&mut self, game_state: &GameState, map: &Map) -> Input;
}

#[derive(Debug)]
pub struct GoUpAI
{
    player_id : PlayerId,
    rng : FroggyRng,
    rng_t : u64,
}

impl GoUpAI {
    pub fn new(player_id : PlayerId) -> Self {
        Self {
            player_id,
            rng: FroggyRng::new(1234 + 555*(player_id.0 as u64)),
            rng_t : 0,
        }
    }

    fn side_random<T : Hash + Debug>(&self, x : T) -> Input {
        if (self.rng.gen_unit(("do_nothing", self.rng_t, &x)) < 0.9) {
            Input::None
        }
        else {
            *self.rng.choose(("side_random", self.rng_t, &x), &[Input::Left, Input::Right])
        }
    }
}

fn is_safe(coordpos : &CoordPos, game_state : &GameState, map : &Map) -> bool {
    let current_row = map.get_row(game_state.get_round_id(), coordpos.y);
    match &current_row.row_type {
        RowType::River(_)  => {
            let lillies = map.get_lillipads(game_state.get_round_id(), game_state.time_us);
            for lilly in &lillies
            {
                let dist = (coordpos.x as f64 - lilly.0).abs();
                if (dist < 0.05) {
                    return true
                }
            }

            false
        }
        RowType::Road(_) => {
            let cars = map.get_cars(game_state.get_round_id(), game_state.time_us);
            for car in &cars
            {
                let dist = (coordpos.x as f64 - car.0).abs();
                if (dist < 2.5) {
                    return false
                }
            }

            true
        }
        _ => true,
    }
}

impl AIAgent for GoUpAI
{
    fn think(&mut self, game_state : &GameState, map : &Map) -> Input
    {
        self.rng_t += 1;
        let maybe_player_state = game_state.get_player(self.player_id);
        if let Some(player_state) = maybe_player_state {
            let player_pos = match &player_state.move_state {
                MoveState::Stationary => player_state.pos,
                MoveState::Moving(moving_state) => moving_state.target,
            };

            let precise_pos = map.realise_pos(game_state.time_us, &player_pos);
            let y_up = precise_pos.y - 1;
            let test_pos = CoordPos { x : precise_pos.x.round() as i32, y : y_up };
            if (is_safe(&test_pos, game_state, map))
            {
                log!("Testing {:?} was safe", &test_pos);
                if (self.rng.gen_unit(("safe_idle", self.rng_t)) < 0.9)
                {
                    Input::None
                }
                else {
                    Input::Up
                }
            }
            else {
                log!("Testing {:?} NOT SAFE", &test_pos);
                self.side_random("not_safe")
            }
        }
        else {
            // Player dead
            Input::None
        }
    }
}