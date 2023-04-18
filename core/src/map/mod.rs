use std::collections::{VecDeque};
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use froggy_rand::FroggyRand;

pub mod road;
pub mod river;
pub mod obstacle_row;
pub mod bushes;

use road::Road;
use river::{River};
use obstacle_row::{ObstaclePublic, ObstacleRowDescr};
use bushes::BushDescr;

use crate::crossy_ruleset::{RulesState, CrossyRulesetFST};
use crate::game::CoordPos;
use crate::SCREEN_SIZE;
use crate::{Pos, PreciseCoords, Input};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct RowId(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
   pub row_id : RowId,
   pub row_type : RowType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RowType {
  River(ObstacleRowDescr),
  Path(PathDescr),
  Road(ObstacleRowDescr),
  Bushes(BushDescr),
  StartingBarrier(),
  Stands(),
}

impl RowType {
    pub fn is_dangerous(&self) -> bool {
        match self {
            RowType::River(_) => true,
            RowType::Road(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiverDescr {
    seed : u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathDescr {
    wall_width : u32,
}

#[derive(Debug)]
struct MapRound {
    seed : u32,
    round_id : u8,
    gen_state_wall_width : i32,
    roads : Vec<(i32, Road)>,
    rivers : Vec<(i32, River)>,
    rows : VecDeque<Row>,
}

#[derive(Debug)]
pub struct MapInner {
    // todo better structure
    seed : u32,
    rounds : Vec<MapRound>,
}

#[derive(Clone, Debug)]
pub struct Map{
   seed : u32,
   inner : std::sync::Arc<std::sync::Mutex<MapInner>>,
}

impl Map {
    pub fn new<T : Hash>(seed_key : T) -> Self {
        let seed = FroggyRand::new(0).gen(seed_key) as u32;
        Self::exact_seed(seed)
    }

    pub fn exact_seed(seed : u32) -> Self {
        Self {
            seed,
            inner: std::sync::Arc::new(std::sync::Mutex::new(MapInner::new(seed))),
        }
    }

    pub fn get_seed(&self) -> u32 {
        self.seed
    }

    /*
    Premature optimisation, add back in if we need
    pub fn update_min_y(&mut self, min_y : i32) {
        if min_y > SCREEN_SIZE {
            panic!("Tried to generate from below the map");
        }
        let screen_bottom_row_id = RowId::from_y(min_y + SCREEN_SIZE);
        let mut guard = self.inner.lock().unwrap();
        guard.update_min_row_id(screen_bottom_row_id);
    }
    */

    pub fn get_row(&self, round : u8, y : i32) -> Row {
        let mut guard = self.inner.lock().unwrap();
        guard.get_mut(round).get_row(RowId::from_y(y))
    }

    pub fn get_cars(&self, round : u8, time_us : u32) -> Vec<ObstaclePublic> {
        let mut guard = self.inner.lock().unwrap();
        guard.get(round).get_cars(time_us)
    }

    pub fn get_lillipads(&self, round : u8, time_us : u32) -> Vec<ObstaclePublic> {
        let mut guard = self.inner.lock().unwrap();
        guard.get(round).get_lillipads(time_us)
    }

    pub fn collides_car(&self, time_us : u32, round : u8, pos : CoordPos) -> bool {
        let mut guard = self.inner.lock().unwrap();
        guard.get_mut(round).generate_to_y(RowId::from_y(pos.y));
        for (_y, road) in &guard.get(round).roads {
            if (road.collides_car(time_us, pos)) {
                return true;
            }
        }

        false
    }

    pub fn solid(&self, time_us : u32, rule_state : &RulesState, pos : CoordPos) -> bool {
        if pos.x < 0 || pos.x >= SCREEN_SIZE {
            return true;
        }

        if pos.y >= SCREEN_SIZE {
            return true;
        }

        let mut guard = self.inner.lock().unwrap();
        let round_id = rule_state.fst.get_round_id();
        let round = guard.get_mut(round_id);

        if (round.seed == 0 && pos.y < 0) {
            return true;
        }

        round.get_row(RowId::from_y(pos.y)).solid(time_us, rule_state, pos)
    }

    pub fn lillipad_at_pos(&self, round_id : u8, time_us : u32, pos : PreciseCoords) -> Option<crate::LillipadId> {
        let mut guard = self.inner.lock().unwrap();
        guard.get_mut(round_id).generate_to_y(RowId::from_y(pos.y));

        for (_i, (_y, river)) in guard.get(round_id).rivers.iter().enumerate() {
            if let Some(lid) = river.lillipad_at_pos(round_id, time_us, pos) {
                return Some(lid);
            }
        }

        None
    }

    pub fn get_lillipad_screen_x(&self, time_us : u32, lillipad : &crate::LillipadId) -> f64 {
        let mut guard = self.inner.lock().unwrap();
        let round_id = lillipad.round_id;

        // Do we need this gen to?
        guard.get_mut(round_id).generate_to_y(RowId::from_y(lillipad.y));

        for (y, river) in &guard.get(round_id).rivers {
            if (*y == lillipad.y) {
                return river.get_lillipad_screen_x(time_us, lillipad)
            }
        }

        panic!("Error, could not find a lillipad from lillipad_id {:?}", lillipad);
    }

    pub fn realise_pos(&self, time_us : u32, pos : &crate::Pos) -> PreciseCoords {
        match pos {
            crate::Pos::Coord(coord) => {
                coord.to_precise()
            },
            crate::Pos::Lillipad(lilli_id) => {
                let x = self.get_lillipad_screen_x(time_us, lilli_id);
                PreciseCoords{x, y: lilli_id.y}
            },
        }
    }

    pub fn try_apply_input(&self, time_us : u32, rule_state : &crate::crossy_ruleset::RulesState, pos : &crate::Pos, input : Input) -> Option<Pos> {
        let round_id = rule_state.fst.get_round_id();
        let pos = self.realise_pos(time_us, pos);
        let precise = pos.apply_input(input);

        if let Some(lillipad_id) = self.lillipad_at_pos(round_id, time_us, precise) {
            Some(Pos::Lillipad(lillipad_id))
        }
        else {
            let coord_pos = precise.to_coords();
            if (self.solid(time_us, &rule_state, coord_pos)) {
                return None;
            }
            else {
                Some(Pos::Coord(coord_pos))
            }
        }
    }
}

impl MapInner {
    fn new(seed : u32) -> Self {
        let mut rounds = Vec::with_capacity(8);

        // Always set first map seed to zero
        rounds.push(MapRound::new(0, 0));

        Self {
            seed,
            rounds,
        }
    }

    fn gen_to(&mut self, i : usize) {
        while i >= self.rounds.len() {
            let rid = self.rounds.len() as u8;
            self.rounds.push(MapRound::new(self.seed, rid));
        }
    }

    fn get(&mut self, round_id : u8) -> &MapRound {
        let i = round_id as usize;
        self.gen_to(i);
        &self.rounds[round_id as usize]
    }

    fn get_mut(&mut self, round_id : u8) -> &mut MapRound {
        let i = round_id as usize;
        self.gen_to(i);
        &mut self.rounds[round_id as usize]
    }
}

impl MapRound {
    fn new(seed : u32, round_id : u8) -> Self {
        let mut rows = VecDeque::with_capacity(64);
        for i in 0..12 {
            rows.push_front(Row {
                row_id : RowId(i),
                row_type : RowType::Path(PathDescr {
                    wall_width : 0,
                }),
            });
        }

        let mut round = Self {
            seed,
            round_id,
            gen_state_wall_width : 0,
            roads : Vec::with_capacity(24),
            rivers : Vec::with_capacity(24),
            rows,
        };

        round.initial_generate();

        round
    }

    /*
    fn update_min_row_id(&mut self, row_id : RowId) {
        while let Some(row) = self.rows.back() {
            if row.row_id.0 < row_id.0 {
                self.rows.pop_back();
            }
            else {
                return;
            }
        }
    }
    */

    fn get_row(&mut self, row_id : RowId) -> Row {
        let need_to_generate = self.rows.front().map(|row| row_id.0 > row.row_id.0).unwrap_or(true);

        if need_to_generate {
            self.generate_to_y(row_id);
        }

        self.get_row_unchecked(row_id)
    }

    fn get_row_unchecked(&mut self, row_id : RowId) -> Row {
        let head_row_id = self.rows.front().unwrap().row_id;
        let diff = head_row_id.0 - row_id.0;
        self.rows[diff as usize].clone()
    }

    fn initial_generate(&mut self) {
        const STANDS_HEIGHT : u32 = 8;
        for i in 0..8 {
            self.rows.push_front(Row {
                row_id : RowId(i),
                row_type : RowType::Stands(),
            });
        }

        self.rows.push_front(Row {
            row_id : RowId(STANDS_HEIGHT),
            row_type : RowType::StartingBarrier(),
        })
    }

    fn generate_to_y(&mut self, row_id_target : RowId) {
        while self.rows.front().map(|row| row_id_target.0 > row.row_id.0).unwrap_or(true) {
            let row_id = RowId(self.rows.front().map(|row| row.row_id.0 + 1).unwrap_or(0));
            let rng = FroggyRand::from_hash((self.seed, self.round_id, row_id));

            verbose_log!("Generating at {:?}, y={} | {:?}", row_id, row_id.to_y(), &rng);

            // Seed 0 is reserved for lobbies
            // We shouldnt generate any roads / rivers
            if (self.seed != 0 && rng.gen_unit("gen_feature") < 0.25) {
                verbose_log!("Generating obtacle row at y={}", row_id.to_y());

                if (rng.gen_unit("feature_type") < 0.5) {
                    verbose_log!("Generating road");

                    let lanes = *rng.choose("road_lanes", &[1, 2, 3, 4, 5]);
                    let initial_direction = *rng.choose("road_initial_direction", &[true, false]);

                    verbose_log!("lanes {}, initial_direction {}", lanes, initial_direction);

                    for i in 0..lanes {
                        let rid = RowId(row_id.0 + i);
                        let y = rid.to_y();
                        verbose_log!("Adding road at {}", y);
                        let road = Road::new(self.seed, self.round_id, y, initial_direction);
                        verbose_log!("Road {:?}", &road);
                        self.roads.push((y, road));
                        self.rows.push_front(Row {
                            row_id: rid,
                            row_type: RowType::Road(ObstacleRowDescr {
                                seed: self.seed,
                                inverted: initial_direction,
                        })});
                    }
                    for i in 0..lanes {
                        let rid = RowId(row_id.0 + lanes + i);
                        let y = rid.to_y();
                        verbose_log!("Adding road inverted at {}", y);
                        let road = Road::new(self.seed, self.round_id, y, !initial_direction);
                        verbose_log!("Road {:?}", &road);
                        self.roads.push((y, road));
                        self.rows.push_front(Row {
                            row_id: rid,
                            row_type: RowType::Road(ObstacleRowDescr {
                                seed: self.seed,
                                inverted: !initial_direction,
                        })});
                    }
                }
                else {
                    verbose_log!("Generating river");

                    let lanes = *rng.choose("river_lanes", &[2, 2, 3, 4]);
                    let river_direction = *rng.choose("river_direction", &[true, false]);

                    verbose_log!("lanes {}, river_direction {}", lanes, river_direction);

                    for i in 0..lanes {
                        let rid = RowId(row_id.0 + i);
                        let y = rid.to_y();

                        verbose_log!("Adding river at {}", y);
                        let river = River::new(self.seed, self.round_id, y, river_direction);
                        verbose_log!("River {:?}", &river);
                        self.rivers.push((y, river));
                        self.rows.push_front(Row {
                            row_id: rid,
                            row_type: RowType::River(ObstacleRowDescr {
                                seed: self.seed,
                                inverted: river_direction,
                        })});
                    }
                }
            }
            else {
                const WALL_WIDTH_MAX : i32 = 6;
                const WALL_WIDTH_MIN : i32 = 1;
                let new_wall_width = self.gen_state_wall_width + rng.choose("wall_width", &[-1, -1, 0, 0, 0, 0, 1, 1, 1]);
                self.gen_state_wall_width = new_wall_width.min(WALL_WIDTH_MAX).max(WALL_WIDTH_MIN);

                let path_descr = PathDescr {
                    wall_width : self.gen_state_wall_width as u32,
                };

                if (self.seed != 0 && rng.gen_unit("gen_bushes") < 0.25)
                {
                    let seed = rng.gen("bush_seed") as u32;
                    self.rows.push_front(Row {
                        row_id,
                        row_type: RowType::Bushes(BushDescr { 
                            path_descr,
                            seed,
                            y : row_id.to_y(),
                        }),
                    });
                }
                else
                {
                    self.rows.push_front(Row {
                        row_id,
                        row_type: RowType::Path(PathDescr {
                            wall_width : self.gen_state_wall_width as u32,
                        }),
                    });
                }
            }
        }
    }

    fn get_cars(&self, time_us : u32) -> Vec<ObstaclePublic> {
        let mut cars = Vec::with_capacity(8);
        for (_y, road) in &self.roads {
            // TODO y offset
            cars.extend(road.get_cars_public(time_us));
        }
        cars
    }

    fn get_lillipads(&self, time_us : u32) -> Vec<ObstaclePublic> {
        let mut lillipads = Vec::with_capacity(32);
        for (i, (_y, river)) in self.rivers.iter().enumerate() {
            // TODO y offset
            lillipads.extend(river.get_lillipads_public(time_us));
        }
        lillipads
    }
}

impl RowId {
    // Hackkyyyyy because we hardcode screen size.
    pub fn from_y(y : i32) -> Self {
        Self((SCREEN_SIZE - y) as u32)
    }

    pub fn to_y(self) -> i32 {
        (SCREEN_SIZE - self.0 as i32)
    }
}


fn outside_walls(x : i32, wall_width : i32) -> bool {
    x <= wall_width as i32 || x >= (SCREEN_SIZE - 1 - wall_width as i32)
}

impl Row {
    pub fn solid(&self, _time_us : u32, rule_state : &RulesState, pos : CoordPos) -> bool {
        //debug_log!("Checking {:?} solid, assert value self.row_id.to_y() = {}", pos, self.row_id.to_y());
        assert!(self.row_id.to_y() == pos.y);
        let x = pos.x;

        if let CrossyRulesetFST::Lobby(_) = rule_state.fst {
            // Nothing is solid
            return false;
        }

        const STANDS_WIDTH : i32 = 6;
        match &self.row_type {
            RowType::Path(s) => {
                outside_walls(x, s.wall_width as i32)
            },
            RowType::Bushes(s) => {
                outside_walls(x, s.path_descr.wall_width as i32)
            }
            RowType::StartingBarrier() => {
                if let CrossyRulesetFST::RoundWarmup(_) = rule_state.fst {
                    // Whole row solid while barrier is up
                    true
                }
                else{
                    outside_walls(x, STANDS_WIDTH)
                }
            }
            RowType::Stands() => {
                outside_walls(x, STANDS_WIDTH)
            },
            _ => false,
        }
    }
}