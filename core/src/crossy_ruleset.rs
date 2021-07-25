use serde::{Deserialize, Serialize};

use crate::game::{PlayerId, PlayerState, Pos, CoordPos};
use crate::player_id_map::PlayerIdMap;
use crate::map::{Map, RowType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LobbyState {
    pub ready_states : PlayerIdMap<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarmupState {
    pub remaining_us : u32,
    // If someone joins during the warmup don't throw them in until the next round
    pub in_game : PlayerIdMap<bool>,
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

    pub fn tick(&self, dt : u32, player_states : &mut PlayerIdMap<PlayerState>, map : &Map) -> Self {
        match self {
            Lobby(state) => {
                let mut new_lobby = state.clone();

                // Ensure all players have an entry in ready_states.
                new_lobby.ready_states.seed_missing(player_states, false);

                let enough_players = new_lobby.ready_states.count_populated() >= MIN_PLAYERS;
                let all_ready = new_lobby.ready_states.iter().all(|(_, x)| *x);

                if (enough_players && all_ready) {
                    println!("Starting Game! ...");
                    // Initialize to all zero
                    let in_game = PlayerIdMap::seed_from(player_states, true);
                    let win_counts = PlayerIdMap::seed_from(player_states, 0);
                    reset_positions(player_states);
                    RoundWarmup(WarmupState {
                        win_counts,
                        in_game,
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
                            in_game : state.in_game.clone(),
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
                kill_players(&mut new_round.alive_players, map, player_states);

                // TODO update screen_y
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
                let mut alive_players = state.alive_players.clone();
                kill_players(&mut alive_players, map, player_states);

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

                        // Take into account all players that have joined during the round
                        let in_game = PlayerIdMap::seed_from(player_states, true);
                        win_counts.seed_missing(player_states, 0);
                        reset_positions(player_states);

                        RoundWarmup(WarmupState {
                            remaining_us : COUNTDOWN_TIME_US,
                            win_counts,
                            in_game,
                        })
                    }
                }
            }
                    
            _ => {todo!()},
        }
    }

    pub fn get_player_alive(&self, player_id : PlayerId) -> bool {
        match self {
            Lobby(_) => true,
            RoundWarmup(state) => {
                // Only players who joined before
                state.in_game.get_copy(player_id).unwrap_or(false)
            }
            Round(state) => {
                state.alive_players.get_copy(player_id).unwrap_or(false)
            },
            RoundCooldown(state) => {
                state.alive_players.get_copy(player_id).unwrap_or(false)
            },
            End(state) => {
                player_id == state.winner_id
            }
        }
    }
}

fn reset_positions(player_states : &mut PlayerIdMap<PlayerState>) {
    for id in player_states.valid_ids() {
        let player_state = player_states.get_mut(id).unwrap();
        let x = player_state.id.0 as i32 + 4;
        let y = 17;
        player_state.pos = Pos::Coord(CoordPos{x, y});
    }
}

fn kill_players(alive_players : &mut PlayerIdMap<bool>, map : &Map, player_states : &PlayerIdMap<PlayerState>) {
    for (id, player_state) in player_states {
        let alive = alive_players.get_copy(id).unwrap_or(false);
        if (!alive) {
            continue;
        }

        let mut kill = false;
        //if let Stationary = player_state.move_state {
            match player_state.pos {
                Pos::Coord(CoordPos{x : _x, y}) => {
                    let row = map.get_row(y);
                    if let RowType::River(_) = row.row_type {
                        kill = true;
                    }
                },
                _ => {},
            }
        //}


        if (kill) {
            alive_players.set(id, false);
        }
    }
}