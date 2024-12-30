use std::hash::Hash;
use std::fmt::Debug;

use crossy_multi_core::map::obstacle_row::ObstaclePublic;
use crossy_multi_core::{GameState, PlayerId, Input, CoordPos, PreciseCoords, player, Pos, crossy_ruleset};
use crossy_multi_core::map::{Map, RowType};
use crossy_multi_core::player::MoveState;

use froggy_rand::FroggyRand;

use crate::ai::*;
use crate::draw_commands::{DrawCommand, DrawCoords, DrawType, DrawColour};

#[derive(Debug)]
pub struct GoUpAI
{
    player_id : PlayerId,
    rng : FroggyRand,
    rng_t : u64,
    careful_t : i32,
    draw_state : DrawCommands,
}

impl GoUpAI {
    pub fn new(player_id : PlayerId) -> Self {
        Self {
            player_id,
            rng: FroggyRand::new(1234 + 555*(player_id.0 as u64)),
            rng_t : 0,
            careful_t : 0,
            draw_state : DrawCommands::default(),
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

    fn think_lobby(&mut self, game_state : &GameState, map : &Map) -> Input
    {
        let maybe_player_state = game_state.get_player(self.player_id);
        if let Some(player_state) = maybe_player_state {
            if (crossy_ruleset::player_in_lobby_ready_zone(player_state)) {
                return Input::None;
            }
            else {
                if let Pos::Coord(CoordPos{x, y}) = player_state.pos {
                    if (x < crossy_ruleset::LOBBY_READ_ZONE_X_MIN) {
                        return Input::Right;
                    }
                    if (x > crossy_ruleset::LOBBY_READ_ZONE_X_MAX) {
                        return Input::Left;
                    }
                    if (y < crossy_ruleset::LOBBY_READ_ZONE_Y_MIN) {
                        return Input::Down;
                    }
                    if (y > crossy_ruleset::LOBBY_READ_ZONE_Y_MAX) {
                        return Input::Up;
                    }
                }
            }
        }

        Input::None
    }

    fn think_game(&mut self, game_state : &GameState, map : &Map) -> Input
    {
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
                let self_pos_safe = is_safe(&player_pos_coords, game_state, map, &mut self.draw_state);

                if (self_pos_safe && should_be_careful(&test_pos, game_state, map) && self.careful_t > 0) {
                    self.careful_t -= 1;
                    return Input::None;
                }
                else {
                    //log!("Testing {:?} was safe", &test_pos);
                    if (self_pos_safe) {
                        return if (self.rng.gen_unit(("safe_idle", self.rng_t)) < 0.9)
                        {
                            //self.careful_t -= 1;
                            Input::None
                        }
                        else {
                            self.careful_t = 8;
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
                    log!("AGENT {:?} picking shuffled {:?} as safe", self.player_id, input);
                    return *input;
                }
            }

            // Last resort pick random
            log!("AGENT {:?} resorting to random", self.player_id);
            return *(to_try.first().unwrap());
        }

        // Player dead
        return Input::None;
    }
}

fn should_be_careful(coordpos : &CoordPos, game_state : &GameState, map : &Map) -> bool {
    let row_type = map.get_row(game_state.get_round_id(), coordpos.y).row_type;
    row_type.is_dangerous()
}

fn is_safe_inner(coordpos : &CoordPos, game_state : &GameState, map : &Map, draw_state : &mut DrawCommands) -> bool {
    let current_row = map.get_row(game_state.get_round_id(), coordpos.y);
    match &current_row.row_type {
        RowType::River(_)  => {
            if let Some(_lillipad) = map.lillipad_at_pos(game_state.get_round_id(), game_state.time_us, coordpos.to_precise(), &game_state.rules_state) {
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
            for ObstaclePublic(car_x, car_y, car_flipped) in cars.iter().cloned()
            {
                if (car_y != coordpos.y) {
                    continue;
                }

                let car_precise = PreciseCoords{ x: car_x, y : coordpos.y};
                let frog_x = coordpos.x as f64 + 0.5;

                let dist_from_movement = if (car_flipped) {
                    -CAR_MOVE_EST
                }
                else {
                    CAR_MOVE_EST
                };

                let dist = (frog_x - car_x + dist_from_movement).abs();
                if (dist < MIN_CAR_DIST) {
                    draw_state.commands.push(DrawCommand {
                        pos: DrawCoords::from_precise(car_precise),
                        draw_type : DrawType::Circle,
                        colour : DrawColour::Red,
                    });
                    result = false;
                }
                else {
                    draw_state.commands.push(DrawCommand {
                        pos: DrawCoords::from_precise(car_precise),
                        draw_type : DrawType::Circle,
                        colour : DrawColour::Green,
                    });
                }
            }

            result
        }
        _ => true,
    }
}

fn is_safe(coordpos : &CoordPos, game_state : &GameState, map : &Map, draw_state : &mut DrawCommands) -> bool {
    let result = is_safe_inner(coordpos, game_state, map, draw_state);

    if (result) {
        draw_state.commands.push(DrawCommand {
            pos: DrawCoords::from_precise(coordpos.to_precise()),
            draw_type : DrawType::Tick,
            colour : DrawColour::Green,
        });
    }
    else {
        draw_state.commands.push(DrawCommand {
            pos: DrawCoords::from_precise(coordpos.to_precise()),
            draw_type : DrawType::Cross,
            colour : DrawColour::Red,
        });
    }

    result
}

impl AIAgent for GoUpAI
{
    fn think(&mut self, game_state : &GameState, map : &Map) -> Input
    {
        self.rng_t += 1;
        self.draw_state.commands.clear();

        match game_state.get_rule_state().fst
        {
            crossy_ruleset::CrossyRulesetFST::Lobby{..} => {
                self.think_lobby(game_state, map)
            },
            _ => {
                self.think_game(game_state, map)
            }
        }
    }

    fn get_drawstate(&self) -> &DrawCommands {
        &self.draw_state
    }
}