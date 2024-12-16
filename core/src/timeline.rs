use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use crate::crossy_ruleset::{RulesState, GameConfig};
use crate::map::Map;
use crate::game::*;
use crate::player::PlayerState;

//const STATE_BUFFER_SIZE: usize = 128;
const STATE_BUFFER_SIZE: usize = 512;

pub const TICK_INTERVAL_US : u32 = 16_666;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteInput {
    pub time_us: u32,
    pub frame_id: u32,
    pub input: Input,
    pub player_id: PlayerId,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteTickState {
    pub frame_id : u32,
    pub time_us: u32,
    pub states: Vec<PlayerState>,
}

impl RemoteTickState {
    pub fn from_gamestate(game_state : &GameState) -> Self {
        Self {
            frame_id: game_state.frame_id,
            time_us: game_state.time_us,
            states: game_state.get_valid_player_states(),
        }
    }
}

#[derive(Debug)]
pub struct Timeline {
    pub states: VecDeque<GameState>,
    pub map : Map,
}

impl Timeline {
    pub fn new(config : GameConfig) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new(config));
        Timeline {
            states,
            map : Map::new(0),
        }
    }

    pub fn from_seed(config : GameConfig, seed: &str) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::new(config));
        Timeline {
            states,
            map : Map::new(seed),
        }
    }

    pub fn set_game_id(&mut self, game_id: u32) {
        // @Hack
        self.states.front_mut().unwrap().rules_state.game_id = game_id;
    }

    pub fn from_server_parts(
        seed: &str,
        frame_id : u32,
        time_us: u32,
        player_states: Vec<PlayerState>,
        rules_state : RulesState
    ) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::from_server_parts(frame_id, time_us, player_states, rules_state));
        Timeline {
            states,
            map: Map::new(seed),
        }
    }

    pub fn from_server_parts_exact_seed(
        seed: u32,
        frame_id : u32,
        time_us: u32,
        player_states: Vec<PlayerState>,
        rules_state: RulesState
    ) -> Self {
        let mut states = VecDeque::new();
        states.push_front(GameState::from_server_parts(frame_id, time_us, player_states, rules_state));
        Timeline {
            states,
            map: Map::exact_seed(seed),
        }
    }

    pub fn tick(&mut self, input: Option<PlayerInputs>, dt_us: u32) {
        let state = self.states.get(0).unwrap();
        let new = state.simulate(input, dt_us, &self.map);
        self.push_state(new);
    }

    pub fn get_last_player_inputs(&self) -> PlayerInputs {
        self.top_state().player_inputs.clone()
    }

    pub fn add_player(&mut self, player_id: PlayerId, pos: Pos) {
        let mut new_front = self.states.front().unwrap().add_player(player_id, pos);
        std::mem::swap(self.states.front_mut().unwrap(), &mut new_front);
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        // Remove from history, is this the correct thing to do?
        debug_log!("Dropping player {player_id:?}");
        let mut states = VecDeque::with_capacity(self.states.len());
        std::mem::swap(&mut self.states, &mut states);
        for state in &states {
            let new = state.remove_player(player_id);
            self.states.push_back(new);
        }
    }

    pub fn top_state(&self) -> &GameState {
        self.states.get(0).unwrap()
    }

    pub fn try_get_state(&self, frame_id : u32) -> Option<&GameState> {
        if (frame_id > self.top_state().frame_id) {
            return None;
        }

        let offset = self.frame_id_to_frame_offset(frame_id)?;
        self.states.get(offset)
    }

    pub fn inputs_since_frame(&self, frame_id : u32) -> Vec<RemoteInput> {
        if let Some(mut offset) = self.frame_id_to_frame_offset(frame_id)
        {
            let mut inputs = Vec::with_capacity(offset);

            loop {
                let state = self.states.get(offset).unwrap();
                for (player_id, _player_state) in state.player_states.iter() {

                    let input = state.player_inputs.get(player_id);

                    const ALLOW_EMPTY_INPUTS_FOR_TESTING : bool = false;
                    if (ALLOW_EMPTY_INPUTS_FOR_TESTING || input != Input::None)
                    {
                        inputs.push(RemoteInput {
                            frame_id : state.frame_id,
                            time_us: state.time_us,
                            input,
                            player_id,
                        });
                    }
                }

                if let Some(offset_updated) = offset.checked_sub(1) {
                    offset = offset_updated;
                }
                else {
                    break;
                }
            }

            inputs
        }
        else
        {
            Vec::new()
        }
    }

    pub fn rebase(&self, base : &GameState) -> Self
    {
        let current_frame_id = self.top_state().frame_id;

        let mut new_timeline = Self {
            states : Default::default(),
            map : self.map.clone(),
        };

        new_timeline.states.push_back(base.clone());

        // TODO do we need to keep track of added / removed players her?
        // I think not
        // Otherwise move call to resimulate up to date.
        while {
            new_timeline.top_state().frame_id < current_frame_id
        } {
            let mut inputs = PlayerInputs::default();
            if let Some(state) = self.try_get_state(new_timeline.top_state().frame_id + 1)
            {
                inputs = state.player_inputs.clone();
            }
            new_timeline.tick(Some(inputs), TICK_INTERVAL_US);
        }

        assert!(self.top_state().frame_id == new_timeline.top_state().frame_id);
        assert!(self.top_state().time_us == new_timeline.top_state().time_us);

        new_timeline
    }

    pub fn try_propagate_inputs(&mut self, mut inputs: Vec<RemoteInput>, is_server : bool) -> bool {
        if (inputs.is_empty()) {
            return true;
        }

        // Can we assume its already sorted?
        inputs.sort_by(|x, y| x.frame_id.cmp(&y.frame_id));

        let last_propagating_frame_id = inputs.last().unwrap().frame_id;
        let current_frame_id = self.top_state().frame_id;
        //debug_log!("Propagating inputs, top frame has delta {}", current_frame_id as i32 - last_propagating_frame_id as i32);

        if (last_propagating_frame_id > current_frame_id) {
            debug_log!("Trying to propagate inputs from the future!\n\n frame_id {}\n states len {}\n\n states {:?}\n\n inputs {:?}", last_propagating_frame_id, self.states.len(), self.top_state(), inputs);
            return false;
        }

        let mut resimulation_frame_id = None;

        for input in &inputs {

            if let Some(frame_offset) = self.frame_id_to_frame_offset(input.frame_id)
            {
                let state_mut = self.states.get_mut(frame_offset).unwrap();

                // @TEMPORARY please cleanup
                // To debug issues we have allowed the server to send empty inputs
                // So we check here to make sure we arent overriding an actual input with an empty one
                // sent by the server before it has received the real input.

                if (state_mut.player_inputs.get(input.player_id) == Input::None)
                {
                    if (self.states.get_mut(frame_offset).unwrap().player_inputs.set(input.player_id, input.input))
                    {
                        // There was some change
                        if let Some(_) = self.frame_id_to_frame_offset(input.frame_id - 1)
                        {
                            //debug_log!("Propagate inputs, change on input {:#?}", input);

                            let new_resim_frame_id = (input.frame_id - 1).min(resimulation_frame_id.unwrap_or(u32::MAX));
                            resimulation_frame_id = Some(new_resim_frame_id);
                        }
                    }
                }
            }
            else
            {
                // Warning this can happen on resets.
                //panic!("Argh! couldnt fetch frame offset for frame id {}, front {}, back {}", input.frame_id, self.states.front().unwrap().frame_id, self.states.back().unwrap().frame_id);
            }
        }

        if let Some(resim_id) = resimulation_frame_id
        {
            //debug_log!(">> Resimulating!");
            let before = self.current_state().clone();
            let start_frame_offset = self.frame_id_to_frame_offset(resim_id).unwrap();
            self.simulate_up_to_date(start_frame_offset, is_server);

            if (self.current_state().player_states == before.player_states) {
                //debug_log!("Resimulating produced the same top state, probably a problem");
                //debug_log!("Before {:#?}", before.player_states);
                //debug_log!("After {:#?}", self.current_state().player_states);
            }
        }

        true
    }

    fn frame_id_to_frame_offset(&self, frame_id : u32) -> Option<usize>
    {
        //assert!(frame_id <= self.states.front().unwrap().frame_id);
        let assert_condition = frame_id <= self.states.front().unwrap().frame_id;
        if (!assert_condition) {
            let bt = backtrace::Backtrace::new();
            panic!("Ahhh! frame_id {} states len {} states front {:?}, backtrace {:?}", frame_id, self.states.len(), self.states.front().map(|x| x.frame_id),  bt);
        }

        let first_state = self.states.back()?;
        let offset_back = frame_id.checked_sub(first_state.frame_id)? as usize;
        let offset_front = self.states.len() - offset_back - 1;
        {
            if let Some(got_frame) = self.states.get(offset_front)
            {
                if (frame_id != got_frame.frame_id)
                {
                    panic!("Error looking up frame {}, got {}",frame_id, got_frame.frame_id) ;
                }
            }
            else
            {
                //panic!("Error looking up frame {}, could not fetch state with offset {}", frame_id, offset_front);
                return None;
            }
        }
        Some(offset_front)
    }

    fn simulate_up_to_date(&mut self, start_frame_offset: usize, is_server : bool) {
        let mut remove_ids = Vec::new();

        for i in (0..start_frame_offset).rev() {
            let dt = self.states[i].time_us - self.states[i + 1].time_us;

            let inputs = self.states[i].player_inputs.clone();
            let mut replacement_state = self.states[i + 1].simulate(Some(inputs), dt as u32, &self.map);

            // Add any newly added players between existing state_i+1 and state_i
            {
                for (id, player_state) in self.states[i].player_states.iter() {
                    if (!replacement_state.player_states.contains(id))
                    {
                        replacement_state.player_states.set(id, player_state.clone());
                    }
                }
            }

            // Prune any removed players between state_i+1 and state_i
            // but only on server side
            if (is_server)
            {
                remove_ids.clear();
                for (id, _) in replacement_state.player_states.iter()
                {
                    if (!self.states[i].player_states.contains(id))
                    {
                        remove_ids.push(id);
                    }
                }

                for id in &remove_ids
                {
                    replacement_state = replacement_state.remove_player(*id);
                }
            }

            assert!(self.states[i].frame_id == replacement_state.frame_id);
            assert!(self.states[i].time_us == replacement_state.time_us);

            self.states[i] = replacement_state;
        }
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
}
