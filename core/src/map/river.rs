use froggy_rand::FroggyRand;
use serde::{Deserialize, Serialize};

use crate::map::obstacle_row::*;
use crate::{CoordPos, LillipadId};

#[derive(Debug)]
pub struct River {
    //pub spawn_time : Option<u32>,
    row : ObstacleRow,
}

const LILLIPAD_WIDTH_TILES : f64 = 1.0;
const R_WIDTH_MIN : f64 = 0.22;
const R_WIDTH_MAX : f64 = 0.42;
//const TIME_SCALE : f64 = 12_000_000.0;
const TIME_SCALE : f64 = 18_000_000.0;

impl River {
    pub fn new(seed : u32, round : u8, y : i32, inverted : bool) -> Self {
        let rng = FroggyRand::from_hash((seed, round, y));

        let mut obstacles = Vec::with_capacity(16);
        let mut cur = 0.0;

        let length = 3 + (rng.gen_froggy("lillipad_length", 0., 5., 3)) as u32;
        let r_width = rng.gen_froggy("r_width", R_WIDTH_MIN, R_WIDTH_MAX, 4);

        let r = 2.0 * r_width;

        let min_spacing = r * 1.9 / crate::SCREEN_SIZE as f64;
        let max_spacing = r * 6.8 / crate::SCREEN_SIZE as f64;

        let lillipad_width_screen = r * LILLIPAD_WIDTH_TILES / crate::SCREEN_SIZE as f64;

        let squeeze_spacing = (1.0 + length as f64) / crate::SCREEN_SIZE as f64;

        while ({
            cur += rng.gen_froggy(("lillipad_spacing", obstacles.len()), min_spacing, max_spacing, 2);
            cur < 1.0 - squeeze_spacing
        })
        {
            for _ in 0..length {
                obstacles.push(Obstacle(cur));
                cur += lillipad_width_screen;
            }
        }

        River {
            row : ObstacleRow::new(y, inverted, TIME_SCALE, obstacles, r_width),
            //spawn_time: None,
        }
    }

    pub fn get_lillipads_public(&self, time_us : u32, spawn_time : Option<u32>) -> Vec<ObstaclePublic> {
        if let Some(spawn_time) = spawn_time
        {
            self.row.get_obstacles_public_filtered(time_us, spawn_time)
        }
        else
        {
            vec![]
        }
    }

    pub fn lillipad_at_pos(&self, round_id : u8, time_us : u32, pos : crate::PreciseCoords, spawn_time : Option<u32>) -> Option<LillipadId> {
        if (spawn_time.is_none())
        {
            return None;
        }

        let spawn_time = spawn_time.unwrap();
        if (pos.y != self.row.y) {
            return None;
        }

        let frog_centre = pos.x;

        let mut closest = None;
        let mut closest_dist = f64::MAX;

        for (id, lillipad) in self.row.get_obstacles_onscreen(time_us)
            .iter()
            .filter(|x| self.row.filter_object(x, time_us, spawn_time))
            .enumerate() {
            let realised = self.row.realise_obstacle(lillipad);
            let dist = (frog_centre - realised).abs();

            if (dist < closest_dist) {
                closest_dist = dist;
                closest = Some(id);
            }
        }

        const MARGIN : f64 = LILLIPAD_WIDTH_TILES / 1.9;
        //debug_log!("Closest {}", closest_dist);
        if (closest_dist < MARGIN) {
            if let Some(id) = closest {
                let lillipad_id = LillipadId {
                    y : pos.y,
                    id : id  as u8,
                    round_id,
                };

                //debug_log!("Lillipad at pos, {:?}, lillipad {:?}", pos, &lillipad_id);

                return Some(lillipad_id);
            }
        }

        None
    }

    pub fn get_lillipad_screen_x(&self, time_us : u32, lillipad_id : &LillipadId) -> f64 {
        let lillipad = self.row.get_obstacle(time_us, lillipad_id.id as usize);
        self.row.realise_obstacle(&lillipad)
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiverSpawnTimes
{
    spawn_times : Vec<u32>,
}

impl RiverSpawnTimes
{
    pub fn get(&self, i : usize) -> Option<u32>
    {
        if (i < self.spawn_times.len())
        {
            Some(self.spawn_times[i])
        }
        else
        {
            None
        }
    }

    pub fn set(&mut self, i : usize, val : u32)
    {
        assert_eq!(i, self.spawn_times.len());
        self.spawn_times.push(val);
    }
}

pub static EMPTY_RIVER_SPAWN_TIMES : RiverSpawnTimes = RiverSpawnTimes { spawn_times : vec![] };