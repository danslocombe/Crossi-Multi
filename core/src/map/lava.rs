use std::collections::VecDeque;

use froggy_rand::FroggyRand;
use serde::{Deserialize, Serialize};

use crate::{bitmap::BitMap, map::RowType};

use super::{PathDescr, Row, RowId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LavaDescr {
    pub path_descr : PathDescr,
    pub seed : u64,
    pub y : i32,
    pub lava: BitMap,
    pub blocks: BitMap,
}

pub fn try_gen_lava_section(rand: FroggyRand, row_id_0: RowId, rows: &mut VecDeque<Row>) -> bool {
    println!("Lava seed = {}", rand.get_seed());
    let height = *rand.choose("lava_len", &[3, 5, 5, 7, 7, 9]);

    for i in 0..height {
        let rid = RowId(row_id_0.0 + i as u32);
        let y = rid.to_y();
        //let blocks = map.inner[i as usize];

        rows.push_front(Row {
            row_id: rid,
            row_type: RowType::LavaRow(LavaDescr {
                path_descr: PathDescr {
                    // @TODO, do properly
                    wall_width: 4,
                },
                seed: rand.get_seed(),
                y,
                lava: Default::default(),
                blocks: Default::default(),
            }),
        });
    }

    true
}