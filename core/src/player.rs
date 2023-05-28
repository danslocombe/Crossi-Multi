use serde::{Deserialize, Serialize};
use crate::crossy_ruleset::{CrossyRulesetFST, RulesState, AliveState};
use crate::map::Map;
use crate::game::*;


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
    //Sliding(MovingState),
}

pub struct Push {
    pub id : PlayerId,
    pub pushed_by : PlayerId,
    pub dir : Input,
}

enum TryMovePlayerState {
    Blocked,
    MoveUnimpeded,
    MoveWithPush,
}

// In us
// In original game move cd is 7 frames
//pub const MOVE_COOLDOWN_MAX: u32 = 150_000;
//pub const MOVE_COOLDOWN_MAX: u32 = 7 * (1_000_000 / 60);
pub const MOVE_COOLDOWN_MAX: u32 = 1;
pub const MOVE_DUR: u32 = 7 * (1_000_000 / 60);

impl PlayerState {
    pub fn can_move(&self) -> bool {
        if let MoveState::Stationary = self.move_state {
            self.move_cooldown == 0
        } else {
            false
        }
    }

    pub fn tick_iterate(&self, state: &GameState, input: Input, dt_us: u32, pushes : &mut Vec<Push>, map : &Map) -> Self {
        let mut new = self.clone();
        match new.move_state {
            MoveState::Stationary => {
                new.move_cooldown = new.move_cooldown.saturating_sub(dt_us);
            }
            MoveState::Moving(moving_state) => {
                match moving_state.remaining_us.checked_sub(dt_us)
                {
                    Some(remaining_us) =>
                    {
                        let mut new_state = moving_state;
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
                        new.move_cooldown = MOVE_COOLDOWN_MAX.saturating_sub(leftover_us);
                    },
                }
            }
        }

        if new.can_move() && input != Input::None {
            if let Some(moving_state) = new.try_move(input, state, pushes, map) {
                new.move_state = MoveState::Moving(moving_state);
            }
        }

        new
    }

    pub fn push(&self, push : &Push, state : &GameState, map : &Map) -> Self {
        let mut current_pos = self.pos.clone();
        if let MoveState::Moving(ms) = &self.move_state {
            if (ms.remaining_us as f32) < MOVE_DUR as f32 * 0.5 {
                current_pos = ms.target.clone();
            }
        }

        let m_new_pos = map.try_apply_input(state.time_us, &state.rules_state, &current_pos, push.dir);

        if let Some(new_pos) = m_new_pos {
            let mut new = self.clone();
            let mut push_info = PushInfo::default();
            push_info.pushed_by = Some(push.pushed_by);

            // @nocheckin testing
            let mut moving_state = MovingState::with_push(new_pos, push_info);
            moving_state.remaining_us = (0.8 * (moving_state.remaining_us as f32)).round() as u32;

            new.move_state = MoveState::Moving(moving_state);
            new
        }
        else {
            self.clone()
        }
    }

    fn try_move(&self, input : Input, state : &GameState, pushes : &mut Vec<Push>, map : &Map) -> Option<MovingState> {
        let mut push_info = PushInfo::default();
        let new_pos = map.try_apply_input(state.time_us, &state.rules_state, &self.pos, input)?;

        for (id, other_player) in state.player_states.iter() {
            // Skip over self and players not in game.
            if (id == self.id || state.rules_state.fst.get_player_alive(id) != AliveState::Alive)
            {
                continue;
            }

            // Note ? operator here
            // If we fail to push a player we cant move into a given spot
            // So we bail out unable to move.
            let possible_push_info = self.try_move_player(input, new_pos, other_player, state, pushes, map)?;

            if (possible_push_info.pushing.is_some()) {
                // Because only one player per spot
                // We can only push one person
                push_info = possible_push_info;
                break;
            }
        }

        Some(MovingState::with_push(new_pos, push_info))
    }

