use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
   y : u32,
   row_type : RowType,
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
   rows : VecDeque<Row>,
}

impl Map {
    pub fn new(seed : u32) -> Self {
        Self {
            seed,
            rows: VecDeque::with_capacity(64),
        }
    }

    pub fn get_seed(&self) -> u32 {
        self.seed
    }

    pub fn update_min_y(&mut self, min_y : u32) {
        while let Some(row) = self.rows.back() {
            if row.y < min_y {
                self.rows.pop_back();
            }
            else {
                return;
            }
        }
    }

    pub fn get_row(&mut self, y : u32) -> &Row {
        let need_to_generate = self.rows.front().map(|row| y > row.y).unwrap_or(true);
        if need_to_generate {
            self.generate_to_y(y);
        }

        self.get_row_unchecked(y)
    }

    fn get_row_unchecked(&self, y : u32) -> &Row {
        let head_y = self.rows.front().unwrap().y;
        let diff = head_y - y;
        &self.rows[diff as usize]
    }

    fn generate_to_y(&mut self, y : u32) {
        // Tmp dumb impl
        while self.rows.front().map(|row| y > row.y).unwrap_or(true) {
            let row_type = if ((y / 6) % 4) == 0 {
                RowType::River(RiverDescr{
                })
            }
            else {
                RowType::Path(PathDescr {
                    wall_width: 1,
                })
            };

            self.rows.push_front(Row{
                y,
                row_type,
            });
        }
    }
}