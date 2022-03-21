use serde::{Deserialize, Serialize};

use crate::game::{PlayerId, Pos, CoordPos};
use crate::player::PlayerState;
use crate::player_id_map::PlayerIdMap;
use crate::map::{Map, RowType};
use crate::map::river::RiverSpawnTimes;

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
    pub round_id : u8,
    pub river_spawn_times : RiverSpawnTimes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundState {
    pub screen_y : i32,
    pub alive_players : PlayerIdMap<bool>,
    pub win_counts : PlayerIdMap<u8>,
    pub round_id : u8,
    pub river_spawn_times : RiverSpawnTimes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownState {
    pub remaining_us : u32,
    pub round_state : RoundState,
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
const REQUIRED_WIN_COUNT : u8 = 25;
const RIVER_SPAWN_Y_OFFSET : i32 = 4;

use CrossyRulesetFST::*;

impl CrossyRulesetFST
{
    pub fn start() -> Self {
        Lobby(LobbyState {
            ready_states: PlayerIdMap::new(),
        })
    }

    pub fn tick(&self, dt : u32, time_us : u32, player_states : &mut PlayerIdMap<PlayerState>, map : &Map) -> Self {
        match self {
            Lobby(state) => {
                let mut new_lobby = state.clone();

                // Ensure all players have an entry in ready_states.
                new_lobby.ready_states.seed_missing(player_states, false);

                let enough_players = new_lobby.ready_states.count_populated() >= MIN_PLAYERS;
                let all_ready = new_lobby.ready_states.iter().all(|(_, x)| *x);

                if (enough_players && all_ready) {

                    debug_log!("Starting Game! ...");

                    // Initialize to all zero
                    let win_counts = PlayerIdMap::seed_from(player_states, 0);
                    let in_game = PlayerIdMap::seed_from(player_states, true);
                    reset_positions(player_states);

                    RoundWarmup(WarmupState {
                        win_counts,
                        in_game,
                        remaining_us : COUNTDOWN_TIME_US,
                        round_id : 1,
                        river_spawn_times : Default::default(),
                    })
                }
                else {
                    Lobby(new_lobby)
                }
            },
            RoundWarmup(state) => {
                // TODO only really need to do this once
                let screen_y = -RIVER_SPAWN_Y_OFFSET;
                let _ = map.get_row(state.round_id, screen_y);
                let river_spawn_times = map.update_river_spawn_times(&state.river_spawn_times, state.round_id, time_us, screen_y);

                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundWarmup(WarmupState {
                            remaining_us,
                            in_game : state.in_game.clone(),
                            win_counts : state.win_counts.clone(),
                            round_id : state.round_id,
                            river_spawn_times,
                        })
                    }
                    _ => {
                        let alive_players = PlayerIdMap::seed_from(player_states, true);
                        Round(RoundState {
                            screen_y : 0,
                            alive_players,
                            win_counts: state.win_counts.clone(),
                            round_id : state.round_id,
                            river_spawn_times,
                        })
                    }
                }
            },
            Round(state) => {
                let mut new_state = state.clone();
                // New player joined?
                new_state.alive_players.seed_missing(player_states, false);
                new_state.screen_y = update_screen_y(new_state.screen_y, player_states, &new_state.alive_players);

                kill_players(time_us, new_state.round_id, &mut new_state.alive_players, map, player_states, new_state.screen_y - RIVER_SPAWN_Y_OFFSET);

                let alive_player_count = new_state.alive_players.iter().filter(|(_, x)| **x).count();

                // Update spawn times
                // Force evaluation up to screen top
                let _ = map.get_row(new_state.round_id, new_state.screen_y - RIVER_SPAWN_Y_OFFSET);
                new_state.river_spawn_times = map.update_river_spawn_times(&state.river_spawn_times, new_state.round_id, time_us, new_state.screen_y);

                if (alive_player_count <= 1) {
                    RoundCooldown(CooldownState {
                        remaining_us : COOLDOWN_TIME_US,
                        round_state : new_state,
                    })
                }
                else {
                    Round(new_state)
                }
            },
            RoundCooldown(state) => {
                let mut new_state = state.clone();
                new_state.round_state.alive_players.seed_missing(player_states, false);
                new_state.round_state.screen_y = update_screen_y(new_state.round_state.screen_y, player_states, &new_state.round_state.alive_players);
                kill_players(time_us, state.round_state.round_id, &mut new_state.round_state.alive_players, map, player_states, new_state.round_state.screen_y);

                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundCooldown(CooldownState {
                            remaining_us,
                            round_state : new_state.round_state,
                        })
                    }
                    _ => {
                        // We know up to one person is alive here
                        let winner = new_state.round_state.alive_players.iter().filter(|(_, x)| **x).map(|(id, _)| id).next();

                        let mut win_counts = new_state.round_state.win_counts.clone();

                        if let Some(winner_id) = winner {
                            let new_count = win_counts.get(winner_id).copied().unwrap_or(0) + 1;
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
                            round_id : new_state.round_state.round_id + 1,
                            river_spawn_times: Default::default(),
                        })
                    }
                }
            }
                    
            _ => {todo!("Unknown state type {:?}", self)},
        }
    }

    pub fn get_round_id(&self) -> u8 {
        match self {
            RoundWarmup(x) => x.round_id,
            Round(x) => x.round_id,
            RoundCooldown(x) => x.round_state.round_id,
            _ => 0,
        }
    }

    pub fn get_river_spawn_times(&self) -> &RiverSpawnTimes {
        match self {
            RoundWarmup(r) => &r.river_spawn_times,
            Round(r) => &r.river_spawn_times,
            RoundCooldown(r) => &r.round_state.river_spawn_times,
            _ => &crate::map::river::EMPTY_RIVER_SPAWN_TIMES,
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
                state.round_state.alive_players.get_copy(player_id).unwrap_or(false)
            },
            End(state) => {
                player_id == state.winner_id
            }
        }
    }
     
    pub fn get_screen_y(&self) -> i32 {
        match self {
            Lobby(_) => 0,
            RoundWarmup(_) => 0,
            Round(state) => state.screen_y,
            RoundCooldown(state) => state.round_state.screen_y,
            End(_) => 0,
        }
    }

    pub fn same_variant(&self, other : &Self) -> bool {
        match (self, other) {
            (Lobby(_), Lobby(_)) => true,
            (RoundWarmup(_), RoundWarmup(_)) => true,
            (Round(_), Round(_)) => true,
            (RoundCooldown(_), RoundCooldown(_)) => true,
            (End(_), End(_)) => true,
            _ => false,
        }
    }
}

