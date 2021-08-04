use crate::rng::FroggyRng;
use crate::map::obstacle_row::*;

#[derive(Debug)]
pub struct River {
    row : ObstacleRow,
}

const LILLIPAD_WIDTH_TILES : f64 = 1.0;
const R_WIDTH_MIN : f64 = 0.32;
const R_WIDTH_MAX : f64 = 0.42;
//const TIME_SCALE : f64 = 12_000_000.0;
const TIME_SCALE : f64 = 18_000_000.0;

impl River {
    pub fn new(seed : u32, round : u8, y : i32, inverted : bool) -> Self {
        let rng = FroggyRng::from_hash((seed, round, y));

        let mut obstacles = Vec::with_capacity(16);
        let mut cur = 0.0;

        let length = 3 + (rng.gen_unit("lillipad_length") * 5.0) as u32;
        let r_width = rng.gen_range("r_width", R_WIDTH_MIN, R_WIDTH_MAX);

        let r = 2.0 * r_width;

        let min_spacing = r / crate::SCREEN_SIZE as f64;
        let max_spacing = r * 5. / crate::SCREEN_SIZE as f64;

        let lillipad_width_screen = r * LILLIPAD_WIDTH_TILES / crate::SCREEN_SIZE as f64;

        let squeeze_spacing = (1.0 + length as f64) / crate::SCREEN_SIZE as f64;

        while ({
            cur += rng.gen_range(("lillipad_spacing", obstacles.len()), min_spacing, max_spacing);
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
        }
    }

    pub fn get_lillipads_public(&self, time_us : u32) -> Vec<ObstaclePublic> {
        self.row.get_obstacles_public(time_us)
    }
}