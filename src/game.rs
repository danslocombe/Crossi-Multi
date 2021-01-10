use std::collections::VecDeque;
use num_derive::FromPrimitive;    

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pos
{
    Coord(CoordPos),
    Log(LogId),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub struct PlayerId(u8);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogId(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum Input
{
    None = 0,
    Up = 1,
    Left = 2,
    Right = 3,
    Down = 4,
}

pub struct SimulationInput
{
    pub inputs : Vec<Input>,
}

impl SimulationInput
{
    fn get_player_input(&self, id: PlayerId) -> Input
    {
        self.inputs[id.0 as usize]
    }
}

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
            player_count: 1,
            rand_seed : 0,
            states : states,
        }
    }

    pub fn tick(&mut self, input : SimulationInput, dt_us : f64) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us);
        self.states.push_front(new);
    }

    pub fn current_state(&self) -> &GameState
    {
        self.states.get(0).unwrap()
    }
}

#[derive(Clone)]
pub struct GameState
{
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter? 
    time_us : u32,

    player_states : Vec<PlayerState>,
    log_states : Vec<LogState>,
}

impl GameState
{
    fn new() -> Self
    {
        let p0 = PlayerState {
            id: PlayerId(0),
            move_state : MoveState::Stationary,
            move_cooldown: MOVE_COOLDOWN_MAX,
            pos: Pos::Coord(CoordPos{x : 10, y : 10}),
        };

        GameState
        {
            time_us : 0,
            player_states : vec![p0],
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

    fn simulate(&self, input : SimulationInput, dt_us : f64) -> Self
    {
        let mut new = self.clone();
        new.simulate_mut(input, dt_us);
        new
    }

    fn simulate_mut(&mut self, input : SimulationInput, dt_us : f64)
    {
        self.time_us += dt_us as u32;

        for _log in &mut self.log_states
        {
            // TODO
        }

        for player in &mut self.player_states
        {
            let player_input = input.get_player_input(player.id);
            player.tick(player_input, dt_us);
        }
    }
}

#[derive(Clone)]
pub struct PlayerState
{
    pub id : PlayerId,

    pub move_state : MoveState,
    pub move_cooldown : f64,

    pub pos : Pos,
}

#[derive(Clone)]
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

#[derive(Clone)]
struct LogState
{
    id : LogId,
    y : i32,

    x : f64,
    xvel : f64,
}
