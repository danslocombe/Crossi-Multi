use serde::{Deserialize, Serialize};

use crate::game::CoordPos;
use crate::rng::FroggyRng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadDescr {
    pub seed : u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Car(f64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CarPublic(f64, i32);

// Describe cars as a closed function
// Get a random generated velocity, constant for all cars on the road
// Cars are a randomly spaced by the seed
// They can then be described by (x0 + t * v) % width
// This way they feel random but not collide with each other
//
// We take a view into an interval say [r0, r1] where 0 < r0 < r1 < width.
// We can simplify by normalising v=1 and varying size of the view.
#[derive(Debug, Clone)]
pub struct Road {
    cars0 : Vec<Car>,
    y : i32,
    r0 : f64, 
    r1 : f64,
    time_scale : f64,
}

const CAR_WIDTH : f64 = 1.25;

impl Road {
    pub fn from_seed(seed : u32, y : i32) -> Self {
        let mut rng = FroggyRng::new(seed);
        let car_count = 8;

        const R_WIDTH_MIN : f64 = 0.2;
        const R_WIDTH_MAX : f64 = 0.25;
        let r_width = rng.next_range(R_WIDTH_MIN, R_WIDTH_MAX);

        let mut cars0 = Vec::with_capacity(car_count);
        for i in 0..car_count {
            // todo
            cars0.push(Car(i as f64 / car_count as f64));
        }

        Road {
            y,
            cars0,
            r0 : 0.5 - r_width,
            r1 : 0.5 + r_width,
            time_scale : 1.0 / 5_000_000.0, 
        }
    }

    pub fn collides_car(&self, time_us : u32, frog_pos : CoordPos) -> bool {
        if (frog_pos.y != self.y) {
            return false
        }

        for car in &self.get_cars_onscreen(time_us) {
            if (frog_pos.x as f64 - self.realise_car(car).abs() < CAR_WIDTH) {
                return true;
            }
        }

        false
    }

    fn realise_car(&self, car : &Car) -> f64 {
        let x_over = car.0 - self.r0;
        ((x_over * super::SCREEN_SIZE as f64) / (self.r1 - self.r0))
    }

    fn transform_car(&self, car : &Car) -> CarPublic {
        CarPublic(self.realise_car(car), self.y)
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
            if (driven_car.on_screen()) {
                cars.push(driven_car);
            }
        }

        cars
    }
}

impl Car {
    fn drive(self, time : f64 ) -> Self {
        Car(f64::fract(self.0 + time))
    }

    fn on_screen(&self) -> bool {
        // OAFIJWOIFJAWOFJAWOFj
        self.0 > -CAR_WIDTH || self.0 < super::SCREEN_SIZE as f64 + CAR_WIDTH
    }
}