    fn try_move_player_initial(
        &self,
        dir : Input,
        candidate_pos : Pos,
        other : &PlayerState,
        state: &GameState,
        map : &Map) -> TryMovePlayerState
    {
        if other.pos == candidate_pos {
            // Try to move into some other player
            if let MoveState::Moving(moving_state) = &other.move_state {
                // Trying to switch positions, not allowed
                if self.pos == moving_state.target {
                    return TryMovePlayerState::Blocked;
                }
                // Other play is moving away, if they have gone far enough, let them go
                if (moving_state.remaining_us as f32) < MOVE_DUR as f32 * 0.5 {
                    return TryMovePlayerState::MoveUnimpeded;
                }
            }

            if (state.can_push(other.id, dir, state.time_us, &state.rules_state, map)) {
                TryMovePlayerState::MoveWithPush
            }
            else {
                TryMovePlayerState::Blocked
            }
        }
        else {
            match &other.move_state {
                MoveState::Moving(moving_state) => {
                    // Moving to a different position than us, don't care as we know they arent moving from where we are headed to.
                    if (moving_state.target != candidate_pos)
                    {
                        TryMovePlayerState::MoveUnimpeded
                    }
                    else
                    {
                        // Moving to same position

                        /*
                        // Other is *close* to being in the position
                        // Try and push them
                        if (moving_state.remaining_us as f32) < MOVE_DUR as f32 * 0.5 {
                            // Try and push!
                            if (state.can_push(other.id, dir, state.time_us, &state.rules_state, map)) {
                                TryMovePlayerState::MoveWithPush
                            }
                            else {
                                TryMovePlayerState::Blocked
                            }
                        }
                        else {
                            TryMovePlayerState::MoveUnimpeded
                        }*/

                        // Try and push!
                        if (state.can_push(other.id, dir, state.time_us, &state.rules_state, map)) {
                            TryMovePlayerState::MoveWithPush
                        }
                        else {
                            TryMovePlayerState::Blocked
                        }
                    }
                },
                _ => TryMovePlayerState::MoveUnimpeded
            }
        }
    }

    fn try_move_player(
        &self,
        dir : Input,
        candidate_pos : Pos,
        other : &PlayerState,
        state: &GameState,
        pushes : &mut Vec<Push>,
        map : &Map) -> Option<PushInfo>
    {
        match self.try_move_player_initial(dir, candidate_pos, other, state, map)
        {
            TryMovePlayerState::Blocked => None,
            TryMovePlayerState::MoveUnimpeded => Some(PushInfo::default()),
            TryMovePlayerState::MoveWithPush => {
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

    pub fn reset_to_pos(&mut self, pos : Pos) {
        self.pos = pos;
        self.move_state = MoveState::Stationary;
        self.move_cooldown = MOVE_COOLDOWN_MAX;
    }
}

// Simple representation to convert to json
#[derive(Debug, Default, Serialize)]
pub struct PlayerStatePublic
{
    pub id : u8,
    pub x : f64,
    pub y : i32,

    pub moving : bool,
    pub t_x : f64,
    pub t_y : i32,
    pub remaining_move_dur : u32, 
    pub move_cooldown: u32,

    pub pushing : i32,
    pub pushed_by : i32,
}

impl PlayerState {
    pub fn to_public(&self, _round_id : u8, time_us : u32, map : &Map) -> PlayerStatePublic {
        let mut player_state_public = PlayerStatePublic::default();

        player_state_public.id = self.id.0;

        let PreciseCoords{x, y} = map.realise_pos(time_us, &self.pos);
        player_state_public.x = x;
        player_state_public.y = y;
        
        if let MoveState::Moving(ms) = &self.move_state {
            let PreciseCoords{x: t_x, y: t_y} = map.realise_pos(time_us, &ms.target);
            player_state_public.moving = true;
            player_state_public.t_x = t_x;
            player_state_public.t_y = t_y;
            player_state_public.remaining_move_dur = ms.remaining_us;
            player_state_public.pushing = ms.push_info.pushing.map(|x| x.0 as i32).unwrap_or(-1);
            player_state_public.pushed_by = ms.push_info.pushed_by.map(|x| x.0 as i32).unwrap_or(-1);
        }

        player_state_public
    }
}