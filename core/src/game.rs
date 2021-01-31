use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_PLAYERS: usize = 8;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pos {
    Coord(CoordPos),
    Log(LogId),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CoordPos {
    pub x: i32,
    pub y: i32,
}

impl CoordPos {
    fn apply_input(&self, input: Input) -> Self {
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

impl Default for PlayerInputs
{
    fn default() -> Self {
        PlayerInputs {
            inputs : [Input::None; MAX_PLAYERS],
        }
    }
}

#[derive(Debug)]
pub struct TimedInput {
    pub time_us: u32,
    pub input: Input,
    pub player_id: PlayerId,
}

pub struct TimedState {
    pub time_us: u32,
    pub player_states: Vec<PlayerState>,
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

const STATE_BUFFER_SIZE: usize = 32;

pub struct Game {
    pub player_count: u8,
    pub seed: u32,
    states: VecDeque<GameState>,
}

impl Game {
    pub fn new() -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Game {
            player_count: 0,
            seed: 0,
            states,
        }
    }

    pub fn from_server_parts(
        seed: u32,
        time_us: u32,
        player_states: Vec<PlayerState>,
        player_count: u8,
    ) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::from_server_parts(seed, time_us, player_states));
        Game {
            player_count,
            seed,
            states,
        }
    }

    pub fn tick(&mut self, input: Option<PlayerInputs>, dt_us: u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us);
        self.push_state(new);
    }

    pub fn tick_current_time(&mut self, input: Option<PlayerInputs>, time_us: u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, time_us - state.time_us);
        self.push_state(new);
    }

    pub fn get_last_player_inputs(&self) -> PlayerInputs {
        self.top_state().player_inputs.clone()
    }

    pub fn add_player(&mut self, player_id: PlayerId) {
        println!("Adding new player {:?}", player_id);

        let state = self.states.get(0).unwrap();
        let new = state.add_player(player_id);
        self.push_state(new);
    }

    pub fn top_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    pub fn propagate_inputs(&mut self, mut inputs: Vec<TimedInput>) {
        if (inputs.len() == 0) {
            return;
        }

        println!("Propagating {} inputs", inputs.len());
        inputs.sort_by(|x, y| x.time_us.cmp(&y.time_us));

        for input in &inputs {
            // TODO optimisation
            // group all updates within same frame
            self.propagate_input(input);
        }
    }

    fn propagate_input(&mut self, input: &TimedInput) {
        if let Some(index) = self.split_with_input(input.player_id, input.input, input.time_us)
        {
            // TODO handle index == 0
            if (index > 0) {
                self.simulate_up_to_date(index);
            }

        }
    }

    fn simulate_up_to_date(&mut self, start_index : usize) {
        for i in (0..start_index).rev() {
            let inputs = self.states[i].player_inputs;
            let dt = self.states[i].time_us - self.states[i+1].time_us;
            let replacement_state = self.states[i+1].simulate(Some(inputs), dt as u32);
            self.states[i] = replacement_state;
        }
    }

    fn split_with_input(&mut self, player_id: PlayerId, input : Input, time_us : u32) -> Option<usize> {

        // Given some time t
        // Find the states before and after t s0 and s1, insert a new state s
        // between them
        //
        //     t0  t  t1
        //     |   |  |
        //  .. s0  s  s1 ..

        let before = self.get_index_before_us(time_us)?;

        if before == 0 {
            None
        }
        else {
            let state_before = &self.states[before];
            let dt = time_us - state_before.time_us;

            let after = before-1;

            let mut inputs = self.states[after].player_inputs.clone();
            inputs.set(player_id, input);
            let mut split_state = state_before
                .simulate(Some(inputs), dt as u32);
            split_state.frame_id -= 0.5;
            drop(state_before);

            self.states.insert(before, split_state);
            Some(before)
        }
    }

    fn get_sandwich(&self, time_us: u32) -> (Option<usize>, Option<usize>) {
        let mut before = None;
        let mut after = None;
        for i in (0..self.states.len()).rev() {
            let t = self.states[i].time_us;
            if t > time_us {
                break;
            }

            before = Some(i);
            after = if i == 0 { None } else { Some(i - 1) };
        }

        println!("{:?}, {:?}", before, after);
        (before, after)
    }

    pub fn propagate_state(&mut self, server_timed_state: TimedState, local_player: PlayerId) {

        // /////////////////////////////////////////////////////////////
        //
        //              s_server
        //                |
        //        |       |
        // s0 .. s1 .. s2 | s3 .. s_now
        //
        // s0 oldest state stored
        // s1 last local state that had an influence on s_server
        // s2 s3 sandwich s_server
        //
        // Strat:
        // Pop all older than s1
        // s1 becomes the "trusted" state to base all else on
        //
        // create modified s_server' by using local player state 
        // from s2 and the inputs from s3
        // modify s3 .. s_now into s3' .. s_now'
        //
        // /////////////////////////////////////////////////////////////
        //
        // s1 .. s2 s_server' s3' .. s_now'
        //
        // /////////////////////////////////////////////////////////////

        let (before, after) = self.get_sandwich(server_timed_state.time_us);

        if let Some(x) = before {
            while self.states.len() > x + 1 {
                self.states.pop_back();
            }
        }

        let state_before_server = if before.is_some() {
            self.states.pop_back()
        } else {
            None
        };
        let state_after_server = after.map(|x| &self.states[x]);

        let mut server_state = GameState::from_server_parts(
            self.seed,
            server_timed_state.time_us,
            server_timed_state.player_states,
        );

        if let (Some(prev_state), Some(next_state)) = (state_before_server, state_after_server) {
            let inputs = next_state.player_inputs;
            let dt = server_state.time_us - prev_state.time_us;
            let game_state_with_local_pos = prev_state.simulate(Some(inputs), dt);
            let override_player_state = game_state_with_local_pos
                .get_player(local_player)
                .unwrap()
                .clone();

            let server_pos = server_state.get_player(local_player).unwrap().pos;
            let local_pos = override_player_state.pos;
            if (server_pos != local_pos) {
                println!(
                    "Overriding server pos {:?} with local {:?}",
                    server_pos, local_pos
                );
            }

            server_state.set_player_state(local_player, override_player_state);
        }

        println!("Pushing back t={}", server_state.time_us);
        self.states.push_back(server_state);

        println!("States len {}", self.states.len());

        // Simulate up to now
        for i in (0..self.states.len() - 1).rev() {
            let dt = self.states[i].time_us - self.states[i + 1].time_us;
            println!(
                "Simulating up to date {} dt {}, t {}",
                i, dt, self.states[i].time_us
            );
            let inputs = Some(self.states[i].player_inputs);
            let new_state = self.states[i + 1].simulate(inputs, dt);
            self.states[i] = new_state;
        }
    }

    pub fn current_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    fn get_state_before_us(&self, time_us: u32) -> Option<&GameState>
    {
        self.get_index_before_us(time_us).map(|x| &self.states[x])
    }

    // Find the first state at a time point before a given time.
    fn get_index_before_us(&self, time_us: u32) -> Option<usize> {
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
        while self.states.len() > STATE_BUFFER_SIZE {
            self.states.pop_back();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    // Can keep around an hour before overflowing
    // Should be fine
    // Only worry is drift from summing, going to matter?
    pub time_us: u32,

    pub player_states: Vec<PlayerState>,
    pub player_inputs: PlayerInputs,
    pub log_states: Vec<LogState>,
    pub frame_id: f64,
}

impl GameState {
    fn new() -> Self {
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
            player_states,
            player_inputs: PlayerInputs::new(),
            log_states: vec![],
            //TODO
            frame_id: 0.0,
        }
    }

    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerState> {
        let index = id.0 as usize;
        if (index < self.player_states.len()) {
            Some(&self.player_states[index])
        } else {
            None
        }
    }

    fn get_player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        &mut self.player_states[id.0 as usize]
    }

    fn set_player_state(&mut self, id: PlayerId, state: PlayerState) {
        self.player_states[id.0 as usize] = state;
    }

    pub fn get_player_count(&self) -> usize {
        self.player_states.len()
    }

    pub fn add_player(&self, id: PlayerId) -> Self {
        let mut new = self.clone();

        let state = PlayerState {
            id,
            move_state: MoveState::Stationary,
            move_cooldown: 0.0,
            pos: Pos::Coord(CoordPos { x: 10, y: 10 }),
        };

        new.player_states.push(state);

        new
    }

    fn simulate(&self, input: Option<PlayerInputs>, dt_us: u32) -> Self {
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
            let id = self.player_states[i].id;
            let player_input = self.player_inputs.get(id);
            let iterated = self.player_states[i].tick_iterate(self, player_input, dt_us as f64);
            self.player_states[i] = iterated;
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
const MOVE_COOLDOWN_MAX: f64 = 150.0 * 1000.0;
const MOVE_DUR: f64 = 10.0 * 1000.0;

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

                    for player in &state.player_states {
                        if player.id != new.id {
                            if player.pos == candidate_pos {
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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_split()
    {
        let mut game = Game::new();
        game.add_player(PlayerId(0));
        game.tick_current_time(Some(PlayerInputs::default()), 50 * 1000);
        assert_eq!(3, game.states.len());

        let index = game.split_with_input(PlayerId(0), Input::Left, 10 * 1000);

        assert_eq!(Some(1), index);
        assert_eq!(4, game.states.len());
        assert_eq!(vec![50, 10, 0, 0],
            game.states.iter().map(|x| x.time_us / 1000).collect::<Vec<_>>());

        let input = game.states[1].player_inputs.get(PlayerId(0));
        assert_eq!(Input::Left, input);

        let state = &game.states[1].player_states[0];
        assert_eq!(MoveState::Moving(MOVE_DUR, Pos::Coord(CoordPos {x: 9, y: 10})), state.move_state);
    }

    #[test]
    fn test_split_front()
    {
        let mut game = Game::new();
        game.add_player(PlayerId(0));
        game.tick_current_time(Some(PlayerInputs::default()), 15 * 1000);
        assert_eq!(3, game.states.len());

        let index = game.split_with_input(PlayerId(0), Input::Left, 30 * 1000);

        assert_eq!(None, index);
        assert_eq!(3, game.states.len());
        assert_eq!(vec![15, 0, 0],
            game.states.iter().map(|x| x.time_us / 1000).collect::<Vec<_>>());
    }

    #[test]
    fn test_split_out_range()
    {
        let mut game = Game::from_server_parts(0, 10*1000, Vec::new(), 0);
        game.add_player(PlayerId(0));
        game.tick_current_time(Some(PlayerInputs::default()), 15 * 1000);
        assert_eq!(3, game.states.len());

        // Before start
        let index = game.split_with_input(PlayerId(0), Input::Left, 5 * 1000);
        assert_eq!(None, index);
        assert_eq!(3, game.states.len());
    }

    #[test]
    fn test_propagate_input()
    {
        let mut game = Game::new();
        game.add_player(PlayerId(0));
        game.add_player(PlayerId(1));

        let pos_p0_0 = game.top_state().get_player(PlayerId(0)).unwrap().pos;
        let pos_p1_0 = game.top_state().get_player(PlayerId(1)).unwrap().pos;
        let pos_p0_shifted;
        if let Pos::Coord(x) = pos_p0_0
        {
            pos_p0_shifted = Pos::Coord(x.apply_input(Input::Left));
        } 
        else
        {
            panic!("Expected initial pos of player 0 to be a coord");
        }

        game.tick_current_time(Some(PlayerInputs::default()), 50 * 1000);
        game.tick_current_time(Some(PlayerInputs::default()), 100 * 1000);
        game.tick_current_time(None, 150 * 1000);

        let timed_input = TimedInput
        {
            time_us: 65 * 1000,
            input : Input::Left,
            player_id: PlayerId(0),
        };

        game.propagate_input(&timed_input);

        assert_eq!(7, game.states.len());

        for i in (0..5) {
            let pos_p1 = game.states[i].get_player(PlayerId(1)).unwrap().pos;
            let state_p1 = &game.states[i].get_player(PlayerId(1)).unwrap().move_state;

            // Expect no change to p1
            assert_eq!(pos_p1_0, pos_p1);
            assert_eq!(MoveState::Stationary, *state_p1);
        }

        for i in 0..2 {
            let pos_p0 = game.states[i].get_player(PlayerId(0)).unwrap().pos;
            let state_p0 = &game.states[i].get_player(PlayerId(0)).unwrap().move_state;
            assert_eq!(pos_p0_shifted, pos_p0, "i = {}", i);
            assert_eq!(MoveState::Stationary, *state_p0);
        }

        // At state 2 should be in original position but moving to new pos
        let state_p0 = &game.states[2].get_player(PlayerId(0)).unwrap().move_state;
        assert_eq!(MoveState::Moving(MOVE_DUR, pos_p0_shifted), *state_p0);

        for i in 2..5 {
            let pos_p0 = game.states[i].get_player(PlayerId(0)).unwrap().pos;
            assert_eq!(pos_p0_0, pos_p0, "i = {}", i);
        }
    }
}