use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

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
    // TODO lillipad spawning info
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathDescr {
    wall_width : u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadDescr {
    // TODO Car spawning info
}


#[derive(Debug)]
pub struct Map{
   seed : u32,
   rows : std::sync::Arc<std::sync::RwLock<VecDeque<Row>>>,
}

impl Map {
    pub fn new(seed : u32) -> Self {
        Self {
            seed,
            rows: std::sync::Arc::new(std::sync::RwLock::new(VecDeque::with_capacity(64))),
        }
    }

    pub fn get_seed(&self) -> u32 {
        self.seed
    }

    pub fn update_min_y(&mut self, min_y : i32) {
        let screen_bottom_row_id = RowId::from_y(min_y + SCREEN_SIZE);
        self.update_min_row_id(screen_bottom_row_id);
    }

    fn update_min_row_id(&mut self, row_id : RowId) {
        let mut row_lock = self.rows.write().unwrap();
        while let Some(row) = row_lock.back() {
            if row.row_id.0 < row_id.0 {
                row_lock.pop_back();
            }
            else {
                return;
            }
        }
    }

    pub fn get_row(&self, y : i32) -> Row {
        self.get_row_internal(RowId::from_y(y))
    }

    fn get_row_internal(&self, row_id : RowId) -> Row {
        let read_lock = self.rows.read().unwrap();
        let need_to_generate = read_lock.front().map(|row| row_id.0 > row.row_id.0).unwrap_or(true);
        drop(read_lock);

        if need_to_generate {
            self.generate_to_y(row_id);
        }

        self.get_row_unchecked(row_id)
    }

    fn get_row_unchecked(&self, row_id : RowId) -> Row {
        let read_lock = self.rows.read().unwrap();
        let head_row_id = read_lock.front().unwrap().row_id;
        let diff = head_row_id.0 - row_id.0;

        // Can I delete this?
        if (row_id.0 > head_row_id.0 || diff as usize > read_lock.len()) {
            println!("diff {} head {:?} y {:?} len {}\n inner {:?}" , diff, head_row_id, row_id, read_lock.len(), &*read_lock);
            unreachable!();
        }

        read_lock[diff as usize].clone()
    }

    fn generate_to_y(&self, row_id_target : RowId) {
        // Tmp dumb impl
        let mut write_lock = self.rows.write().unwrap();
        while write_lock.front().map(|row| row_id_target.0 > row.row_id.0).unwrap_or(true) {
            let row_id = RowId(write_lock.front().map(|row| row.row_id.0 + 1).unwrap_or(0));
            let row_type = if (row_id.0 > 6 && ((row_id.0 / 2) % 4) == 0) {
                println!("row_id {:?} y {} is river", row_id, row_id.to_y());
                RowType::River(RiverDescr{
                })
            }
            else {
                RowType::Path(PathDescr {
                    wall_width: 1,
                })
            };

            write_lock.push_front(Row{
                row_id,
                row_type,
            });
        }
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