fn reset_positions(player_states : &mut PlayerIdMap<PlayerState>) {
    let player_count_for_offset = player_states.iter().map(|(id, _)| id.0 as i32).max().unwrap_or(0);

    for id in player_states.valid_ids() {
        let player_state = player_states.get_mut(id).unwrap();

        let x_off_from_count = (player_count_for_offset / 2);
        let x = player_state.id.0 as i32 + 9 - x_off_from_count;
        let y = 17;
        player_state.reset_to_pos(Pos::Coord(CoordPos{x, y}));
    }
}

fn update_screen_y(mut screen_y : i32, player_states : &PlayerIdMap<PlayerState>, alive_players : &PlayerIdMap<bool>) -> i32 {
    const SCREEN_Y_BUFFER : i32 = 6;
    for (id, player) in player_states.iter() {
        if alive_players.get_copy(id).unwrap_or(false) {
            let y = match &player.pos {
                Pos::Coord(pos) => pos.y,
                Pos::Lillipad(lilli) => lilli.y,
            };

            screen_y = screen_y.min(y - SCREEN_Y_BUFFER);
        }
    }

    screen_y
}

fn should_kill(time_us : u32, round_id : u8, map : &Map, player_state : &PlayerState, screen_y : i32) -> bool{
    // TODO also check position you are moving to
    //if let Stationary = player_state.move_state {
        match &player_state.pos {
            Pos::Coord(coord_pos) => {
                let CoordPos{x : _x, y} = *coord_pos;
                const SCREEN_KILL_BUFFER : i32 = 4;
                if y > screen_y + crate::SCREEN_SIZE + SCREEN_KILL_BUFFER {
                    return true;
                }

                let row = map.get_row(round_id, y);
                if let RowType::River(_) = row.row_type {
                    return true;
                }

                if map.collides_car(time_us, round_id, *coord_pos) {
                    return true;
                }

                false
            },
            Pos::Lillipad(lillypad_id) => {
                let precise_pos = map.get_lillipad_screen_x(time_us, lillypad_id);
                const KILL_OFF_MAP_THRESH : f64 = 2.5;
                precise_pos < -KILL_OFF_MAP_THRESH || precise_pos > (160.0 / 8.0 + KILL_OFF_MAP_THRESH)
            }
        }
    //}
}

fn kill_players(time_us : u32, round_id : u8, alive_players : &mut PlayerIdMap<bool>, map : &Map, player_states : &PlayerIdMap<PlayerState>, screen_y : i32) {
    for (id, player_state) in player_states {
        let alive = alive_players.get_copy(id).unwrap_or(false);
        if (!alive) {
            continue;
        }

        if should_kill(time_us, round_id, map, player_state, screen_y) {
            alive_players.set(id, false);
        }
    }
}