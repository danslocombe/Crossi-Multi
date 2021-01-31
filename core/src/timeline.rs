use std::collections::VecDeque;

use super::game::*;

const STATE_BUFFER_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct RemoteInput {
    pub time_us: u32,
    pub input: Input,
    pub player_id: PlayerId,
}

#[derive(Debug, Clone)]
pub struct RemoteState {
    pub time_us: u32,
    pub last_sent_us : u32,
    pub player_states: Vec<PlayerState>,
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub player_count: u8,
    pub seed: u32,
    states: VecDeque<GameState>,
}

impl Timeline {
    pub fn new() -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Timeline {
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
        Timeline {
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

    pub fn add_player(&mut self, player_id: PlayerId, pos : Pos) {
        println!("Adding new player {:?}", player_id);

        let state = self.states.get(0).unwrap();
        let new = state.add_player(player_id, pos);
        self.push_state(new);
    }

    pub fn top_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    pub fn propagate_inputs(&mut self, mut inputs: Vec<RemoteInput>) {
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

    fn propagate_input(&mut self, input: &RemoteInput) {
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

    pub fn propagate_state(&mut self, remote_state: &RemoteState, local_player: PlayerId) {

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


        if let Some(before) = self.get_index_before_us(remote_state.last_sent_us) {
            // Before points to an index before the last tick that was sent to server
            // Pop off state with index before
            while self.states.len() > before {
                self.states.pop_back();
            }
        }

        if let Some(index) = self.split_with_state(local_player, &remote_state.player_states, remote_state.time_us) {
            if (index > 0) {
                self.simulate_up_to_date(index);
            }
        }
    }

    fn split_with_state(&mut self, player_id: PlayerId, states : &Vec<PlayerState>, time_us : u32) -> Option<usize> {

        let before = self.get_index_before_us(time_us)?;

        if before == 0 {
            None
        }
        else {
            let state_before = &self.states[before];
            let dt = time_us - state_before.time_us;

            let mut split_state = state_before
                .simulate(None, dt as u32);

            for server_player_state in states {
                if server_player_state.id != player_id {
                    split_state.set_player_state(server_player_state.id, server_player_state.clone());
                }
            }

            split_state.frame_id -= 0.5;
            drop(state_before);

            self.states.insert(before, split_state);
            Some(before)
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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_split()
    {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 50_000);
        assert_eq!(3, timeline.states.len());

        let index = timeline.split_with_input(PlayerId(0), Input::Left, 10_000);

        assert_eq!(Some(1), index);
        assert_eq!(4, timeline.states.len());
        assert_eq!(vec![50, 10, 0, 0],
            timeline.states.iter().map(|x| x.time_us / 1000).collect::<Vec<_>>());

        let input = timeline.states[1].player_inputs.get(PlayerId(0));
        assert_eq!(Input::Left, input);

        let state = &timeline.states[1].get_player(PlayerId(0)).unwrap();
        assert_eq!(MoveState::Moving(MOVE_DUR, Pos::new_coord(-1, 0)), state.move_state);
    }

    #[test]
    fn test_split_front()
    {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 15_000);
        assert_eq!(3, timeline.states.len());

        let index = timeline.split_with_input(PlayerId(0), Input::Left, 30_000);

        assert_eq!(None, index);
        assert_eq!(3, timeline.states.len());
        assert_eq!(vec![15, 0, 0],
            timeline.states.iter().map(|x| x.time_us / 1000).collect::<Vec<_>>());
    }

    #[test]
    fn test_split_out_range()
    {
        let mut timeline = Timeline::from_server_parts(0, 10_000, Vec::new(), 0);
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 15_000);
        assert_eq!(3, timeline.states.len());

        // Before start
        let index = timeline.split_with_input(PlayerId(0), Input::Left, 5_000);
        assert_eq!(None, index);
        assert_eq!(3, timeline.states.len());
    }

    #[test]
    fn test_propagate_input()
    {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.add_player(PlayerId(1), Pos::new_coord(5, 5));

        timeline.tick_current_time(Some(PlayerInputs::default()), 50_000);
        timeline.tick_current_time(Some(PlayerInputs::default()), 100_000);
        timeline.tick_current_time(None, 150_000);

        let timed_input = RemoteInput
        {
            time_us: 65_000,
            input : Input::Left,
            player_id: PlayerId(0),
        };

        timeline.propagate_input(&timed_input);

        assert_eq!(7, timeline.states.len());

        for i in (0..5) {
            let pos = timeline.states[i].get_player(PlayerId(1)).unwrap().pos;
            let state = &timeline.states[i].get_player(PlayerId(1)).unwrap().move_state;

            // Expect no change to p1
            assert_eq!(Pos::new_coord(5, 5), pos);
            assert_eq!(MoveState::Stationary, *state);
        }

        for i in 0..2 {
            let pos = timeline.states[i].get_player(PlayerId(0)).unwrap().pos;
            let state = &timeline.states[i].get_player(PlayerId(0)).unwrap().move_state;
            assert_eq!(Pos::new_coord(-1, 0), pos, "i = {}", i);
            assert_eq!(MoveState::Stationary, *state);
        }

        {
            // At state 2 should be in original position but moving to new pos
            let state = &timeline.states[2].get_player(PlayerId(0)).unwrap().move_state;
            assert_eq!(MoveState::Moving(MOVE_DUR, Pos::new_coord(-1, 0)), *state);
        }

        for i in 2..5 {
            let pos = timeline.states[i].get_player(PlayerId(0)).unwrap().pos;
            assert_eq!(Pos::new_coord(0, 0), pos, "i = {}", i);
        }
    }

    #[test]
    fn test_propagate_state()
    {
        let mut client_timeline = Timeline::new();
        client_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        client_timeline.add_player(PlayerId(1), Pos::new_coord(5, 5));

        let mut p0_left = PlayerInputs::default();
        p0_left.set(PlayerId(0), Input::Left);
        client_timeline.tick_current_time(Some(p0_left), 500_000);

        let mut server_timeline = client_timeline.clone();

        client_timeline.tick_current_time(Some(p0_left), 1_000_000);
        client_timeline.tick_current_time(Some(p0_left), 1_500_000);

        let mut p1_left = PlayerInputs::default();
        p1_left.set(PlayerId(1), Input::Left);
        server_timeline.tick_current_time(Some(p1_left), 1_250_000);

        let server_state = RemoteState
        {
            time_us : 1_250000,
            last_sent_us : 500_000,
            player_states : server_timeline.top_state().get_valid_player_states(),
        };


        assert_eq!(6, client_timeline.states.len());
        {
            let p0 = client_timeline.top_state().get_player(PlayerId(0)).unwrap();
            assert_eq!(Pos::new_coord(-2, 0), p0.pos);
            assert_eq!(MoveState::Moving(MOVE_DUR, Pos::new_coord(-3, 0)), p0.move_state);
        }

        client_timeline.propagate_state(&server_state, PlayerId(0));

        assert_eq!(vec![1500, 1250, 1000, 500],
            client_timeline.states.iter().map(|x| x.time_us / 1000).collect::<Vec<_>>());

        let s = client_timeline.top_state();
        let p0 = s.get_player(PlayerId(0)).unwrap();
        let p1 = s.get_player(PlayerId(1)).unwrap();
        assert_eq!(Pos::new_coord(-2, 0), p0.pos);
        assert_eq!(MoveState::Moving(MOVE_DUR, Pos::new_coord(-3, 0)), p0.move_state);
        assert_eq!(Pos::new_coord(4, 5), p1.pos);
        assert_eq!(MoveState::Stationary, p1.move_state);
    }
}