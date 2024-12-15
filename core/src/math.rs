use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl Eq for V2 {
}

impl Default for V2 {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl std::fmt::Display for V2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn norm_from_angle(angle: f32) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    pub fn mult(&self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k)
    }

    pub fn dist2(&self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy)
    }

    pub fn dist(&self, other: Self) -> f32 {
        self.dist2(other).sqrt()
    }

    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn mag2(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn mag(&self) -> f32 {
        self.mag2().sqrt()
    }

    pub fn norm(&self) -> Self {
        self.mult(1.0 / self.mag())
    }

    pub fn get_angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn normal_norm(self) -> Self {
        self.normal().norm()
    }

    pub fn normal(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn project_dist_towards(self, other: Self, dist: f32) -> Self {
        let diff = other - self;

        let diff_mag = diff.mag();
        let diff_with_dist = diff.mult(dist / diff_mag);

        self + diff_with_dist
    }
}

impl std::ops::Add for V2 {
    type Output = V2;

    fn add(self, rhs: Self) -> Self::Output {
        V2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for V2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x = self.x + rhs.x;
        self.y = self.y + rhs.y;
    }
}

impl std::ops::Sub for V2 {
    type Output = V2;

    fn sub(self, rhs: Self) -> Self::Output {
        V2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::SubAssign for V2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x = self.x - rhs.x;
        self.y = self.y - rhs.y;
    }
}

impl std::ops::Mul<V2> for f32 {
    type Output = V2;
    
    fn mul(self, rhs: V2) -> Self::Output {
        V2::new(self * rhs.x,self * rhs.y)
    }

}

impl std::ops::MulAssign<f32> for V2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x = self.x * rhs;
        self.y = self.y * rhs;
    }
}

impl std::ops::Mul<f32> for V2 {
    type Output = V2;
    
    fn mul(self, rhs: f32) -> Self::Output {
        V2::new(self.x * rhs,self.y * rhs)
    }

}

#[derive(Debug, Clone, Copy, Default)]
pub struct AARectangle {
    pub x : f32,
    pub y : f32,
    pub w : f32,
    pub h : f32,
}

impl AARectangle {
    pub fn top_left(&self) -> V2 {
        V2::new(self.x, self.y)
    }

    pub fn contains(&self, p: V2) -> bool {
        p.x > self.x && p.x < self.x + self.w
        &&
        p.y > self.y && p.y < self.y + self.h
    }
}

impl std::ops::Add<V2> for AARectangle {
    type Output = AARectangle;

    fn add(self, rhs: V2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            w: self.w,
            h: self.h,
        }
    }
}

impl std::ops::Sub<V2> for AARectangle {
    type Output = AARectangle;

    fn sub(self, rhs: V2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            w: self.w,
            h: self.h,
        }
    }
}