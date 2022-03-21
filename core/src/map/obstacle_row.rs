use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleRowDescr {
    pub seed : u32,
    pub inverted : bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Obstacle(pub f64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ObstaclePublic(pub f64, pub i32, pub bool);

/// Used for both cars on roads, and lillipads on rivers
/// 
/// Describe obstacles as a closed function
/// Get a random generated velocity, constant for all obstacles on the row
/// obstacles are a randomly spaced by the seed
/// They can then be described by (x0 + t * v) % width
/// This way they feel random but not collide with each other
///
/// We take a view into an interval say [r0, r1] where 0 < r0 < r1 < width.
/// We can simplify by normalising v=1 and varying size of the view.
#[derive(Debug, Clone)]
pub struct ObstacleRow {
    obstacles0 : Vec<Obstacle>,
    pub y : i32,
    r0 : f64, 
    r1 : f64,
    time_scale : f64,
    inverted : bool,
}

impl ObstacleRow {
    pub fn new(y : i32, inverted : bool, time_scale : f64, initial_obstacles : Vec<Obstacle>, r_width : f64) -> Self {
        ObstacleRow {
            y,
            obstacles0 : initial_obstacles,
            r0 : 0.5 - r_width,
            r1 : 0.5 + r_width,
            time_scale : 1.0 / time_scale,
            inverted,
        }
    }

    pub fn filter_object(&self, obstacle : &Obstacle, current_time : u32, row_start_time : u32) -> bool
    {
        //let hack_val = 0.75;
        //let since_start = (hack_val * obstacle.0 as f64 / self.time_scale as f64) as u32;
        //let spawn_time = row_start_time.saturating_sub(since_start);
        //current_time > spawn_time

        let t_over = current_time.saturating_sub(row_start_time);

        let x0_over_v = (obstacle.0 as f64 - self.r0) / self.time_scale;

        t_over > x0_over_v as u32
    }

    pub fn realise_obstacle(&self, obstacle : &Obstacle) -> f64 {
        let pos = if (self.inverted) {
            1.0 - obstacle.0
        }
        else {
            obstacle.0
        };

        let x_over = pos - self.r0;
        ((x_over * super::SCREEN_SIZE as f64) / (self.r1 - self.r0))
    }

    fn transform_car(&self, car : &Obstacle) -> ObstaclePublic {
        ObstaclePublic(self.realise_obstacle(car), self.y, self.inverted)
    }

    pub fn get_obstacles_public(&self, time_us : u32) -> Vec<ObstaclePublic> {
        self.get_obstacles_onscreen(time_us)
            .iter()
            .map(|x| self.transform_car(x))
            .collect()
    }

    pub fn get_obstacles_public_filtered(&self, time_us : u32, start_time : u32) -> Vec<ObstaclePublic> {
        self.get_obstacles_onscreen(time_us)
            .iter()
            .filter(|x| self.filter_object(x, time_us, start_time))
            .map(|x| self.transform_car(x))
            .collect()
    }

    pub fn get_obstacles_onscreen(&self, time_us : u32) -> Vec<Obstacle> {
        let mut cars = Vec::with_capacity(self.obstacles0.len());
        for car in &self.obstacles0 {
            let driven_car = car.at_time(self.time_scale * time_us as f64);
            cars.push(driven_car);
        }

        cars
    }

    pub fn get_obstacle(&self, time_us : u32, i : usize) -> Obstacle {
        self.obstacles0[i].at_time(self.time_scale * time_us as f64)
    }
}

impl Obstacle {
    fn at_time(self, time : f64 ) -> Self {
        Obstacle(f64::fract(self.0 + time))
    }
}