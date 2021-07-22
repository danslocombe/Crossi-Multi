use serde::{Deserialize, Serialize};

use crate::game::PlayerId;
use crate::player_id_map::PlayerIdMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LobbyState {
    pub ready_states : PlayerIdMap<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarmupState {
    pub remaining_us : u32,
    pub win_counts : PlayerIdMap<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundState {
    pub screen_y : u32,
    pub alive_players : PlayerIdMap<bool>,
    pub win_counts : PlayerIdMap<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownState {
    pub remaining_us : u32,
    pub alive_players : PlayerIdMap<bool>,
    pub win_counts : PlayerIdMap<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndState {
    pub winner_id : PlayerId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrossyRulesetFST
{
    Lobby(LobbyState),
    RoundWarmup(WarmupState),
    Round(RoundState),
    RoundCooldown(CooldownState),
    End(EndState),
}

const MIN_PLAYERS : usize = 2;
const COUNTDOWN_TIME_US : u32 = 3 * 1_000_000;
const COOLDOWN_TIME_US : u32 = 4 * 1_000_000;
const REQUIRED_WIN_COUNT : u8 = 3;

use CrossyRulesetFST::*;

impl CrossyRulesetFST
{
    pub fn start() -> Self {
        Lobby(LobbyState {
            ready_states: PlayerIdMap::new(),
        })
    }

    pub fn tick(&self, dt : u32, player_states : &PlayerIdMap<crate::game::PlayerState>) -> Self {
        match self {
            Lobby(state) => {
                let mut new_lobby = state.clone();

                // Ensure all players have an entry in ready_states.
                new_lobby.ready_states.seed_missing(player_states, false);

                let enough_players = new_lobby.ready_states.count_populated() > MIN_PLAYERS;
                let all_ready = new_lobby.ready_states.iter().all(|(_, x)| *x);

                if (enough_players && all_ready) {
                    // Initialize to all zero
                    let win_counts = PlayerIdMap::seed_from(player_states, 0);
                    RoundWarmup(WarmupState {
                        win_counts,
                        remaining_us : COUNTDOWN_TIME_US,
                    })
                }
                else {
                    Lobby(new_lobby)
                }
            },
            RoundWarmup(state) => {
                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundWarmup(WarmupState {
                            remaining_us,
                            win_counts : state.win_counts.clone(),
                        })
                    }
                    _ => {
                        let alive_players = PlayerIdMap::seed_from(player_states, true);
                        Round(RoundState {
                            screen_y : 0,
                            alive_players,
                            win_counts: state.win_counts.clone(),
                        })
                    }
                }
            },
            Round(state) => {
                let mut new_round = state.clone();
                // New player joined?
                new_round.alive_players.seed_missing(player_states, false);

                // TODO update screen_y
                // TODO kill players

                let alive_player_count = new_round.alive_players.iter().filter(|(_, x)| **x).count();

                if (alive_player_count <= 1) {
                    RoundCooldown(CooldownState {
                        remaining_us : COOLDOWN_TIME_US,
                        alive_players : new_round.alive_players.clone(),
                        win_counts : new_round.win_counts.clone(),
                    })
                }
                else {
                    Round(new_round)
                }
            },
            RoundCooldown(state) => {

                // TODO kill players

                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundCooldown(CooldownState {
                            remaining_us,
                            alive_players : state.alive_players.clone(),
                            win_counts : state.win_counts.clone(),
                        })
                    }
                    _ => {
                        // We know up to one person is alive here
                        let winner = state.alive_players.iter().filter(|(_, x)| **x).map(|(id, _)| id).next();

                        let mut win_counts = state.win_counts.clone();

                        if let Some(winner_id) = winner {
                            let new_count = win_counts.get(winner_id).map(|x| *x).unwrap_or(0) + 1;
                            if (new_count >= REQUIRED_WIN_COUNT) {
                                return End(EndState {
                                    winner_id,
                                })
                            }
                            win_counts.set(winner_id, new_count);
                        }

                        RoundWarmup(WarmupState {
                            remaining_us : COUNTDOWN_TIME_US,
                            win_counts,
                        })
                    }
                }
            }
                    
            _ => {todo!()},
        }
    }
}