use froggy_rand::FroggyRand;
use serde::{Deserialize, Serialize};
use crate::SCREEN_SIZE;

use super::PathDescr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BushDescr {
    pub path_descr : PathDescr,
    pub seed : u32,
    pub y : i32,
}

impl BushDescr {
    pub fn hydrate(&self) -> HydratedBushRow {
        let mut bushes = Vec::new();
        let rng = FroggyRand::from_hash((self.y, self.seed));

        for x in 0..SCREEN_SIZE {
            if (x <= self.path_descr.wall_width as i32 || x >= (SCREEN_SIZE - self.path_descr.wall_width as i32 - 1)) {
                continue;
            }

            //if (rng.gen_unit(x) < 0.45) {
                bushes.push(x);
            //}
        }

        HydratedBushRow {
            bushes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydratedBushRow {
    bushes : Vec<i32>,
}