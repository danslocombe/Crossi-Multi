use std::collections::VecDeque;
use num_derive::FromPrimitive;    
use serde::{Serialize, Deserialize};

pub const MAX_PLAYERS : usize = 8;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pos
{
    Coord(CoordPos),
    Log(LogId),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CoordPos
{
    pub x : i32,
    pub y : i32,
}

impl CoordPos
{
    fn apply_input(&self, input : Input) -> Self
    {
        match input
        {
            Input::None => *self,
            Input::Up => CoordPos {x: self.x, y: self.y - 1},
            Input::Down => CoordPos {x: self.x, y: self.y + 1},
            Input::Left => CoordPos {x: self.x - 1, y: self.y},
            Input::Right => CoordPos {x: self.x + 1, y: self.y},
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub struct PlayerId(pub u8);

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogId(pub u32);

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum Input
{
    None = 0,
    Up = 1,
    Left = 2,
    Right = 3,
    Down = 4,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerInputs
{
    pub inputs : [Input;MAX_PLAYERS],
}

pub struct TimedInput
{
    pub time_us : u32,
    pub input : Input,
    pub player_id : PlayerId,
}

impl PlayerInputs
{
    pub fn new() -> Self
    {
        PlayerInputs {
            inputs : [Input::None;MAX_PLAYERS],
        }
    }

    pub fn set(&mut self, id: PlayerId, input : Input)
    {
        self.inputs[id.0 as usize] = input;
    }

    pub fn get(&self, id: PlayerId) -> Input
    {
        self.inputs[id.0 as usize]
    }
}

const STATE_BUFFER_SIZE : usize = 16;

pub struct Game
{
    player_count : u8,
    rand_seed : u32,
    states : VecDeque<GameState>,
}

impl Game {
    pub fn new() -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Game {
            player_count: 0,
            rand_seed : 0,
            states : states,
        }
    }

    pub fn tick(&mut self, input : Option<PlayerInputs>, dt_us : f64) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us);
        self.push_state(new);
    }

    pub fn add_player(&mut self, player_id : PlayerId) {
        println!("Adding new player {:?}", player_id);

        let state = self.states.get(0).unwrap();
        let new = state.add_player(player_id);
        self.push_state(new);
    }

    pub fn top_state(&self) -> &GameState
    {
        self.states.get(0).unwrap()
    }

    pub fn propagate_inputs(&mut self, mut inputs : Vec<TimedInput>)
    {
        inputs.sort_by(|x, y| {x.time_us.cmp(&y.time_us)});

        for input in &inputs
        {
            // TODO optimisation
            // group all updates within same frame
            self.propagate_input(input);

            // TMP
            return;
        }
    }

    fn propagate_input(&mut self, input : &TimedInput)
    {
        // Try and get the state we should start propagating from
        // If the input is too old we drop it
        if let Some(mut oldest_index) = self.get_index_for_us(input.time_us)
        {
            if (oldest_index == 0)
            {
                // Packet got here so quick! (Latency estimate must be off)
                // We look at the diff between the last two states
                oldest_index = 1;
            }

            for i in (1..oldest_index + 1).rev() {
                let state_inputs = self.states[i-1].player_inputs;
                if (state_inputs.get(input.player_id) == input.input)
                {
                    // Up to date, nothing to do
                    return
                }

                println!("Modifying! {}", i);

                let mut new_inputs = state_inputs.clone();
                new_inputs.set(input.player_id, input.input);

                let dt = self.states[i-1].time_us - self.states[i].time_us;

                let replacement_state = self.states[i].simulate(Some(new_inputs), dt as f64);
                self.states[i] = replacement_state;
            }
        }
    }

    pub fn current_state(&self) -> &GameState
    {
        self.states.get(0).unwrap()
    }

    // Find the first state at a time point before a given time.
    fn get_index_for_us(&self, time_us : u32) -> Option<usize> {
        // TODO binary search
        for i in 0..self.states.len() {
            let state = &self.states[i];
            if (state.time_us < time_us) {
                return Some(i);
            }
        }

        None
    }

    fn push_state(&mut self, state: GameState) {
        self.states.push_front(state);
        while self.states.len() > STATE_BUFFER_SIZE
        {
            self.states.pop_back();
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState
{
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter? 
    pub time_us : u32,

    pub player_states : Vec<PlayerState>,
    pub player_inputs : PlayerInputs,
    pub log_states : Vec<LogState>,
}

impl GameState
{
    fn new() -> Self
    {
        GameState
        {
            time_us : 0,
            player_states : vec![],
            player_inputs : PlayerInputs::new(),
            log_states : vec![],
        }
    }

    pub fn get_player(&self, id: PlayerId) -> &PlayerState
    {
        &self.player_states[id.0 as usize]
    }

    fn get_player_mut(&mut self, id: PlayerId) -> &mut PlayerState
    {
        &mut self.player_states[id.0 as usize]
    }
    
    pub fn add_player(&self, id: PlayerId) -> Self
    {
        let mut new = self.clone();

        let state = PlayerState {
            id: id,
            move_state : MoveState::Stationary,
            move_cooldown: MOVE_COOLDOWN_MAX,
            pos: Pos::Coord(CoordPos{x : 10, y : 10}),
        };

        new.player_states.push(state);

        new
    }

    fn simulate(&self, input : Option<PlayerInputs>, dt_us : f64) -> Self
    {
        let mut new = self.clone();
        new.simulate_mut(input, dt_us);
        new
    }

    fn simulate_mut(&mut self, player_inputs : Option<PlayerInputs>, dt_us : f64)
    {
        self.time_us += dt_us as u32;

        if let Some(inputs) = player_inputs {
            self.player_inputs = inputs;
        }

        for _log in &mut self.log_states
        {
            // TODO
        }

        for player in &mut self.player_states
        {
            let player_input = self.player_inputs.get(player.id);
            player.tick(player_input, dt_us);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PlayerState
{
    pub id : PlayerId,

    pub move_state : MoveState,
    pub move_cooldown : f64,

    pub pos : Pos,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MoveState
{
    Stationary,

    // 0-1 representing current interpolation
    Moving(f64, Pos),
}

// In ms
const MOVE_COOLDOWN_MAX : f64 = 350.0 * 1000.0;

impl PlayerState
{
    fn can_move(&self) -> bool
    {
        if let MoveState::Stationary = self.move_state {
            self.move_cooldown <= 0.0
        }
        else
        {
            false
        }
    }

    fn tick(&mut self, input : Input, dt_us : f64)
    {
        match self.move_state
        {
            MoveState::Stationary => {
                self.move_cooldown = (self.move_cooldown - dt_us).max(0.0);
            },
            MoveState::Moving(x, target_pos) => {
                let rem_ms = x - dt_us;
                if rem_ms > 0.0
                {
                    self.move_state = MoveState::Moving(rem_ms, target_pos);
                }
                else
                {
                    // In new pos
                    self.pos = target_pos;
                    self.move_state = MoveState::Stationary;

                    // rem_ms <= 0 so we add it to the max cooldown
                    self.move_cooldown = MOVE_COOLDOWN_MAX + rem_ms;
                }
            }
        }

        if self.can_move() && input != Input::None
        {
            // Start moving
            // todo

            // no logs
            match self.pos
            {
                Pos::Coord(pos) =>
                {
                    let new_pos = Pos::Coord(pos.apply_input(input));
                    self.move_state = MoveState::Moving(0.0, new_pos);
                },
                _ => {},
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogState
{
    id : LogId,
    y : i32,

    x : f64,
    xvel : f64,
}
