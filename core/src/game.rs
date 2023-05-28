use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use crate::player_id_map::PlayerIdMap;
use crate::crossy_ruleset::{RulesState, GameConfig, AliveState};
use crate::map::Map;

use crate::player::*;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pos {
    Coord(CoordPos),
    Lillipad(LillipadId),
}

impl Pos {
    pub fn new_coord(x: i32, y: i32) -> Self {
        Pos::Coord(CoordPos { x, y })
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CoordPos {
    pub x: i32,
    pub y: i32,
}

impl CoordPos {
    pub fn to_precise(self) -> PreciseCoords {
        PreciseCoords {
            x: self.x as f64,
            y: self.y,
        }
    }

    pub fn apply_input(&self, input: Input) -> Self {
        match input {
            Input::Up => CoordPos {
                x: self.x,
                y: self.y - 1,
            },
            Input::Down => CoordPos {
                x: self.x,
                y: self.y + 1,
            },
            Input::Left => CoordPos {
                x: self.x - 1,
                y: self.y,
            },
            Input::Right => CoordPos {
                x: self.x + 1,
                y: self.y,
            },
            _  => *self,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct PreciseCoords {
    pub x : f64,
    pub y : i32,
}

impl PreciseCoords {
    pub fn to_coords(self) -> CoordPos {
        CoordPos {
            x: self.x.round() as i32,
            y: self.y,
        }
    }

    pub fn apply_input(&self, input: Input) -> Self {
        match input {
            Input::Up => Self {
                x: self.x,
                y: self.y - 1,
            },
            Input::Down => Self {
                x: self.x,
                y: self.y + 1,
            },
            Input::Left => Self {
                x: self.x - 1.0,
                y: self.y,
            },
            Input::Right => Self {
                x: self.x + 1.0,
                y: self.y,
            },
            _ => *self,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub struct PlayerId(pub u8);

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct LillipadId
{
    pub id : u8,
    pub y : i32,
    pub round_id : u8,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum Input {
    None = 0,
    Up = 1,
    Left = 2,
    Right = 3,
    Down = 4,
}

pub const ALL_INPUTS : [Input; 4] = [
    Input::Up,
    Input::Left,
    Input::Right,
    Input::Down,
];

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlayerInputs {
    pub inputs: Vec<Input>,
}

impl Default for PlayerInputs {
    fn default() -> Self {
        PlayerInputs {
            inputs: Vec::with_capacity(8),
        }
    }
}

impl PlayerInputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, id: PlayerId, input: Input) -> bool {
        let index = id.0 as usize;
        if (index >= self.inputs.len())
        {
            self.inputs.resize(index + 1, Input::None);
        }

        let changed = self.inputs[index] != input;
        self.inputs[index] = input;
        changed
    }

    pub fn get(&self, id: PlayerId) -> Input {
        let index = id.0 as usize;
        if index < self.inputs.len()
        {
            self.inputs[index]
        }
        else
        {
            Input::None
        }
    }

    pub fn player_count(&self) -> usize {
        self.inputs.len()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter?
    pub time_us: u32,
    pub frame_id : u32,

    pub player_states: PlayerIdMap<PlayerState>,
    pub rules_state : RulesState,
    pub player_inputs: PlayerInputs,
}

impl GameState {
    pub fn new(config : GameConfig) -> Self {
        GameState {
            time_us: 0,
            frame_id: 0,
            player_states: PlayerIdMap::new(),
            rules_state: RulesState::new(config),
            player_inputs: PlayerInputs::new(),
        }
    }

    pub fn from_server_parts(frame_id : u32, time_us: u32, player_states_def: Vec<PlayerState>, rules_state : RulesState) -> Self {
        let player_states = PlayerIdMap::from_definition(player_states_def.into_iter().map(|x| (x.id, x)).collect());
        GameState {
            time_us,
            player_states,
            player_inputs: PlayerInputs::new(),
            rules_state,
            frame_id,
        }
    }

    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerState> {
        self.player_states.get(id)
    }

    pub fn get_player_mut(&mut self, id: PlayerId) -> Option<&mut PlayerState> {
        self.player_states.get_mut(id)
    }

    pub fn set_player_state(&mut self, id: PlayerId, state: PlayerState) {
        self.player_states.set(id, state);
    }

    pub fn get_player_count(&self) -> usize {
        self.player_states.count_populated()
    }

    pub fn get_valid_player_states(&self) -> Vec<PlayerState> {
        self.player_states.get_populated()
    }

    pub fn get_rule_state(&self) -> &RulesState {
        &self.rules_state
    }

    pub fn get_round_id(&self) -> u8 {
        self.rules_state.fst.get_round_id()
    }

    #[must_use]
    pub fn add_player(&self, id: PlayerId, pos: Pos) -> Self {
        let mut new = self.clone();

        let state = PlayerState {
            id,
            pos,
            move_state: MoveState::Stationary,
            move_cooldown: 0,
        };

        new.set_player_state(id, state);
        new
    }

    #[must_use]
    pub fn remove_player(&self, id: PlayerId) -> Self {
        let mut new = self.clone();
        new.player_states.remove(id);
        new
    }

    #[must_use]
    pub fn simulate(&self, input: Option<PlayerInputs>, dt_us: u32, map : &crate::map::Map) -> Self {
        let mut new = self.clone();
        new.simulate_mut(input, dt_us, map);
        new
    }

    fn simulate_mut(&mut self, player_inputs: Option<PlayerInputs>, dt_us: u32, map : &crate::map::Map) {
        self.time_us += dt_us;
        self.frame_id += 1;

        self.player_inputs = player_inputs.unwrap_or_default();

        for id in self.player_states.valid_ids() {
            if self.rules_state.fst.get_player_alive(id) != AliveState::Alive {
                continue;
            }

            let mut pushes = Vec::new();

            let player_input = self.player_inputs.get(id);

            // We can safely unwrap as we are iterating over valid_ids()
            let player_state = self.player_states.get(id).unwrap();

            let iterated = player_state.tick_iterate(self, player_input, dt_us, &mut pushes, map);

            self.set_player_state(id, iterated);

            // @TODO do we want to iterate all pushes?
            if let Some(push) = pushes.first() {
                let player_state = self.get_player(push.id).unwrap();
                let pushed = player_state.push(push, self, map);
                self.set_player_state(push.id, pushed);
            }
        }

        self.rules_state = self.rules_state.tick(dt_us, self.time_us, &mut self.player_states, map);
    }

    pub fn space_occupied_with_player(&self, pos : Pos, ignore_id : Option<PlayerId>) -> bool {
        for (_, player) in self.player_states.iter().filter(|(id, _)| Some(*id) != ignore_id) {
            if player.pos == pos {
                return true;
            }
            else {
                match &player.move_state {
                    MoveState::Moving(moving_state) => {
                        if moving_state.target == pos {
                            return true;
                        }
                    },
                    _ => {},
                }
            }
        }

        false 
    }

    pub(crate) fn can_push(&self, id : PlayerId, dir : Input, time_us : u32, rule_state : &RulesState, map : &Map) -> bool {
        let player = self.get_player(id).unwrap();

        match map.try_apply_input(time_us, &rule_state, &player.pos, dir)
        {
            Some(new_pos) => {
                !self.space_occupied_with_player(new_pos, Some(id))
            }
            _ => false,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_gamestate(states : Vec<PlayerState>) -> GameState {
        let player_states = PlayerIdMap::from_definition(states.into_iter().map(|x| (x.id, x)).collect());
        GameState {
            time_us : 0,
            frame_id : 0,
            player_states,
            player_inputs: PlayerInputs::default(),
            rules_state : RulesState::new(Default::default()),
        }
    }


    #[test]
    fn move_success() {
        let players = vec![
            PlayerState {
                id : PlayerId(0),
                move_state : MoveState::Stationary,
                move_cooldown : 0,
                pos : Pos::new_coord(0, 0),
            }
        ];

        let mut inputs = PlayerInputs::default();
        inputs.set(PlayerId(0), Input::Down);

        let world = make_gamestate(players);
        let map = Map::new(0);
        let new = world.simulate(Some(inputs), 100_000, &map);

        let new_player = new.get_player(PlayerId(0)).unwrap();
        match &new_player.move_state {
            MoveState::Moving(state) => {
                assert_eq!(state.target, Pos::new_coord(0, 1));
                assert_eq!(state.remaining_us, MOVE_DUR);
            }
            _ => panic!("Not moving"),
        }
    }

    #[test]
    fn move_not_blocked()
    {
        let players = vec![
            PlayerState {
                id : PlayerId(0),
                move_state : MoveState::Stationary,
                move_cooldown : 0,
                pos : Pos::new_coord(0, 0),
            },
            PlayerState {
                id : PlayerId(1),
                move_state : MoveState::Stationary,
                move_cooldown : 0,
                pos : Pos::new_coord(1, 0),
            },
        ];

        let mut inputs = PlayerInputs::default();
        inputs.set(PlayerId(0), Input::Down);

        let world = make_gamestate(players);
        let map = Map::new(0);
        let new = world.simulate(Some(inputs), 100_000, &map);

        let new_player = new.get_player(PlayerId(0)).unwrap();
        match &new_player.move_state {
            MoveState::Moving(state) => {
                assert_eq!(state.target, Pos::new_coord(0, 1));
                assert_eq!(state.remaining_us, MOVE_DUR);
            }
            _ => panic!("Not moving"),
        }
    }

    #[test]
    fn move_blocked_other_moving()
    {
        let players = vec![
            PlayerState {
                id : PlayerId(0),
                move_state : MoveState::Stationary,
                move_cooldown : 0,
                pos : Pos::new_coord(0, 0),
            },
            PlayerState {
                id : PlayerId(1),
                move_state : MoveState::Moving(MovingState::new(Pos::new_coord(1, 1))),
                move_cooldown : 0,
                pos : Pos::new_coord(0, 1),
            },
        ];

        let mut inputs = PlayerInputs::default();
        inputs.set(PlayerId(0), Input::Down);

        let world = make_gamestate(players);
        let map = Map::new(0);
        let new = world.simulate(Some(inputs), 100_000, &map);

        let new_player = new.get_player(PlayerId(0)).unwrap();
        match new_player.move_state {
            MoveState::Moving(_) => {
                panic!("Not expected to be moving")
            }
            _ => {},
        }
    }

    #[test]
    fn move_blocked_other_moving_to_pos()
    {
        let players = vec![
            PlayerState {
                id : PlayerId(0),
                move_state : MoveState::Stationary,
                move_cooldown : 0,
                pos : Pos::new_coord(0, 0),
            },
            PlayerState {
                id : PlayerId(1),
                move_state : MoveState::Moving(MovingState::new(Pos::new_coord(0, 1))),
                move_cooldown : 0,
                pos : Pos::new_coord(1, 1),
            },
        ];

        let mut inputs = PlayerInputs::default();
        inputs.set(PlayerId(0), Input::Down);

        let world = make_gamestate(players);
        let map = Map::new(0);
        let new = world.simulate(Some(inputs), 10_000, &map);

        let new_player = new.get_player(PlayerId(0)).unwrap();
        match new_player.move_state {
            MoveState::Moving(_) => {
                panic!("Not expected to be moving")
            }
            _ => {},
        }
    }
}