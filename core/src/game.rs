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

    pub fn set(&mut self, id: PlayerId, input: Input) {
        let index = id.0 as usize;
        if (index >= self.inputs.len())
        {
            self.inputs.resize(index + 1, Input::None);
        }

        self.inputs[index] = input;
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
            move_cooldown: 0,
        };

        new.set_player_state(id, state);
        new
    }

    pub fn remove_player(&self, id: PlayerId) -> Self {
        let mut new = self.clone();
        let idx = id.0 as usize;
        if idx >= new.player_states.len() {
            // Nothing to do
        }
        else {
            new.player_states[idx] = None;
        }

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
                let mut pushes = Vec::new();
                let id = player_state.id;
                let player_input = self.player_inputs.get(id);
                let iterated = player_state.tick_iterate(self, player_input, dt_us, &mut pushes);
                drop(player_state);

                self.set_player_state(id, iterated);

                if let Some(push) = pushes.first() {
                    let player_state = self.get_player(push.id).unwrap();
                    let pushed = player_state.push(push);
                    drop(player_state);
                    self.set_player_state(push.id, pushed);
                }
            }
        }
    }

    fn space_occupied_with_player(&self, pos : Pos, ignore_id : PlayerId) -> bool {
        for player in self.player_states.iter()
            .flat_map(|x| x.as_ref())
            .filter(|x| x.id != ignore_id) {

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

    fn can_push(&self, id : PlayerId, dir : Input) -> bool {
        let player = self.get_player(id).unwrap();
        match player.pos {
            Pos::Coord(p) => {
                let new = p.apply_input(dir);
                !self.space_occupied_with_player(Pos::Coord(new), id)
            }
            _ => {
                false
            }
        }
    }
}

// TODO change these times to u32 from f64 micros
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PlayerState {
    pub id: PlayerId,

    pub move_state: MoveState,
    pub move_cooldown: u32,

    pub pos: Pos,
}

// TODO a really good idea here
// if a player being pushed recovers faster than the pusher then stunlock would be lessened
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct PushInfo {
    pub pushed_by : Option<PlayerId>,
    pub pushing : Option<PlayerId>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MovingState
{
    pub remaining_us : u32,
    pub target : Pos,
    pub push_info : PushInfo,
}

impl MovingState {
    pub fn new(target : Pos) -> MovingState {
        MovingState {
            remaining_us : MOVE_DUR,
            push_info : Default::default(),
            target,
        }
    }

    pub fn with_push(target : Pos, push_info : PushInfo) -> MovingState {
        MovingState {
            remaining_us : MOVE_DUR,
            target,
            push_info,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MoveState {
    Stationary,
    Moving(MovingState),
}

struct Push {
    pub id : PlayerId,
    pub pushed_by : PlayerId,
    pub dir : Input,
}

// In us
pub const MOVE_COOLDOWN_MAX: u32 = 150_000;
pub const MOVE_DUR: u32 = 50_000;

impl PlayerState {
    fn can_move(&self) -> bool {
        if let MoveState::Stationary = self.move_state {
            self.move_cooldown <= 0
        } else {
            false
        }
    }

    fn tick_iterate(&self, state: &GameState, input: Input, dt_us: u32, pushes : &mut Vec<Push>) -> Self {
        let mut new = self.clone();
        match new.move_state {
            MoveState::Stationary => {
                new.move_cooldown = new.move_cooldown.checked_sub(dt_us).unwrap_or(0);
            }
            MoveState::Moving(moving_state) => {
                match moving_state.remaining_us.checked_sub(dt_us)
                {
                    Some(remaining_us) =>
                    {
                        let mut new_state = moving_state.clone();
                        new_state.remaining_us = remaining_us;
                        new.move_state = MoveState::Moving(new_state);
                    },
                    _ => {
                        // Safe as we know dt_us >= remaining_us from previous
                        // subtraction. 
                        let leftover_us = dt_us - moving_state.remaining_us;

                        // In new pos
                        new.pos = moving_state.target;
                        new.move_state = MoveState::Stationary;

                        // rem_ms <= 0 so we add it to the max cooldown
                        new.move_cooldown = MOVE_COOLDOWN_MAX.checked_sub(leftover_us).unwrap_or(0);
                    },
                }
            }
        }

        if new.can_move() && input != Input::None {
            if let Some(moving_state) = new.try_move(input, state, pushes) {
                new.move_state = MoveState::Moving(moving_state);
            }
        }

        new
    }

    fn push(&self, push : &Push) -> Self {
        let mut new = self.clone();
        match new.pos {
            Pos::Coord(p) => {
                let new_pos = p.apply_input(push.dir);
                let mut push_info = PushInfo::default();
                push_info.pushed_by = Some(push.pushed_by);
                new.move_state = MoveState::Moving(MovingState::with_push(Pos::Coord(new_pos), push_info));
            }
            _ => {},
        }
        new
    }

    fn try_move(&self, input : Input, state : &GameState, pushes : &mut Vec<Push>) -> Option<MovingState> {
        let mut new_pos = None;
        let mut push_info = PushInfo::default();

        match self.pos {
            Pos::Coord(pos) => {
                let candidate_pos = Pos::Coord(pos.apply_input(input));
                new_pos = Some(candidate_pos);

                for other_player in state.player_states.iter()
                    .flat_map(|x| x.as_ref())
                    .filter(|x| x.id != self.id) {

                    // Because only one player per spot
                    // We can only push one person
                    push_info = self.try_move_player(input, candidate_pos, other_player, state, pushes)?;
                    break;
                }
            },
            _ => {},
        }

        new_pos.map(|target| {
            MovingState::with_push(target, push_info)
        })
    }

    fn try_move_player(&self, dir : Input, candidate_pos : Pos, other : &PlayerState, state: &GameState, pushes : &mut Vec<Push>) -> Option<PushInfo>
    {
        if other.pos == candidate_pos {
            // Try to move into some other player
            match &other.move_state {
                MoveState::Moving(_ms) => None,
                MoveState::Stationary => {
                    if (state.can_push(other.id, dir)) {
                        pushes.push(Push {
                            id : other.id,
                            pushed_by : self.id,
                            dir,
                        });

                        // Managed to push
                        let mut push_info = PushInfo::default();
                        push_info.pushing = Some(other.id);
                        Some(push_info)
                    }
                    else {
                        None
                    }
                }
            }
        }
        else {
            match &other.move_state {
                MoveState::Moving(state) => {
                    // Only allow moevement if we are going to a different position to another frog
                    if (state.target != candidate_pos)
                    {
                        Some(PushInfo::default())
                    }
                    else
                    {
                        None
                    }
                },
                _ => Some(PushInfo::default())
            }
        }
    }

    pub fn is_being_pushed(&self) -> bool {
        match (&self.move_state) {
            MoveState::Moving(s) if s.push_info.pushed_by.is_some() => {
                true
            },
            _ => false,
        }
    }

    pub fn is_being_pushed_by(&self, player : PlayerId) -> bool {
        match (&self.move_state) {
            MoveState::Moving(s) if s.push_info.pushed_by == Some(player) => {
                true
            },
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogState {
    id: LogId,
    y: i32,

    x: f64,
    xvel: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gamestate(states : Vec<PlayerState>) -> GameState {
        GameState {
            time_us : 0,
            frame_id : 0.,
            player_states: states.into_iter().map(|x| Some(x)).collect(),
            player_inputs: PlayerInputs::default(),
            log_states : vec![],
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
        let new = world.simulate(Some(inputs), 100_000);

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
        let new = world.simulate(Some(inputs), 100_000);

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
        let new = world.simulate(Some(inputs), 100_000);

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
        let new = world.simulate(Some(inputs), 10_000);

        let new_player = new.get_player(PlayerId(0)).unwrap();
        match new_player.move_state {
            MoveState::Moving(_) => {
                panic!("Not expected to be moving")
            }
            _ => {},
        }
    }
}