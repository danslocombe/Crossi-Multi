use crate::game::CoordPos;
use crate::map::obstacle_row::*;
use crate::rng::FroggyRng;

#[derive(Debug)]
pub struct Road {
    row : ObstacleRow,
}

const CAR_WIDTH : f64 = 24.0 / 8.0;

const R_WIDTH_MIN : f64 = 0.15;
const R_WIDTH_MAX : f64 = 0.35;
const TIME_SCALE : f64 = 12_000_000.0;

const MIN_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 0.8;
const MAX_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 4.0;
const SQUEEZE_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 1.35;

const MIN_SPACING : f64 = MIN_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;
const MAX_SPACING : f64 = MAX_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;
const SQUEEZE_SPACING : f64 = SQUEEZE_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;

impl Road {
    pub fn new(seed : u32, round : u8, y : i32, inverted : bool) -> Self {
        let rng = FroggyRng::from_hash((seed, round, y));

        let mut obstacles = Vec::with_capacity(16);
        let mut cur = 0.0;

        // Make sure that there is at least one space at the end of the cycle large enough to go through
        // Make sure we never produce an impossible level
        while ({
            cur += rng.gen_range(("car_spacing", obstacles.len()), MIN_SPACING, MAX_SPACING);
            cur < 1.0 - SQUEEZE_SPACING
        })
        {
            obstacles.push(Obstacle(cur));
        }

        let r_width = rng.gen_range("r_width", R_WIDTH_MIN, R_WIDTH_MAX);

        Road {
            row : ObstacleRow::new(y, inverted, TIME_SCALE, obstacles, r_width),
        }
    }

    pub fn get_cars_public(&self, time_us : u32) -> Vec<ObstaclePublic> {
        self.row.get_obstacles_public(time_us)
    }

    pub fn collides_car(&self, time_us : u32, frog_pos : CoordPos) -> bool {
        if (frog_pos.y != self.row.y) {
            return false
        }

        let frog_centre = frog_pos.x as f64 + 0.5;

        for car in &self.row.get_obstacles_onscreen(time_us) {
            // Be a little kind
            const MARGIN : f64 = CAR_WIDTH / 2.25;
            let realised_car = self.row.realise_obstacle(car);
            if (frog_centre - realised_car).abs() < MARGIN {
                debug_log!("Killing, Collided with car {} {:?}", realised_car, frog_pos);
                return true;
            }
        }

        false
    }
}