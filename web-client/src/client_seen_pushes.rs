use std::collections::{VecDeque, BTreeMap};
use crossy_multi_core::{timeline::Timeline, PlayerId, player::{PushInfo, MoveState}, PreciseCoords};
use froggy_rand::FroggyRand;

#[derive(Default)]
pub struct ClientSeenPushManager {
    pub pushes : BTreeMap<PushTriple, PushData>,
    pub last_round_id : Option<u8>,
}

impl ClientSeenPushManager {
    pub fn tick(&mut self, timeline: &Timeline) {
        let current_round_id = Some(timeline.top_state().get_round_id());
        let reset = current_round_id != self.last_round_id;
        self.last_round_id = current_round_id;

        if (reset) {
            self.pushes.clear();
        }

        let archive_frame_id = timeline.top_state().frame_id.saturating_sub(512);

        for (k, v) in self.pushes.iter_mut() {
            v.state = if (k.frame_id < archive_frame_id) {
                PushDataState::Archived
            }
            else {
                PushDataState::Invalid
            };
        }

        for state in timeline.states.iter().rev() {
            for (id, player_state) in state.player_states.iter() {
                if let MoveState::Moving(ms) = &player_state.move_state {
                    if let Some(pushee_id) = &ms.push_info.pushing {
                        let push_triple = PushTriple::from_info_with_pushing(id, &ms.push_info);
                        let pusher_pos = &player_state.pos;
                        let pushee_pos = &state.player_states.get(*pushee_id).unwrap().pos;
                        let push_data = PushData {
                            state: PushDataState::Valid,
                            pusher_pos : timeline.map.realise_pos(state.time_us, pusher_pos),
                            pushee_pos: timeline.map.realise_pos(state.time_us, pushee_pos),
                        };

                        _ = self.pushes.insert(push_triple, push_data);
                        /*
                        if let Some(mut_info) = self.pushes.get_mut(&push_triple) {
                            mut_info.invalidated = false;
                        }
                        else {
                            self.pushes.insert(push_triple, PushData { invalidated: false });
                        }
                        */
                    }
                }
            }
        }

        //log!("{:?}", self.pushes);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PushTriple
{
    pub pusher : PlayerId,
    pub pushee : PlayerId,
    pub frame_id : u32,
}

impl PushTriple {
    pub fn from_info_with_pushing(player_id : PlayerId, push_info : &PushInfo) -> Self {
        assert!(push_info.pushing.is_some());

        Self {
            pusher : player_id,
            pushee : push_info.pushing.unwrap(),
            frame_id: push_info.push_start_frame_id,
        }
    }
}

#[derive(Debug)]
pub enum PushDataState {
    Valid,
    Invalid,
    Archived,
}

#[derive(Debug)]
pub struct PushData {
    pub pusher_pos : PreciseCoords,
    pub pushee_pos : PreciseCoords,
    pub state: PushDataState,
}