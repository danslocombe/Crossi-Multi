use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

pub mod road;

use road::{RoadDescr, Road, CarPublic};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
pub struct GeneratorState {
    rand : rand_xorshift::XorShiftRng,
}

#[derive(Debug)]
pub struct MapInner {
    // todo better structure
    roads : Vec<(i32, Road)>,
    rows : VecDeque<Row>,
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

    pub fn update_min_y(&mut self, min_y : i32) {
        let screen_bottom_row_id = RowId::from_y(min_y + SCREEN_SIZE);
        let mut guard = self.inner.lock().unwrap();
        guard.update_min_row_id(screen_bottom_row_id);
    }

    pub fn get_row(&self, y : i32) -> Row {
        let mut guard = self.inner.lock().unwrap();
        guard.get_row(RowId::from_y(y))
    }

    pub fn get_cars(&self, time_us : u32) -> Vec<CarPublic> {
        let guard = self.inner.lock().unwrap();
        guard.get_cars(time_us)
    }

    pub fn collides_car(&self, time_us : u32, pos : crate::game::CoordPos) -> bool {
        let guard = self.inner.lock().unwrap();
        for (_y, road) in &guard.roads {
            if (road.collides_car(time_us, pos)) {
                return true;
            }
        }

        false
    }
}

impl MapInner {
    fn new(seed : u32) -> Self {
        MapInner {
            roads : Vec::with_capacity(24),
            rows : VecDeque::with_capacity(64),
        }
    }

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

    fn generate_to_y(&mut self, row_id_target : RowId) {
        // Tmp dumb impl
        while self.rows.front().map(|row| row_id_target.0 > row.row_id.0).unwrap_or(true) {
            let row_id = RowId(self.rows.front().map(|row| row.row_id.0 + 1).unwrap_or(0));

            let row_type = if (row_id.0 > 6) {
                let mod_val = (row_id.0 / 2) % 6;
                if mod_val == 1 {
                    RowType::River(RiverDescr{
                        seed : 0,
                    })
                }
                else if mod_val == 3 {
                    let seed = 0;
                    let y = row_id.to_y();
                    let inverted = row_id.0 % 2 == 0;
                    self.roads.push((y, Road::from_seed(seed, y, inverted)));

                    RowType::Road(RoadDescr {
                        seed,
                        inverted,
                    })
                }
                else {
                    RowType::Path(PathDescr {
                        wall_width: 1,
                    })
                }
            }
            else {
                RowType::Path(PathDescr {
                    wall_width: 1,
                })
            };

            self.rows.push_front(Row{
                row_id,
                row_type,
            });
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
const SCREEN_SIZE : i32 = 160 / 8;

impl RowId {
    pub fn from_y(y : i32) -> Self {
        Self((SCREEN_SIZE - y) as u32)
    }

    pub fn to_y(&self) -> i32 {
        (SCREEN_SIZE - self.0 as i32)
    }
}