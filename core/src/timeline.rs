use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use crate::crossy_ruleset::CrossyRulesetFST;
use crate::map::Map;
use crate::game::*;
use crate::player::PlayerState;

const STATE_BUFFER_SIZE: usize = 128;

#[derive(Debug, Clone)]
pub struct RemoteInput {
    pub time_us: u32,
    pub input: Input,
    pub player_id: PlayerId,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteTickState {
    pub time_us: u32,
    pub states: Vec<PlayerState>,
}

#[derive(Debug)]
pub struct Timeline {
    states: VecDeque<GameState>,
    pub map : Map,
}

impl Timeline {
    pub fn new() -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Timeline {
            states,
            map : Map::new(0),
        }
    }

    pub fn from_seed(seed: u32) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new());
        Timeline {
            states,
            map : Map::new(seed),
        }
    }

    pub fn from_server_parts(
        seed: u32,
        time_us: u32,
        player_states: Vec<PlayerState>,
        ruleset_state : CrossyRulesetFST
    ) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::from_server_parts(seed, time_us, player_states, ruleset_state));
        Timeline {
            states,
            map: Map::new(seed),
        }
    }

    pub fn tick(&mut self, input: Option<PlayerInputs>, dt_us: u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us, &self.map);
        self.push_state(new);
    }

    pub fn tick_current_time(&mut self, input: Option<PlayerInputs>, time_us: u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, time_us - state.time_us, &self.map);
        self.push_state(new);
    }

    pub fn get_last_player_inputs(&self) -> PlayerInputs {
        self.top_state().player_inputs.clone()
    }

    pub fn add_player(&mut self, player_id: PlayerId, pos: Pos) {
        debug_log!("Adding new player {:?}", player_id);

        let new = self.top_state().add_player(player_id, pos);
        self.push_state(new);
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        debug_log!("Dropping player {player_id:?}");

        let new = self.top_state().remove_player(player_id);
        self.push_state(new);
    }

    pub fn top_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    pub fn set_player_ready(&mut self, player_id : PlayerId, ready_state : bool) {
        let new = self.top_state().set_player_ready(player_id, ready_state);
        self.push_state(new);
    }

    pub fn propagate_inputs(&mut self, mut inputs: Vec<RemoteInput>) {
        if (inputs.is_empty()) {
            return;
        }

        inputs.sort_by(|x, y| x.time_us.cmp(&y.time_us));

        for input in &inputs {
            if (input.input != Input::None) {
                self.propagate_input(input);
            }
        }
    }

    fn propagate_input(&mut self, input: &RemoteInput) {
        if let Some(index) = self.split_with_input(input.player_id, input.input, input.time_us) {
            // TODO handle index == 0
            if (index > 0) {
                self.simulate_up_to_date(index);
            }
        }
    }

    fn simulate_up_to_date(&mut self, start_index: usize) {
        for i in (0..start_index).rev() {
            let inputs = self.states[i].player_inputs.clone();
            let dt = self.states[i].time_us - self.states[i + 1].time_us;
            let replacement_state = self.states[i + 1].simulate(Some(inputs), dt as u32, &self.map);
            self.states[i] = replacement_state;
        }
    }

    fn split_with_input(
        &mut self,
        player_id: PlayerId,
        input: Input,
        time_us: u32,
    ) -> Option<usize> {
        // Given some time t
        // Find the states before and after t s0 and s1, insert a new state s
        // between them
        //
        //     t0  t  t1
        //     |   |  |
        //  .. s0  s  s1 ..

        let before = self.get_index_before_us(time_us)?;

        if before == 0 {
            // TODO handle super-low latency edgecase
            // Can only happen when latency < frame delay
            None
        } else {
            let state_before = &self.states[before];
            let dt = time_us - state_before.time_us;

            let after = before - 1;

            let mut inputs = self.states[after].player_inputs.clone();
            inputs.set(player_id, input);
            let mut split_state = state_before.simulate(Some(inputs), dt as u32, &self.map);
            split_state.frame_id -= 0.5;

            self.states.insert(before, split_state);
            Some(before)
        }
    }

    pub fn propagate_state(
        &mut self,
        latest_remote_state: &RemoteTickState,
        rule_state : Option<&CrossyRulesetFST>,
        client_latest_remote_state: Option<&RemoteTickState>,
        local_player: Option<PlayerId>,
    ) {
        // /////////////////////////////////////////////////////////////
        //    client_last     s_server
        //        |              |
        //        |              |
        // s0 .. s1 ..     .. s2 | s3 .. s_now
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

        let mut use_client_predictions : Vec<PlayerId> = local_player.into_iter().collect();

        if let Some(state) = client_latest_remote_state.as_ref() {
            if let Some(index) = self.split_with_state(&[], &state.states, None, state.time_us) {
                while self.states.len() > index + 1 {
                    self.states.pop_back();
                }

                if (index > 0) {
                    self.simulate_up_to_date(index);

                    if let Some(lp) = local_player {
                        use_client_predictions = self.players_to_use_client_predictions(index, lp);
                        //if (use_client_predictions.len() > 1) {
                            //crate::debug_log(&format!("{:?}", use_client_predictions));
                        //}
                    }
                }
            }
        }

        if let Some(index) = self.split_with_state(
            &use_client_predictions,
            &latest_remote_state.states,
            rule_state,
            latest_remote_state.time_us,
        ) {
            if (index > 0) {
                self.simulate_up_to_date(index);
            }
        }
    }

    fn split_with_state(
        &mut self,
        ignore_player_ids: &[PlayerId],
        server_states: &[PlayerState],
        maybe_server_rule_state : Option<&CrossyRulesetFST>,
        time_us: u32,
    ) -> Option<usize> {
        let before = self.get_index_before_us(time_us)?;

        if before == 0 {
            None
        } else {
            let state_before = &self.states[before];
            let dt = time_us - state_before.time_us;

            let mut split_state = state_before.simulate(None, dt as u32, &self.map);

            for server_player_state in server_states {
                if (!ignore_player_ids.contains(&server_player_state.id)) {
                    split_state
                        .set_player_state(server_player_state.id, server_player_state.clone());
                }
            }

            if let Some(server_rule_state) = maybe_server_rule_state {
                split_state.ruleset_state = server_rule_state.clone();
            }

            split_state.frame_id -= 0.5;
            self.states.insert(before, split_state);
            Some(before)
        }
    }

    fn players_to_use_client_predictions(&self, index : usize, local_player : PlayerId) -> Vec<PlayerId> {
        let mut player_ids = vec![local_player];

        for i in (0..=index).rev() {
            let state = &self.states[i];

            for player in &state.get_valid_player_states() {
                let mut to_add : Option<PlayerId> = None;
                for pid in &player_ids {
                    if player.is_being_pushed_by(*pid) {
                        to_add = Some(player.id);
                        break;
                    }
                }

                // Bug
                // Edge case where we dont add secondary push if on the last frame
                // Think its fine to ignore

                match to_add {
                    Some(pid) => {
                        if !player_ids.contains(&pid) {
                            player_ids.push(pid);
                        }
                    },
                    _ => {},
                }
            }
        }

        player_ids
    }

    pub fn current_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    // Find the first state at a time point before a given time.
    pub fn get_index_before_us(&self, time_us: u32) -> Option<usize> {
        // TODO binary search
        for i in 0..self.states.len() {
            let state = &self.states[i];
            if (state.time_us < time_us) {
                return Some(i);
            }
        }

        None
    }

    pub fn get_state_before_eq_us(&self, time_us: u32) -> Option<&GameState> {
        self.get_index_before_eq_us(time_us)
            .map(|x| &self.states[x])
    }

    pub fn get_index_before_eq_us(&self, time_us: u32) -> Option<usize> {
        // TODO binary search
        // go down states until we find one with time < target
        for i in 0..self.states.len() {
            let state = &self.states[i];
            if (state.time_us <= time_us) {
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

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::*;

    // We dont want to actually expose this
    fn clone_timeline(timeline : &Timeline) -> Timeline {
        Timeline {
            map : Map::new(timeline.map.get_seed()),
            states : timeline.states.clone(),
        }
    }

    #[test]
    fn test_split() {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 50_000);
        assert_eq!(3, timeline.states.len());

        let index = timeline.split_with_input(PlayerId(0), Input::Left, 10_000);

        assert_eq!(Some(1), index);
        assert_eq!(4, timeline.states.len());
        assert_eq!(
            vec![50, 10, 0, 0],
            timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );

        let input = timeline.states[1].player_inputs.get(PlayerId(0));
        assert_eq!(Input::Left, input);

        let state = &timeline.states[1].get_player(PlayerId(0)).unwrap();
        match (&state.move_state) {
            MoveState::Moving(state) => {
                assert_eq!(MOVE_DUR, state.remaining_us);
                assert_eq!(Pos::new_coord(-1, 0), state.target);
            },
            _ => {
                panic!("Expected to be moving");
            },
        }
    }

    #[test]
    fn test_split_front() {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 15_000);
        assert_eq!(3, timeline.states.len());

        let index = timeline.split_with_input(PlayerId(0), Input::Left, 30_000);

        assert_eq!(None, index);
        assert_eq!(3, timeline.states.len());
        assert_eq!(
            vec![15, 0, 0],
            timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_split_out_range() {
        let mut timeline = Timeline::from_server_parts(0, 10_000, Vec::new(), CrossyRulesetFST::start());
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.tick_current_time(Some(PlayerInputs::default()), 15_000);
        assert_eq!(3, timeline.states.len());

        // Before start
        let index = timeline.split_with_input(PlayerId(0), Input::Left, 5_000);
        assert_eq!(None, index);
        assert_eq!(3, timeline.states.len());
    }

    #[test]
    fn test_propagate_input() {
        let mut timeline = Timeline::new();
        timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        timeline.add_player(PlayerId(1), Pos::new_coord(5, 5));

        timeline.tick_current_time(Some(PlayerInputs::default()), 50_000);
        timeline.tick_current_time(Some(PlayerInputs::default()), 200_000);
        timeline.tick_current_time(None, 300_000);

        let timed_input = RemoteInput {
            time_us: 65_000,
            input: Input::Left,
            player_id: PlayerId(0),
        };


        timeline.propagate_input(&timed_input);

        assert_eq!(7, timeline.states.len());

        for i in (0..5) {
            let pos = timeline.states[i].get_player(PlayerId(1)).unwrap().pos;
            let state = &timeline.states[i]
                .get_player(PlayerId(1))
                .unwrap()
                .move_state;

            // Expect no change to p1
            assert_eq!(Pos::new_coord(5, 5), pos);
            assert_eq!(MoveState::Stationary, *state);
        }

        for i in 0..2 {
            let pos = timeline.states[i].get_player(PlayerId(0)).unwrap().pos;
            let state = &timeline.states[i]
                .get_player(PlayerId(0))
                .unwrap()
                .move_state;
            assert_eq!(Pos::new_coord(-1, 0), pos, "i = {}", i);
            assert_eq!(MoveState::Stationary, *state);
        }

        {
            // At state 2 should be in original position but moving to new pos
            let mv = &timeline.states[2]
                .get_player(PlayerId(0))
                .unwrap()
                .move_state;
            match (mv) {
                MoveState::Moving(state) => {
                    assert_eq!(MOVE_DUR, state.remaining_us);
                    assert_eq!(Pos::new_coord(-1, 0), state.target);
                },
                _ => {
                    panic!("Expected to be moving");
                },
            }
        }

        for i in 2..5 {
            let pos = timeline.states[i].get_player(PlayerId(0)).unwrap().pos;
            assert_eq!(Pos::new_coord(0, 0), pos, "i = {}", i);
        }
    }

    #[test]
    fn test_propagate_state() {
        // Client makes some changes after the server ticks
        // Expect them to be respected and propagated forward

        let mut client_timeline = Timeline::new();
        client_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        client_timeline.add_player(PlayerId(1), Pos::new_coord(5, 5));

        let mut p0_left = PlayerInputs::default();
        p0_left.set(PlayerId(0), Input::Left);
        client_timeline.tick_current_time(Some(p0_left.clone()), 500_000);

        let mut server_timeline = clone_timeline(&client_timeline);

        client_timeline.tick_current_time(Some(p0_left.clone()), 1_000_000);
        client_timeline.tick_current_time(Some(p0_left.clone()), 1_500_000);

        let mut p1_left = PlayerInputs::default();
        p1_left.set(PlayerId(1), Input::Left);
        server_timeline.tick_current_time(Some(p1_left), 1_250_000);

        let server_state_latest = RemoteTickState {
            time_us: 1_250000,
            states: server_timeline.top_state().get_valid_player_states(),
        };

        let server_state_client = RemoteTickState {
            time_us: 500_000,
            states: server_timeline.states[1].get_valid_player_states(),
        };

        assert_eq!(6, client_timeline.states.len());
        {
            let p0 = client_timeline.top_state().get_player(PlayerId(0)).unwrap();
            assert_eq!(Pos::new_coord(-2, 0), p0.pos);
            match (&p0.move_state) {
                MoveState::Moving(state) => {
                    assert_eq!(MOVE_DUR, state.remaining_us);
                    assert_eq!(Pos::new_coord(-3, 0), state.target);
                },
                _ => {
                    panic!("Expected to be moving");
                },
            }
        }

        client_timeline.propagate_state(
            &server_state_latest,
            None,
            Some(&server_state_client),
            Some(PlayerId(0)),
        );

        assert_eq!(
            vec![1500, 1250, 1000, 500, 500],
            client_timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );

        let s = client_timeline.top_state();
        let p0 = s.get_player(PlayerId(0)).unwrap();
        let p1 = s.get_player(PlayerId(1)).unwrap();
        assert_eq!(Pos::new_coord(-2, 0), p0.pos);
        match (&p0.move_state) {
            MoveState::Moving(state) => {
                assert_eq!(MOVE_DUR, state.remaining_us);
                assert_eq!(Pos::new_coord(-3, 0), state.target);
            },
            _ => {
                panic!("Expected to be moving");
            },
        }
        assert_eq!(Pos::new_coord(4, 5), p1.pos);
        assert_eq!(MoveState::Stationary, p1.move_state);
    }

    #[test]
    fn test_no_client_tick() {
        // Propagate server inputs before the server has received input from us.

        let mut client_timeline = Timeline::new();
        client_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        client_timeline.add_player(PlayerId(1), Pos::new_coord(5, 5));

        let mut server_timeline = clone_timeline(&client_timeline);

        let mut p0_left = PlayerInputs::default();
        p0_left.set(PlayerId(0), Input::Left);
        client_timeline.tick_current_time(Some(p0_left), 400_000);

        let mut p1_right = PlayerInputs::default();
        p1_right.set(PlayerId(1), Input::Right);
        server_timeline.tick_current_time(Some(p1_right), 600_000);

        let server_state_latest = RemoteTickState {
            time_us: 600_000,
            states: server_timeline.top_state().get_valid_player_states(),
        };

        client_timeline.tick_current_time(None, 1_000_000);
        client_timeline.propagate_state(&server_state_latest, None, None, Some(PlayerId(0)));

        assert_eq!(
            vec![1_000, 600, 400, 0, 0, 0],
            client_timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );

        let s = client_timeline.top_state();

        let p0 = s.get_player(PlayerId(0)).unwrap();
        assert_eq!(Pos::new_coord(-1, 0), p0.pos);
        assert_eq!(MoveState::Stationary, p0.move_state);

        let p1 = s.get_player(PlayerId(1)).unwrap();
        assert_eq!(Pos::new_coord(6, 5), p1.pos);
        assert_eq!(MoveState::Stationary, p1.move_state);
    }

    #[test]
    fn test_client_disagrees_server() {
        // Server sends state that disagrees with our worldview
        // Accept server state but still apply our local inputs since on top

        let mut client_timeline = Timeline::new();
        client_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        client_timeline.add_player(PlayerId(1), Pos::new_coord(2, 2));

        let mut local_inputs = PlayerInputs::new();
        local_inputs.set(PlayerId(0), Input::Left);
        client_timeline.tick_current_time(None, 200_000);
        client_timeline.tick_current_time(Some(local_inputs), 700_000);

        let mut server_timeline = Timeline::new();
        server_timeline.add_player(PlayerId(0), Pos::new_coord(5, 5));
        server_timeline.add_player(PlayerId(1), Pos::new_coord(10, 10));

        let mut server_inputs = PlayerInputs::default();
        server_inputs.set(PlayerId(0), Input::Right);
        server_inputs.set(PlayerId(1), Input::Right);
        server_timeline.tick_current_time(None, 200_000);
        server_timeline.tick_current_time(Some(server_inputs), 450_000);

        let server_state_latest = RemoteTickState {
            time_us: 450_000,
            states: server_timeline.top_state().get_valid_player_states(),
        };

        let server_state_client_last = Some(RemoteTickState {
            time_us: 200_000,
            states: server_timeline.states[1].get_valid_player_states(),
        });

        client_timeline.tick_current_time(None, 1_000_000);

        client_timeline.propagate_state(
            &server_state_latest,
            None,
            server_state_client_last.as_ref(),
            Some(PlayerId(0)),
        );

        assert_eq!(
            vec![1_000, 700, 450, 200, 200],
            client_timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );

        let s = client_timeline.top_state();

        let p0 = s.get_player(PlayerId(0)).unwrap();
        // Expect to be at server initial position with local input (-1) added on top
        assert_eq!(Pos::new_coord(4, 5), p0.pos);
        assert_eq!(MoveState::Stationary, p0.move_state);

        let p1 = s.get_player(PlayerId(1)).unwrap();
        // Expect to be at server current position
        assert_eq!(Pos::new_coord(11, 10), p1.pos);
        assert_eq!(MoveState::Stationary, p1.move_state);
    }

    #[test]
    fn test_client_disagrees_server_client_moves_invalid_pos() {
        // Server sends state that disagrees with client position
        // Client makes a move that is invalid given new server states

        let mut client_timeline = Timeline::new();
        client_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        client_timeline.add_player(PlayerId(1), Pos::new_coord(2, 2));

        let mut local_inputs = PlayerInputs::new();
        local_inputs.set(PlayerId(0), Input::Right);
        client_timeline.tick_current_time(None, 200_000);
        client_timeline.tick_current_time(Some(local_inputs), 300_000);

        let mut server_timeline = Timeline::new();
        server_timeline.add_player(PlayerId(0), Pos::new_coord(0, 0));
        server_timeline.add_player(PlayerId(1), Pos::new_coord(1, 0));

        // Set player 1 as moving so that player 0 can't move in when
        // inputs synced from server.
        let mut server_inputs = PlayerInputs::new();
        server_inputs.set(PlayerId(1), Input::Up);
        server_timeline.tick_current_time(Some(server_inputs), 295_000);
        server_timeline.tick_current_time(None, 400_000);

        let server_state_latest = RemoteTickState {
            time_us: 400_000,
            states: server_timeline.top_state().get_valid_player_states(),
        };

        let server_state_client_last = Some(RemoteTickState {
            time_us: 200_000,
            states: server_timeline.states[1].get_valid_player_states(),
        });

        client_timeline.tick_current_time(None, 1_000_000);

        client_timeline.propagate_state(
            &server_state_latest,
            None,
            server_state_client_last.as_ref(),
            Some(PlayerId(0)),
        );

        assert_eq!(
            vec![1_000, 400, 300, 200, 200],
            client_timeline
                .states
                .iter()
                .map(|x| x.time_us / 1000)
                .collect::<Vec<_>>()
        );

        let s = client_timeline.top_state();

        let p0 = s.get_player(PlayerId(0)).unwrap();
        assert_eq!(Pos::new_coord(0, 0), p0.pos);
        assert_eq!(MoveState::Stationary, p0.move_state);

        let p1 = s.get_player(PlayerId(1)).unwrap();
        assert_eq!(Pos::new_coord(1, -1), p1.pos);
        assert_eq!(MoveState::Stationary, p1.move_state);
    }
}
