use serde::{Deserialize, Serialize};

use crate::game::CoordPos;
use crate::rng::FroggyRng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadDescr {
    pub seed : u32,
    pub inverted : bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Car(f64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CarPublic(f64, i32, bool);

/// Describe cars as a closed function
/// Get a random generated velocity, constant for all cars on the road
/// Cars are a randomly spaced by the seed
/// They can then be described by (x0 + t * v) % width
/// This way they feel random but not collide with each other
///
/// We take a view into an interval say [r0, r1] where 0 < r0 < r1 < width.
/// We can simplify by normalising v=1 and varying size of the view.
#[derive(Debug, Clone)]
pub struct Road {
    cars0 : Vec<Car>,
    y : i32,
    r0 : f64, 
    r1 : f64,
    time_scale : f64,
    inverted : bool,
}

const CAR_WIDTH : f64 = 24.0 / 8.0;

impl Road {
    pub fn new(seed : u32, round : u8, y : i32, inverted : bool) -> Self {
        let rng = FroggyRng::from_hash((seed, round, y));
        debug_log!("Create road seed={} round={} y={} rng {:?}", seed, round, y, rng);

        const R_WIDTH_MIN : f64 = 0.15;
        const R_WIDTH_MAX : f64 = 0.35;
        let r_width = rng.gen_range("r_width", R_WIDTH_MIN, R_WIDTH_MAX);

        const MIN_CAR_SPACING_SCREEN : f64 = CAR_WIDTH  * 1.25;
        const MAX_CAR_SPACING_SCREEN : f64 = CAR_WIDTH  * 22.;

        // Min space to squeeze through
        const SQUEEZE_CAR_SPACING_SCREEN : f64 = CAR_WIDTH  * 3.25;
        //const TILES_TO_UNIT :  = (1.0 / super::SCREEN_SIZE as f64) * 

        //let tile_size_under_r = r_width / super::SCREEN_SIZE as f64;
        let min_car_spacing = MIN_CAR_SPACING_SCREEN / super::SCREEN_SIZE as f64;
        let max_car_spacing = MAX_CAR_SPACING_SCREEN / super::SCREEN_SIZE as f64;
        let squeeze_car_spacing_screen = SQUEEZE_CAR_SPACING_SCREEN / super::SCREEN_SIZE as f64;

        let mut cars0 = Vec::with_capacity(16);
        let mut cur = 0.0;

        // Make sure that there is at least one space at the end of the cycle large enough to go through
        // Make sure we never produce an impossible level
        while (cur < 1.0 - squeeze_car_spacing_screen) {
            cur += rng.gen_range(("car_spacing", cars0.len()), min_car_spacing, max_car_spacing);
            cars0.push(Car(cur));
        }

        Road {
            y,
            cars0,
            r0 : 0.5 - r_width,
            r1 : 0.5 + r_width,
            time_scale : 1.0 / 12_000_000.0, 
            inverted,
        }
    }

    pub fn collides_car(&self, time_us : u32, frog_pos : CoordPos) -> bool {
        if (frog_pos.y != self.y) {
            return false
        }

        let frog_centre = frog_pos.x as f64 + 0.5;

        for car in &self.get_cars_onscreen(time_us) {
            // Be a little kind
            const MARGIN : f64 = CAR_WIDTH / 2.25;
            let realised_car = self.realise_car(car);
            if (frog_centre - realised_car).abs() < MARGIN {
                debug_log!("Killing, Collided with car {} {:?}", realised_car, frog_pos);
                return true;
            }
        }

        false
    }

    fn realise_car(&self, car : &Car) -> f64 {
        let pos = if (self.inverted) {
            1.0 - car.0
        }
        else {
            car.0
        };

        let x_over = pos - self.r0;
        ((x_over * super::SCREEN_SIZE as f64) / (self.r1 - self.r0))
    }

    fn transform_car(&self, car : &Car) -> CarPublic {
        CarPublic(self.realise_car(car), self.y, self.inverted)
    }

    pub fn get_cars_public(&self, time_us : u32) -> Vec<CarPublic> {
        self.get_cars_onscreen(time_us)
            .iter()
            .map(|x| self.transform_car(x))
            .collect()
    }

    pub fn get_cars_onscreen(&self, time_us : u32) -> Vec<Car> {
        let mut cars = Vec::with_capacity(self.cars0.len());
        for car in &self.cars0 {
            let driven_car = car.drive(self.time_scale * time_us as f64);
            cars.push(driven_car);
        }

        cars
    }
}

impl Car {
    fn drive(self, time : f64 ) -> Self {
        Car(f64::fract(self.0 + time))
    }
}