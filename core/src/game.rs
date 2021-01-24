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

#[derive(Debug)]
pub struct TimedInput
{
    pub time_us : u32,
    pub input : Input,
    pub player_id : PlayerId,
}

pub struct TimedState
{
    pub time_us : u32,
    pub player_states : Vec<PlayerState>,
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

const STATE_BUFFER_SIZE : usize = 32;

pub struct Game
{
    pub player_count : u8,
    pub seed : u32,
    states : VecDeque<GameState>,
}

impl Game {
    pub fn new() -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Game {
            player_count: 0,
            seed : 0,
            states : states,
        }
    }

    pub fn from_server_parts(seed : u32, time_us : u32, player_states : Vec<PlayerState>, player_count : u8) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::from_server_parts(seed, time_us, player_states));
        Game {
            player_count: player_count,
            seed : seed,
            states : states,
        }
    }

    pub fn tick(&mut self, input : Option<PlayerInputs>, dt_us : u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us);
        self.push_state(new);
    }

    pub fn tick_current_time(&mut self, input : Option<PlayerInputs>, time_us : u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, time_us - state.time_us);
        self.push_state(new);
    }

    pub fn get_last_player_inputs(&self) -> PlayerInputs
    {
        self.top_state().player_inputs.clone()
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
        if (inputs.len() == 0) {
            return;
        }

        println!("Propagating {} inputs", inputs.len());
        inputs.sort_by(|x, y| {x.time_us.cmp(&y.time_us)});

        for input in &inputs
        {
            //println!("{}", input.time_us);
            // TODO optimisation
            // group all updates within same frame
            self.propagate_input(input);
        }
    }

    //
    // TODO subdivision problem?
    // Not a problem as long as the cooldown time > frame length
    fn propagate_input(&mut self, input : &TimedInput)
    {
        // Try and get the state we should start propagating from
        // If the input is too old we drop it
        if let Some(mut oldest_index) = self.get_index_for_us(input.time_us)
        {
            println!("Propagating {:?} oldest_index = {}", input, oldest_index);
            if (oldest_index == 0)
            {
                // Packet got here so quick! (Latency estimate must be off)
                // We look at the diff between the last two states
                oldest_index = 1;
            }

            let mut first = true;

            for i in (1..oldest_index + 1).rev() {
                let state_inputs = self.states[i-1].player_inputs;
                if (state_inputs.get(input.player_id) == input.input)
                {
                    // Up to date, nothing to do
                    //println!("Nothing to do");
                    return
                }

                let mut new_inputs = state_inputs.clone();
                if first {
                    // Bifurcate

                    let dt = input.time_us - self.states[i].time_us;
                    let mut partial_state = self.states[i].simulate(None, dt as u32);
                    partial_state.frame_id -= 1.0;
                    self.states.pop_back();
                    self.states.push_back(partial_state);

                    println!("replacing frame {} with new input", self.states[i-1].frame_id);
                    new_inputs.set(input.player_id, input.input);
                    first = false;
                }
                else {
                    println!("propagating {} with new input", self.states[i-1].frame_id);
                    new_inputs.set(input.player_id, Input::None);
                }

                let dt = self.states[i-1].time_us - self.states[i].time_us;
                let replacement_state = self.states[i].simulate(Some(new_inputs), dt as u32);
                self.states[i-1] = replacement_state;
                /*
                let mut new_inputs = state_inputs.clone();
                if first {
                    println!("replacing frame {} with new input", self.states[i-1].frame_id);
                    {
                        // HACK HACK
                        //self.states[i].player_states[input.player_id.0 as usize].move_cooldown = 0.0;
                    }

                    new_inputs.set(input.player_id, input.input);
                    first = false;
                }
                else {
                    println!("propagating {} with new input", self.states[i-1].frame_id);
                    new_inputs.set(input.player_id, Input::None);
                }

                let dt = self.states[i-1].time_us - self.states[i].time_us;

                let replacement_state = self.states[i].simulate(Some(new_inputs), dt as u32);
                self.states[i-1] = replacement_state;
                */
            }
        }
    }

    fn get_sandwich(&self, time_us : u32) -> (Option<usize>, Option<usize>) {
        let mut before = None;
        let mut after = None;
        for i in (0..self.states.len()).rev() {

            let t = self.states[i].time_us;
            if t > time_us {
                break;
            } 

            before = Some(i);
            after = if i == 0 {
                None
            }
            else {
                Some(i-1)
            };
        }

        println!("{:?}, {:?}", before, after);
        (before, after)
    }

    pub fn propagate_state(&mut self, server_timed_state : TimedState, local_player : PlayerId)
    {
        // Insert the new state into the ring buffer
        // Pop everything after it off
        // Simulate up to now

        let (before, after) = self.get_sandwich(server_timed_state.time_us);

        if let Some(x) = before {
            while self.states.len() > x+1 {
                self.states.pop_back();
            }
        }

        let state_before_server = if before.is_some() {self.states.pop_back()} else {None};
        let state_after_server = after.map(|x| &self.states[x]);

        let mut server_state = GameState::from_server_parts(
            self.seed,
            server_timed_state.time_us,
            server_timed_state.player_states);

        if let (Some(prev_state), Some(next_state)) = (state_before_server, state_after_server) {
            //println!("XXX prev {} next {}", prev_state.time_us, next_state.time_us);
            //println!("Interpolating between two");
            let inputs = next_state.player_inputs;
            let dt = server_state.time_us - prev_state.time_us;
            let game_state_with_local_pos = prev_state.simulate(Some(inputs), dt);
            let override_player_state = game_state_with_local_pos.get_player(local_player).unwrap().clone();

            let server_pos = server_state.get_player(local_player).unwrap().pos;
            let local_pos = override_player_state.pos;
            if (server_pos != local_pos)
            {
                println!("Overriding server pos {:?} with local {:?}", server_pos, local_pos);
            }

            server_state.set_player_state(local_player, override_player_state);
        }

        println!("Pushing back t={}", server_state.time_us);
        self.states.push_back(server_state);

        println!("States len {}", self.states.len());
        // TODO make sure it has local pplayers inputs ok?

        // Simulate up to now
        for i in (0..self.states.len()-1).rev() {
            //let local_player_input = self.states[i].player_inputs.get(local_player);
            let dt = self.states[i].time_us - self.states[i+1].time_us;
            println!("Simulating up to date {} dt {}, t {}", i, dt, self.states[i].time_us);
            let inputs = Some(self.states[i].player_inputs);
            //inputs
            let new_state = self.states[i+1].simulate(inputs, dt);
            self.states[i] = new_state;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState
{
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter? 
    pub time_us : u32,

    pub player_states : Vec<PlayerState>,
    pub player_inputs : PlayerInputs,
    pub log_states : Vec<LogState>,
    pub frame_id : f64,
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
            frame_id : 0.0,
        }
    }

    pub fn from_server_parts(_seed : u32, time_us : u32, player_states : Vec<PlayerState>) -> Self {
        GameState
        {
            time_us : time_us,
            player_states : player_states,
            player_inputs : PlayerInputs::new(),
            log_states : vec![],
            //TODO
            frame_id : 0.0,
        }
    }

    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerState>
    {
        let index = id.0 as usize;
        if (index < self.player_states.len())
        {
            Some(&self.player_states[index])
        }
        else
        {
            None
        }
    }

    fn get_player_mut(&mut self, id: PlayerId) -> &mut PlayerState
    {
        &mut self.player_states[id.0 as usize]
    }

    fn set_player_state(&mut self, id: PlayerId, state: PlayerState)
    {
        self.player_states[id.0 as usize] = state;
    }

    pub fn get_player_count(&self) -> usize {
        self.player_states.len()
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

    fn simulate(&self, input : Option<PlayerInputs>, dt_us : u32) -> Self
    {
        let mut new = self.clone();
        new.simulate_mut(input, dt_us);
        new
    }

    fn simulate_mut(&mut self, player_inputs : Option<PlayerInputs>, dt_us : u32)
    {
        self.time_us += dt_us;
        self.frame_id+=1.0;

        /*
        if let Some(inputs) = player_inputs {
            self.player_inputs = inputs;
        }
        */

        self.player_inputs = player_inputs.unwrap_or(PlayerInputs::new());

        for _log in &mut self.log_states
        {
            // TODO
        }

        //for player in &mut self.player_states
        for i in 0..self.player_states.len()
        {
            let id = self.player_states[i].id;
            let player_input = self.player_inputs.get(id);
            let iterated = self.player_states[i].tick_iterate(self, player_input, dt_us as f64);
            self.player_states[i] = iterated;
        }
    }
}

// TODO change these times to u32 from f64 micros

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
const MOVE_COOLDOWN_MAX : f64 = 150.0 * 1000.0;

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

    fn tick_iterate(&self, state: &GameState, input : Input, dt_us : f64) -> Self
    {
        let mut new = self.clone();
        match new.move_state
        {
            MoveState::Stationary => {
                new.move_cooldown = (new.move_cooldown - dt_us).max(0.0);
            },
            MoveState::Moving(x, target_pos) => {
                let rem_ms = x - dt_us;
                if rem_ms > 0.0
                {
                    new.move_state = MoveState::Moving(rem_ms, target_pos);
                }
                else
                {
                    // In new pos
                    new.pos = target_pos;
                    new.move_state = MoveState::Stationary;

                    // rem_ms <= 0 so we add it to the max cooldown
                    new.move_cooldown = MOVE_COOLDOWN_MAX + rem_ms;
                }
            }
        }

        if new.can_move() && input != Input::None
        {
            let mut new_pos = None;

            match new.pos
            {
                Pos::Coord(pos) =>
                {
                    let candidate_pos = Pos::Coord(pos.apply_input(input));
                    new_pos = Some(candidate_pos);

                    for player in &state.player_states {
                        if player.id != new.id {
                            if player.pos == candidate_pos {
                                new_pos = None;
                                break;
                            }
                        }
                    }
                },
                _ => {},
            }

            if let Some(pos) = new_pos {
                new.move_state = MoveState::Moving(0.0, pos);
            }
        }

        new
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogState
{
    id : LogId,
    y : i32,

    x : f64,
    xvel : f64,
}
