use crate::game::CoordPos;
use crate::map::obstacle_row::*;
use crate::rng::FroggyRng;

#[derive(Debug)]
pub struct Road {
    row : ObstacleRow,
}

const CAR_WIDTH : f64 = 24.0 / 8.0;

const R_WIDTH_MIN : f64 = 0.11;
const R_WIDTH_MAX : f64 = 0.31;
const TIME_SCALE : f64 = 8_500_000.0;

const MIN_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 0.8;
const MAX_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 10.5;
const SQUEEZE_SPAWN_DIST_TILES : f64 = CAR_WIDTH * 3.45;


impl Road {
    pub fn new(seed : u32, round : u8, y : i32, inverted : bool) -> Self {
        let rng = FroggyRng::from_hash((seed, round, y));


        let r_width = rng.gen_froggy("r_width", R_WIDTH_MIN, R_WIDTH_MAX, 4);

        let r = r_width * 2.;
        let min_spacing = r * MIN_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;
        let max_spacing = r * MAX_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;
        let squeeze_spacing = r * SQUEEZE_SPAWN_DIST_TILES / crate::SCREEN_SIZE as f64;

        let mut obstacles = Vec::with_capacity(16);

        // initial spacing
        let mut cur = rng.gen_range("car_spacing_0", 0., min_spacing) + rng.gen_range("car_spacing_0_0", 0., min_spacing);

        // Make sure that there is at least one space at the end of the cycle large enough to go through
        // Make sure we never produce an impossible level
        while (cur < 1.0 - squeeze_spacing)
        {
            obstacles.push(Obstacle(cur));
            cur += rng.gen_froggy(("car_spacing", obstacles.len()), min_spacing, max_spacing, 2);
        }

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