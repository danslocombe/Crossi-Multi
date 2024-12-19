use froggy_rand::FroggyRand;
use serde::{Deserialize, Serialize};
use crate::SCREEN_SIZE;

use super::PathDescr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcyDescr {
    pub path_descr : PathDescr,
    pub seed : u32,
    pub y : i32,
}

impl IcyDescr {
    pub fn hydrate(&self) -> HydratedIcyRow {
        let mut ice = Vec::new();
        let mut blocks = Vec::new();
        let rng = FroggyRand::from_hash((self.y, self.seed));

        for x in 0..SCREEN_SIZE {
            if (x <= self.path_descr.wall_width as i32 || x >= (SCREEN_SIZE - self.path_descr.wall_width as i32 - 1)) {
                continue;
            }

            //if (rng.gen_unit((x, 1)) < 0.45) {
                //ice.push(x);
            //}
            if (rng.gen_unit((x, 1)) < 0.45) {
                blocks.push(x);
            }
        }

        HydratedIcyRow {
            ice,
            blocks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydratedIcyRow {
    pub ice : Vec<i32>,
    pub blocks : Vec<i32>,
}