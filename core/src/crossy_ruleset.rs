use serde::{Deserialize, Serialize};

use crate::game::{PlayerId, Pos, CoordPos};
use crate::player::PlayerState;
use crate::player_id_map::PlayerIdMap;
use crate::map::{Map, RowType};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameConfig {
    pub required_win_count : u8,
    pub minimum_players : u8,
    pub bypass_lobby : bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            required_win_count : 3,
            minimum_players : 2,
            bypass_lobby: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LobbyState {
    pub time_with_all_players_in_ready_zone : u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AliveState
{
    NotInGame,
    Alive,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarmupState {
    pub remaining_us : u32,
    pub time_full_us : u32,
    // If someone joins during the warmup don't throw them in until the next round
    pub alive_states : PlayerIdMap<AliveState>,
    pub win_counts : PlayerIdMap<u8>,
    pub round_id : u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundState {
    pub screen_y : i32,
    pub alive_states : PlayerIdMap<AliveState>,
    pub win_counts : PlayerIdMap<u8>,
    pub round_id : u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownState {
    pub remaining_us : u32,

    #[serde(flatten)]
    pub round_state : RoundState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndWinnerState {
    pub winner_id : PlayerId,
    pub remaining_us : u32,
}

impl EndWinnerState {
    fn new(winner_id : PlayerId) -> Self {
        Self {
            winner_id,
            remaining_us: WINNER_TIME_US,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndAllLeftState {
    pub remaining_us : u32,
}

impl Default for EndAllLeftState {
    fn default() -> Self {
        Self {
            remaining_us: WINNER_TIME_US,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum CrossyRulesetFST
{
    Lobby{time_with_all_players_in_ready_zone : u32, raft_pos: f32,},
    RoundWarmup(WarmupState),
    Round(RoundState),
    RoundCooldown(CooldownState),
    EndWinner(EndWinnerState),
    EndAllLeft(EndAllLeftState),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RulesState
{
    pub game_id : u32,
    pub fst : CrossyRulesetFST,
    pub config : GameConfig,
}

impl RulesState {
    pub fn new(config : GameConfig) -> Self {
        Self {
            game_id: 0,
            fst: CrossyRulesetFST::start(),
            config,
        }
    }
}

pub const INTRO_COUNTDOWN_TIME_US : u32 = 6 * 1_000_000;
const COUNTDOWN_TIME_US : u32 = 3 * 1_000_000;
const COOLDOWN_TIME_US : u32 = 4 * 1_000_000;
pub const WINNER_TIME_US : u32 = 3 * 1_000_000;
const RIVER_SPAWN_Y_OFFSET : i32 = 12;

use CrossyRulesetFST::*;

impl RulesState
{
    pub fn tick(&self, dt : u32, time_us : u32, player_states : &mut PlayerIdMap<PlayerState>, map : &Map) -> Self {
        let new_fst = self.fst.tick(dt, time_us, player_states, map, &self.config);

        if (self.fst.in_lobby() && !new_fst.in_lobby())
        {
            // Went from non-lobby back to lobby
            // Increment the game id

            Self {
                game_id: self.game_id + 1,
                fst: new_fst,
                config: self.config.clone(),
            }
        }
        else
        {
            Self {
                game_id: self.game_id,
                fst: new_fst,
                config: self.config.clone(),
            }
        }
    }
}

impl CrossyRulesetFST
{
    pub fn start() -> Self {
        Lobby{
            time_with_all_players_in_ready_zone: 0,
            raft_pos: 8.0,
        }
    }

    pub fn tick(&self, dt : u32, time_us : u32, player_states : &mut PlayerIdMap<PlayerState>, map : &Map, game_config : &GameConfig) -> Self {
        match self {
            Lobby{time_with_all_players_in_ready_zone, raft_pos} => {

                // "kill players" by removing them from the lobby
                let mut to_kill = Vec::new();
                for (id, state) in player_states.iter() {
                    if let Pos::Coord(coord_pos) = &state.pos {
                        let row = map.get_row(0, coord_pos.y);
                        if let RowType::LobbyRiver = row.row_type {
                            debug_log!("Killing {:?}",id);
                            to_kill.push(id);
                        }
                    }
                }

                for kill in to_kill {
                    player_states.remove(kill);
                }

                let bypass = game_config.bypass_lobby && player_states.count_populated() > 0;
                let enough_players = player_states.count_populated() >= game_config.minimum_players as usize;
                let all_in_ready_zone = player_states.iter().all(|(_, x)| player_in_lobby_ready_zone(x));
                //println!("states {:#?}", player_states);
                //println!("All in ready zone {}, enough players {}", all_in_ready_zone, enough_players);

                if bypass || (enough_players && all_in_ready_zone) {
                    //if bypass || (*time_with_all_players_in_ready_zone > 120) { 
                    if bypass || (*time_with_all_players_in_ready_zone > 40) { 
                        if (*raft_pos > 20.0) {
                            debug_log!("Starting Game! ...");
                            debug_log!("Player States {:?}", player_states);

                            // Initialize to all zero
                            let win_counts = PlayerIdMap::seed_from(player_states, 0);
                            let alive_states = PlayerIdMap::seed_from(player_states, AliveState::Alive);
                            reset_positions(player_states, ResetPositionTarget::RacePositions);

                            RoundWarmup(WarmupState {
                                win_counts,
                                alive_states,
                                remaining_us : INTRO_COUNTDOWN_TIME_US,
                                time_full_us: INTRO_COUNTDOWN_TIME_US,
                                round_id : 1,
                            })
                        }
                        else {
                            Lobby{
                                time_with_all_players_in_ready_zone: time_with_all_players_in_ready_zone + 1,
                                raft_pos: *raft_pos + 0.05,
                            }
                        }
                    }
                    else {
                        Lobby{
                            time_with_all_players_in_ready_zone: time_with_all_players_in_ready_zone + 1,
                            raft_pos: dan_lerp(*raft_pos, 8.0, 50.0),
                        }
                    }
                }
                else {
                    Lobby{
                        time_with_all_players_in_ready_zone: (*time_with_all_players_in_ready_zone as f32 * 0.8).round() as u32,
                        raft_pos: dan_lerp(*raft_pos, 8.0, 50.0),
                    }
                }
            },
            RoundWarmup(state) => {
                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundWarmup(WarmupState {
                            remaining_us,
                            time_full_us: state.time_full_us,
                            alive_states : state.alive_states.clone(),
                            win_counts : state.win_counts.clone(),
                            round_id : state.round_id,
                        })
                    }
                    _ => {
                        let alive_states = PlayerIdMap::seed_from(player_states, AliveState::Alive);
                        Round(RoundState {
                            screen_y : 0,
                            alive_states,
                            win_counts: state.win_counts.clone(),
                            round_id : state.round_id,
                        })
                    }
                }
            },
            Round(state) => {
                if (player_states.count_populated() < game_config.minimum_players as usize)
                {
                    // No longer enough players in the game, because people left.
                    return EndAllLeft(EndAllLeftState::default());
                }

                let mut new_state = state.clone();
                // New player joined?
                new_state.alive_states.seed_missing(player_states, AliveState::NotInGame);
                new_state.screen_y = update_screen_y(new_state.screen_y, player_states, &new_state.alive_states);

                kill_players(time_us, new_state.round_id, &mut new_state.alive_states, map, player_states, new_state.screen_y, self);

                let alive_player_count = new_state.alive_states.iter().filter(|(_, x)| **x == AliveState::Alive).count();

                // Update spawn times
                // Force evaluation up to screen top
                let spawn_to_y = new_state.screen_y - RIVER_SPAWN_Y_OFFSET;
                let _ = map.get_row(new_state.round_id, spawn_to_y);

                if (alive_player_count < game_config.minimum_players as usize) {
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
                new_state.round_state.alive_states.seed_missing(player_states, AliveState::NotInGame);
                new_state.round_state.screen_y = update_screen_y(new_state.round_state.screen_y, player_states, &new_state.round_state.alive_states);
                kill_players(time_us, state.round_state.round_id, &mut new_state.round_state.alive_states, map, player_states, new_state.round_state.screen_y, &self);

                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        RoundCooldown(CooldownState {
                            remaining_us,
                            round_state : new_state.round_state,
                        })
                    }
                    _ => {
                        // We know up to one person is alive here
                        let winner = new_state.round_state.alive_states.iter().filter(|(_, x)| **x == AliveState::Alive).map(|(id, _)| id).next();

                        let mut win_counts = new_state.round_state.win_counts.clone();

                        if let Some(winner_id) = winner {
                            let new_count = win_counts.get(winner_id).copied().unwrap_or(0) + 1;
                            debug_log!("Going to next round, winner player={:?} count={}", winner_id, new_count);
                            if (new_count >= game_config.required_win_count) {
                                debug_log!("Going to end state");
                                reset_positions(player_states, ResetPositionTarget::LobbyPositions);
                                return EndWinner(EndWinnerState::new(winner_id));
                            }
                            win_counts.set(winner_id, new_count);
                        }

                        // Take into account all players that have joined during the round
                        let alive_states = PlayerIdMap::seed_from(player_states, AliveState::Alive);
                        win_counts.seed_missing(player_states, 0);
                        println!("CALLING RESET_POSITIONS BEFORE {:#?}", player_states);
                        reset_positions(player_states, ResetPositionTarget::RacePositions);
                        println!("CALLING RESET_POSITIONS AFTER {:#?}", player_states);

                        RoundWarmup(WarmupState {
                            remaining_us : COUNTDOWN_TIME_US,
                            time_full_us: COUNTDOWN_TIME_US,
                            win_counts,
                            alive_states,
                            round_id : new_state.round_state.round_id + 1,
                        })
                    }
                }
            },
            EndWinner(state) => {
                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        EndWinner(EndWinnerState {
                            remaining_us,
                            winner_id : state.winner_id,
                        })
                    }
                    _ => {
                        // Reset to lobby
                        reset_positions(player_states, ResetPositionTarget::LobbyPositions);
                        Self::start()
                    }
                }
            },
            EndAllLeft(state) => {
                match state.remaining_us.checked_sub(dt) {
                    Some(remaining_us) => {
                        EndAllLeft(EndAllLeftState {
                            remaining_us,
                        })
                    }
                    _ => {
                        // Reset to lobby
                        reset_positions(player_states, ResetPositionTarget::LobbyPositions);
                        Self::start()
                    }
                }
            }
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

    pub fn get_player_alive(&self, player_id : PlayerId) -> AliveState {
        match self {
            Lobby{..} => AliveState::Alive,
            RoundWarmup(state) => {
                // Only players who joined before
                state.alive_states.get_copy(player_id).unwrap_or(AliveState::NotInGame)
            }
            Round(state) => {
                state.alive_states.get_copy(player_id).unwrap_or(AliveState::NotInGame)
            },
            RoundCooldown(state) => {
                state.round_state.alive_states.get_copy(player_id).unwrap_or(AliveState::NotInGame)
            },
            EndWinner(state) => {
                if player_id == state.winner_id
                {
                    AliveState::Alive
                }
                else
                {
                    AliveState::NotInGame
                }
            },
            EndAllLeft(_) => AliveState::Alive,
        }
    }

    pub fn winner_counts(&self) -> PlayerIdMap<u8> {
        match self {
            CrossyRulesetFST::Round(state) => {
                state.win_counts.clone()
            },
            CrossyRulesetFST::RoundCooldown(state) => {
                state.round_state.win_counts.clone()
            },
            CrossyRulesetFST::RoundWarmup(state) => {
                state.win_counts.clone()
            },
            _ => {
                Default::default()
            }
        }
    }
     
    pub fn get_screen_y(&self) -> i32 {
        match self {
            Lobby{..} => 0,
            RoundWarmup(_) => 0,
            Round(state) => state.screen_y,
            RoundCooldown(state) => state.round_state.screen_y,
            EndWinner(_) => 0,
            EndAllLeft(_) => 0,
        }
    }

    pub fn same_variant(&self, other : &Self) -> bool {
        match (self, other) {
            (Lobby{..}, Lobby{..}) => true,
            (RoundWarmup(_), RoundWarmup(_)) => true,
            (Round(_), Round(_)) => true,
            (RoundCooldown(_), RoundCooldown(_)) => true,
            (EndWinner(_), EndWinner(_)) => true,
            (EndAllLeft(_), EndAllLeft(_)) => true,
            _ => false,
        }
    }

    pub fn in_lobby(&self) -> bool {
        match self {
            CrossyRulesetFST::Lobby{..} => true,
            _ => false,
        }
    }
}

enum ResetPositionTarget {
    LobbyPositions,
    RacePositions,
}

fn reset_positions(player_states : &mut PlayerIdMap<PlayerState>, target : ResetPositionTarget) {
    let player_count_for_offset = player_states.iter().map(|(id, _)| id.0 as i32).max().unwrap_or(0);

    for id in player_states.valid_ids() {
        let player_state = player_states.get_mut(id).unwrap();

        let x_off_from_count = (player_count_for_offset / 2);
        let x = player_state.id.0 as i32 + 9 - x_off_from_count;

        let y = match target
        {
            ResetPositionTarget::LobbyPositions => 11,
            ResetPositionTarget::RacePositions => 16,
        };

        player_state.reset_to_pos(Pos::Coord(CoordPos{x, y}));
    }
}

fn update_screen_y(mut screen_y : i32, player_states : &PlayerIdMap<PlayerState>, alive_states : &PlayerIdMap<AliveState>) -> i32 {
    const SCREEN_Y_BUFFER : i32 = 6;
    for (id, player) in player_states.iter() {
        if let Some(AliveState::Alive) = alive_states.get_copy(id) {
            let y = match &player.pos {
                Pos::Coord(pos) => pos.y,
                Pos::Lillipad(lilli) => lilli.y,
                _ => {
                    unreachable!()
                }
            };

            screen_y = screen_y.min(y - SCREEN_Y_BUFFER);
        }
    }

    screen_y
}

fn should_kill(time_us : u32, round_id : u8, map : &Map, player_state : &PlayerState, screen_y : i32, ruleset_fst: &CrossyRulesetFST) -> bool{
    // TODO also check position you are moving to
    //if let Stationary = player_state.move_state {
        match &player_state.pos {
            Pos::Coord(coord_pos) => {
                let CoordPos{x : _x, y} = *coord_pos;
                const SCREEN_KILL_BUFFER : i32 = 4;
                if y > screen_y + crate::SCREEN_SIZE + SCREEN_KILL_BUFFER {
                    debug_log!("Killing, off the end of the screen {:?} {:?}", player_state.id, player_state.pos);
                    return true;
                }

                let row = map.get_row(round_id, y);
                if let RowType::River(_) = row.row_type {
                    debug_log!("Killing, walked into river {:?} {:?}", player_state.id, player_state.pos);
                    return true;
                }

                let mut coord_pos_to_check_car_collision = *coord_pos;

                // When the player is moving between spots be more generous to player
                // Check for car collisions at the point they are moving to.
                if let crate::player::MoveState::Moving(moving_state) = &player_state.move_state {
                    if let Pos::Coord(moving_to_coord_pos) = moving_state.target {
                        coord_pos_to_check_car_collision = moving_to_coord_pos;
                    }
                }

                if map.collides_car(time_us, round_id, coord_pos_to_check_car_collision) {
                    return true;
                }

                false
            },
            Pos::Lillipad(lillypad_id) => {
                let precise_pos = map.get_lillipad_screen_x(time_us, lillypad_id, ruleset_fst);
                const KILL_OFF_MAP_THRESH : f64 = 2.5;
                precise_pos < -KILL_OFF_MAP_THRESH || precise_pos > (160.0 / 8.0 + KILL_OFF_MAP_THRESH)
            },
            _ => {
                unreachable!()
            },
        }
    //}
}

fn kill_players(time_us : u32, round_id : u8, alive_states : &mut PlayerIdMap<AliveState>, map : &Map, player_states : &mut PlayerIdMap<PlayerState>, screen_y : i32, ruleset_fst: &CrossyRulesetFST) {
    for id in player_states.valid_ids() {
        let alive = alive_states.get_copy(id).unwrap_or(AliveState::NotInGame);
        if (alive != AliveState::Alive) {
            continue;
        }

        if should_kill(time_us, round_id, map, player_states.get(id).unwrap(), screen_y, ruleset_fst) {
            alive_states.set(id, AliveState::Dead);
        }
    }
}

pub const LOBBY_READ_ZONE_X_MIN : i32 = 7;
pub const LOBBY_READ_ZONE_X_MAX : i32 = 12;
pub const LOBBY_READ_ZONE_Y_MIN : i32 = 12;
pub const LOBBY_READ_ZONE_Y_MAX : i32 = 15;

pub fn player_in_lobby_ready_zone(player : &PlayerState) -> bool {
    /*
    if let Pos::Coord(CoordPos{x, y}) = player.pos {
        x >= LOBBY_READ_ZONE_X_MIN && x <= LOBBY_READ_ZONE_X_MAX 
        &&
        y >= LOBBY_READ_ZONE_Y_MIN && y <= LOBBY_READ_ZONE_Y_MAX
    }
    else
    {
        false
    }
    */

    // In raft
    if let Pos::Lillipad(_lilly) = player.pos {
        true
    }
    else {
        false
    }
}

// @Dedup
fn dan_lerp(x0 : f32, x : f32, k : f32) -> f32 {
    (x0 * (k-1.0) + x) / k
}