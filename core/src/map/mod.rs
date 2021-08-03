use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

pub mod road;

use road::{RoadDescr, Road, CarPublic};
use crate::rng::FroggyRng;
use crate::crossy_ruleset::CrossyRulesetFST;
use crate::game::CoordPos;
use crate::SCREEN_SIZE;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct RowId(u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
   pub row_id : RowId,
   pub row_type : RowType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RowType {
  River(RiverDescr),
  Path(PathDescr),
  Road(RoadDescr),
  StartingBarrier(),
  Stands(),
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
    rows : VecDeque<Row>,
}

#[derive(Debug)]
pub struct MapInner {
    // todo better structure
    seed : u32,
    rounds : Vec<MapRound>,
}

#[derive(Debug)]
pub struct Map{
   seed : u32,
   inner : std::sync::Arc<std::sync::Mutex<MapInner>>,
}

impl Map {
    pub fn new(seed : u32) -> Self {
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

    pub fn get_cars(&self, round : u8, time_us : u32) -> Vec<CarPublic> {
        let mut guard = self.inner.lock().unwrap();
        guard.get(round).get_cars(time_us)
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

    pub fn solid(&self, time_us : u32, rule_state : &CrossyRulesetFST, pos : CoordPos) -> bool {
        let mut guard = self.inner.lock().unwrap();
        guard.get_mut(rule_state.get_round_id()).get_row(RowId::from_y(pos.y)).solid(time_us, rule_state, pos)
    }
}

impl MapInner {
    fn new(seed : u32) -> Self {
        Self {
            seed,
            rounds : Vec::with_capacity(8),
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
            //println!("{} {} {:?}", self.seed, self.round_id, row_id);
            let rng = FroggyRng::from_hash((self.seed, self.round_id, row_id));

            debug_log!("Generating at {:?}, y={} | {:?}", row_id, row_id.to_y(), &rng);

            if (rng.gen("gen_feature") % 5 == 0) {
                    debug_log!("Generating road... at y={}", row_id.to_y());
                //if (rng.gen("feature_type") % 2 == 0) {
                    let lanes = *rng.choose("road_lanes", &[1, 2, 3, 4, 5]);
                    let initial_direction = *rng.choose("initial_direction", &[true, false]);

                    debug_log!("lanes {}, initial_direction {}", lanes, initial_direction);

                    //println!("generating road at {:?}, y={}, lanes {}", row_id, row_id.to_y(), lanes);
                    for i in 0..lanes {
                        let rid = RowId(row_id.0 + i);
                        let y = rid.to_y();
                        debug_log!("Adding road at {}", y);
                        let road = Road::new(self.seed, self.round_id, y, initial_direction);
                        debug_log!("Road {:?}", &road);
                        self.roads.push((y, road));
                        self.rows.push_front(Row {
                            row_id: rid,
                            row_type: RowType::Road(RoadDescr {
                                seed: self.seed,
                                inverted: initial_direction,
                        })});
                    }
                    for i in 0..lanes {
                        let rid = RowId(row_id.0 + lanes + i);
                        let y = rid.to_y();
                        debug_log!("Adding road inverted at {}", y);
                        let road = Road::new(self.seed, self.round_id, y, !initial_direction);
                        debug_log!("Road {:?}", &road);
                        self.roads.push((y, road));
                        self.rows.push_front(Row {
                            row_id: rid,
                            row_type: RowType::Road(RoadDescr {
                                seed: self.seed,
                                inverted: !initial_direction,
                        })});
                    }
                //}
            }
            else {
                const WALL_WIDTH_MAX : i32 = 4;
                const WALL_WIDTH_MIN : i32 = 1;
                let new_wall_width = self.gen_state_wall_width + rng.choose("wall_width", &[-1, 0, 0, 1]);
                self.gen_state_wall_width = new_wall_width.min(WALL_WIDTH_MAX).max(WALL_WIDTH_MIN);

                self.rows.push_front(Row {
                    row_id,
                    row_type: RowType::Path(PathDescr {
                        wall_width : self.gen_state_wall_width as u32,
                    }),
                });
            }
        }
    }


    fn get_cars(&self, time_us : u32) -> Vec<CarPublic> {
        let mut cars = Vec::with_capacity(8);
        for (_y, road) in &self.roads {
            // TODO y offset
            cars.extend(road.get_cars_public(time_us));
        }
        cars
    }
}

// Hackkyyyyy

impl RowId {
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
    pub fn solid(&self, _time_us : u32, rule_state : &CrossyRulesetFST, pos : CoordPos) -> bool {
        assert!(self.row_id.to_y() == pos.y);
        let x = pos.x;

        if let CrossyRulesetFST::Lobby(_) = rule_state {
            // Nothing is solid
            return false;
        }

        const STANDS_WIDTH : i32 = 6;
        match &self.row_type {
            RowType::Path(s) => {
                outside_walls(x, s.wall_width as i32)
            },
            RowType::StartingBarrier() => {
                if let CrossyRulesetFST::RoundWarmup(_) = rule_state {
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