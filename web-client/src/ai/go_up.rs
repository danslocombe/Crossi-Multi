use std::hash::Hash;
use std::fmt::Debug;

use crossy_multi_core::{GameState, PlayerId, Input, CoordPos, PreciseCoords};
use crossy_multi_core::map::{Map, RowType};
use crossy_multi_core::player::MoveState;
use crossy_multi_core::rng::FroggyRng;

use crate::ai::*;

#[derive(Debug)]
pub struct GoUpAI
{
    player_id : PlayerId,
    rng : FroggyRng,
    rng_t : u64,
    careful_t : i32,
    draw_state : AIDrawState,
}

impl GoUpAI {
    pub fn new(player_id : PlayerId) -> Self {
        Self {
            player_id,
            rng: FroggyRng::new(1234 + 555*(player_id.0 as u64)),
            rng_t : 0,
            careful_t : 0,
            draw_state : AIDrawState::default(),
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

fn should_be_careful(coordpos : &CoordPos, game_state : &GameState, map : &Map) -> bool {
    let row_type = map.get_row(game_state.get_round_id(), coordpos.y).row_type;
    row_type.is_dangerous()
}

fn is_safe_inner(coordpos : &CoordPos, game_state : &GameState, map : &Map, draw_state : &mut AIDrawState) -> bool {
    let current_row = map.get_row(game_state.get_round_id(), coordpos.y);
    match &current_row.row_type {
        RowType::River(_)  => {
            if let Some(_lillipad) = map.lillipad_at_pos(game_state.get_round_id(), game_state.time_us, coordpos.to_precise()) {
                true
            }
            else {
                false
            }
        }
        RowType::Road(_) => {
            let cars = map.get_cars(game_state.get_round_id(), game_state.time_us);
            const MIN_CAR_DIST : f64 = 3.0;
            const CAR_MOVE_EST : f64 = 0.5;
            let mut result = true; 
            for car in &cars
            {
                if (car.1 != coordpos.y) {
                    continue;
                }
                let car_precise = PreciseCoords{ x: car.0, y : coordpos.y};
                let frog_x = coordpos.x as f64 + 0.5;

                let dist_from_movement = if (car.2) {
                    -CAR_MOVE_EST
                }
                else {
                    CAR_MOVE_EST
                };

                let dist = (frog_x - car.0 + dist_from_movement).abs();
                if (dist < MIN_CAR_DIST) {
                    draw_state.draw_objs.push(AIDrawObj {
                        precise_pos: car_precise,
                        draw_type : AIDrawType::Circle,
                        colour : AIDrawColour::Red,
                    });
                    result = false;
                }
                else {
                    draw_state.draw_objs.push(AIDrawObj {
                        precise_pos: car_precise,
                        draw_type : AIDrawType::Circle,
                        colour : AIDrawColour::Green,
                    });
                }
            }

            result
        }
        _ => true,
    }
}

fn is_safe(coordpos : &CoordPos, game_state : &GameState, map : &Map, draw_state : &mut AIDrawState) -> bool {
    let result = is_safe_inner(coordpos, game_state, map, draw_state);

    if (result) {
        draw_state.draw_objs.push(AIDrawObj {
            precise_pos: coordpos.to_precise(),
            draw_type : AIDrawType::Tick,
            colour : AIDrawColour::Green,
        });
    }
    else {
        draw_state.draw_objs.push(AIDrawObj {
            precise_pos: coordpos.to_precise(),
            draw_type : AIDrawType::Cross,
            colour : AIDrawColour::Red,
        });
    }

    result
}

impl AIAgent for GoUpAI
{
    fn think(&mut self, game_state : &GameState, map : &Map) -> Input
    {
        self.rng_t += 1;
        self.draw_state.draw_objs.clear();

        let maybe_player_state = game_state.get_player(self.player_id);
        if let Some(player_state) = maybe_player_state {
            let player_pos = match &player_state.move_state {
                MoveState::Stationary => player_state.pos,
                MoveState::Moving(moving_state) => moving_state.target,
            };

            let precise_pos = map.realise_pos(game_state.time_us, &player_pos);
            let player_pos_coords = precise_pos.to_coords();
            let y_up = precise_pos.y - 1;
            let test_pos = CoordPos { x : precise_pos.x.round() as i32, y : y_up };
            if (is_safe(&test_pos, game_state, map, &mut self.draw_state))
            {
                /*
                if (should_be_careful(&test_pos, game_state, map) && self.careful_t > 0) {
                    self.careful_t -= 1;
                    Input::None
                }
                else {
                    */
                {
                    //log!("Testing {:?} was safe", &test_pos);
                    if (is_safe(&player_pos_coords, game_state, map, &mut self.draw_state)) {
                        return if (self.rng.gen_unit(("safe_idle", self.rng_t)) < 0.1)
                        {
                            //self.careful_t -= 1;
                            Input::None
                        }
                        else {
                            //self.careful_t = 8;
                            Input::Up
                        }
                    }
                }
            }

            //log!("Testing {:?} NOT SAFE", &test_pos);
            // TODO Fix not shuffling last elem correctly
            let mut to_try = [Input::None, Input::Left, Input::Right, Input::Down];
            self.rng.shuffle(("shuffle_inputs", self.rng_t), &mut to_try);

            for input in &to_try {
                let try_pos = player_pos_coords.apply_input(*input);
                if (is_safe(&try_pos, game_state, map, &mut self.draw_state)) {
                    return *input;
                }
            }

            // Last resort pick random
            return *(to_try.first().unwrap());
        }
        else {
            // Player dead
            Input::None
        }
    }

    fn get_drawstate(&self) -> &AIDrawState {
        &self.draw_state
    }
}