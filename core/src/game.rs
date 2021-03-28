use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};

pub const MAX_PLAYERS: usize = 8;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pos {
    Coord(CoordPos),
    Log(LogId),
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
    pub fn apply_input(&self, input: Input) -> Self {
        match input {
            Input::None => *self,
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
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub struct PlayerId(pub u8);

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogId(pub u32);

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum Input {
    None = 0,
    Up = 1,
    Left = 2,
    Right = 3,
    Down = 4,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerInputs {
    pub inputs: [Input; MAX_PLAYERS],
}

impl Default for PlayerInputs {
    fn default() -> Self {
        PlayerInputs {
            inputs: [Input::None; MAX_PLAYERS],
        }
    }
}

impl PlayerInputs {
    pub fn new() -> Self {
        PlayerInputs {
            inputs: [Input::None; MAX_PLAYERS],
        }
    }

    pub fn set(&mut self, id: PlayerId, input: Input) {
        self.inputs[id.0 as usize] = input;
    }

    pub fn get(&self, id: PlayerId) -> Input {
        self.inputs[id.0 as usize]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter?
    pub time_us: u32,

    player_states: Vec<Option<PlayerState>>,
    pub player_inputs: PlayerInputs,
    pub log_states: Vec<LogState>,
    pub frame_id: f64,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            time_us: 0,
            player_states: vec![],
            player_inputs: PlayerInputs::new(),
            log_states: vec![],
            frame_id: 0.0,
        }
    }

    pub fn from_server_parts(_seed: u32, time_us: u32, player_states: Vec<PlayerState>) -> Self {
        GameState {
            time_us: time_us,
            player_states: player_states.into_iter().map(|x| Some(x)).collect(),
            player_inputs: PlayerInputs::new(),
            log_states: vec![],
            //TODO
            frame_id: 0.0,
        }
    }

    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerState> {
        let index = id.0 as usize;
        if (index < self.player_states.len()) {
            self.player_states[index].as_ref()
        } else {
            None
        }
    }

    pub fn get_player_mut(&mut self, id: PlayerId) -> Option<&mut PlayerState> {
        let idx = id.0 as usize;
        if idx >= self.player_states.len() {
            None
        } else {
            self.player_states[idx].as_mut()
        }
    }

    pub fn set_player_state(&mut self, id: PlayerId, state: PlayerState) {
        let idx = id.0 as usize;
        if idx >= self.player_states.len() {
            self.player_states.resize(idx + 1, None);
        }

        self.player_states[idx] = Some(state);
    }

    pub fn get_player_count(&self) -> usize {
        self.player_states.iter().flatten().count()
    }

    pub fn get_valid_player_states(&self) -> Vec<PlayerState> {
        self.player_states.iter().flatten().cloned().collect()
    }

    pub fn add_player(&self, id: PlayerId, pos: Pos) -> Self {
        let mut new = self.clone();

        let state = PlayerState {
            id,
            pos,
            move_state: MoveState::Stationary,
            move_cooldown: 0.0,
        };

        new.set_player_state(id, state);
        new
    }

    pub fn simulate(&self, input: Option<PlayerInputs>, dt_us: u32) -> Self {
        let mut new = self.clone();
        new.simulate_mut(input, dt_us);
        new
    }

    fn simulate_mut(&mut self, player_inputs: Option<PlayerInputs>, dt_us: u32) {
        self.time_us += dt_us;
        self.frame_id += 1.0;

        self.player_inputs = player_inputs.unwrap_or(PlayerInputs::new());

        for _log in &mut self.log_states {
            // TODO
        }

        for i in 0..self.player_states.len() {
            if let Some(player_state) = self.player_states[i].as_ref() {
                let id = player_state.id;
                let player_input = self.player_inputs.get(id);
                let iterated = player_state.tick_iterate(self, player_input, dt_us as f64);
                drop(player_state);

                self.set_player_state(id, iterated);
            }
        }
    }
}

// TODO change these times to u32 from f64 micros
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PlayerState {
    pub id: PlayerId,

    pub move_state: MoveState,
    pub move_cooldown: f64,

    pub pos: Pos,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MoveState {
    Stationary,

    // 0-1 representing current interpolation
    Moving(f64, Pos),
}

// In us
pub const MOVE_COOLDOWN_MAX: f64 = 150_000.0;
pub const MOVE_DUR: f64 = 10_000.0;

impl PlayerState {
    fn can_move(&self) -> bool {
        if let MoveState::Stationary = self.move_state {
            self.move_cooldown <= 0.0
        } else {
            false
        }
    }

    fn tick_iterate(&self, state: &GameState, input: Input, dt_us: f64) -> Self {
        let mut new = self.clone();
        match new.move_state {
            MoveState::Stationary => {
                new.move_cooldown = (new.move_cooldown - dt_us).max(0.0);
            }
            MoveState::Moving(x, target_pos) => {
                let rem_ms = x - dt_us;
                if rem_ms > 0.0 {
                    new.move_state = MoveState::Moving(rem_ms, target_pos);
                } else {
                    // In new pos
                    new.pos = target_pos;
                    new.move_state = MoveState::Stationary;

                    // rem_ms <= 0 so we add it to the max cooldown
                    new.move_cooldown = MOVE_COOLDOWN_MAX + rem_ms;
                }
            }
        }

        if new.can_move() && input != Input::None {
            let mut new_pos = None;

            match new.pos {
                Pos::Coord(pos) => {
                    let candidate_pos = Pos::Coord(pos.apply_input(input));
                    new_pos = Some(candidate_pos);

                    for m_player in &state.player_states {
                        if let Some(player) = m_player {
                            if player.id != new.id && player.pos == candidate_pos {
                                // Can push?
                                new_pos = None;
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }

            if let Some(pos) = new_pos {
                new.move_state = MoveState::Moving(MOVE_DUR, pos);
            }
        }

        new
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogState {
    id: LogId,
    y: i32,

    x: f64,
    xvel: f64,
}
