use serde_derive::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

// ---

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

// ---

impl Vec2 {
    pub fn null_vec() -> Vec2 {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn length_squared(self) -> f32 {
        (self.x * self.x + self.y * self.y)
    }

    pub fn normalize(self) -> Vec2 {
        let len = self.length();
        Vec2::new(self.x / len, self.y / len)
    }

    /// TODO make clear that it clones?
    pub fn scale(self, x: f32, y: f32) -> Vec2 {
        Vec2::new(self.x * x, self.y * y)
    }

    pub fn scale_uni(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }

    pub fn dot(a: Vec2, b: Vec2) -> f32 {
        a.x * b.x + a.y * b.y
    }

    pub fn cross(a: Vec2, b: Vec2) -> f32 {
        a.x * b.y - a.y * b.x
    }

    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// Rotate the vector around the origin by a given angle
    pub fn rotate(self, angle: f32) -> Vec2 {
        let angle = self.angle() + angle;
        let x = angle.cos() * self.length();
        let y = angle.sin() * self.length();
        Vec2 { x, y }
    }
}

// ---

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul for Vec2 {
    type Output = Vec2;
    fn mul(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl Div for Vec2 {
    type Output = Vec2;
    fn div(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

// ---

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl MulAssign for Vec2 {
    fn mul_assign(&mut self, other: Vec2) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl DivAssign for Vec2 {
    fn div_assign(&mut self, other: Vec2) {
        self.x /= other.x;
        self.y /= other.y;
    }
}

// ---

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, n: f32) -> Vec2 {
        Vec2 {
            x: self.x * n,
            y: self.y * n,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, n: f32) -> Vec2 {
        Vec2 {
            x: self.x / n,
            y: self.y / n,
        }
    }
}

// ---

impl From<(f32, f32)> for Vec2 {
    fn from(point: (f32, f32)) -> Self {
        Vec2 {
            x: point.0,
            y: point.1,
        }
    }
}

impl From<Vec2> for (f32, f32) {
    fn from(point: Vec2) -> Self {
        (point.x, point.y)
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn null() {
        assert_eq![Vec2 { x: 0.0, y: 0.0 }, Vec2::null_vec(),];
    }

    #[test]
    fn dot() {
        assert_eq![
            3.6000001,
            Vec2::dot(Vec2 { x: 3.0, y: 0.0 }, Vec2 { x: 1.2, y: 3.8 })
        ];
    }

    #[test]
    fn cross() {
        assert_eq![
            6.0,
            Vec2::cross(Vec2 { x: 2.0, y: 0.0 }, Vec2 { x: 0.0, y: 3.0 })
        ];
        assert_eq![
            5.8,
            Vec2::cross(Vec2 { x: 2.0, y: 1.0 }, Vec2 { x: 0.2, y: 3.0 })
        ];
    }

    #[test]
    fn length() {
        assert_eq![10.0, Vec2 { x: 10.0, y: 0.0 }.length(),];
        assert_eq![10.0, Vec2 { x: 0.0, y: 10.0 }.length(),];
        assert_eq![10.0, Vec2 { x: -10.0, y: 0.0 }.length(),];
        assert_eq![10.0, Vec2 { x: 0.0, y: -10.0 }.length(),];
    }

    #[test]
    fn eq() {
        assert![Vec2 { x: 1.2, y: 3.4 } == Vec2 { x: 1.2, y: 3.4 }]
    }

    #[quickcheck_macros::quickcheck]
    fn rotate_preserves_length(angle: f32) {
        if angle.is_nan() {
            return;
        }
        let vec = Vec2 { x: 1.0, y: 1.0 };
        assert![(vec.length() - vec.rotate(angle).length()).abs() < 0.00001];
    }

    #[test]
    fn rotate_correct_direction() {
        let vec = Vec2 { x: 1.0, y: 0.0 };
        assert_eq![1.0, vec.rotate(std::f32::consts::PI / 2.0).y];
        assert_eq![-1.0, vec.rotate(std::f32::consts::PI).x];
    }

    #[test]
    fn add() {
        assert_eq![
            Vec2 { x: 1.2, y: 3.4 },
            Vec2 { x: 0.1, y: 3.2 } + Vec2 { x: 1.1, y: 0.2 }
        ];
    }

    #[test]
    fn add_assign() {
        let mut vec = Vec2 { x: 1.2, y: 3.4 };
        vec += Vec2 { x: 0.1, y: 3.2 };
        assert_eq![
            vec,
            Vec2 {
                x: 1.3000001,
                y: 6.6000004
            },
        ];
    }

    #[test]
    fn sub() {
        assert_eq![
            Vec2 { x: -1.0, y: 3.0 },
            Vec2 { x: 0.1, y: 3.2 } - Vec2 { x: 1.1, y: 0.2 }
        ];
    }

    #[test]
    fn sub_assign() {
        let mut vec = Vec2 { x: 1.2, y: 3.4 };
        vec -= Vec2 { x: 0.1, y: 3.2 };
        assert_eq![
            vec,
            Vec2 {
                x: 1.1,
                y: 0.20000005
            },
        ];
    }

    #[bench]
    fn single_floating_point_add(b: &mut Bencher) {
        fn add(l: f32, r: f32) -> f32 {
            l + r
        }
        b.iter(|| {
            for _ in 0..10_000 {
                black_box(add(black_box(1.0), black_box(2.0)));
            }
        });
    }

    #[bench]
    fn double_floating_point_add(b: &mut Bencher) {
        fn add(l: f64, r: f64) -> f64 {
            l + r
        }
        b.iter(|| {
            for _ in 0..10_000 {
                black_box(add(black_box(1.0), black_box(2.0)));
            }
        });
    }

    #[bench]
    fn single_floating_point_mul(b: &mut Bencher) {
        fn mul(l: f32, r: f32) -> f32 {
            l * r
        }
        b.iter(|| {
            for _ in 0..10_000 {
                black_box(mul(black_box(1.0), black_box(2.0)));
            }
        });
    }

    #[bench]
    fn double_floating_point_mul(b: &mut Bencher) {
        fn mul(l: f64, r: f64) -> f64 {
            l * r
        }
        b.iter(|| {
            for _ in 0..10_000 {
                black_box(mul(black_box(1.0), black_box(2.0)));
            }
        });
    }

    #[bench]
    fn dot_product_speed(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(Vec2::dot(
                    black_box(Vec2::new(0.1, 0.2)),
                    black_box(Vec2::new(4.3, -1.8)),
                ));
            }
        });
    }

    #[bench]
    fn cross_product_speed(b: &mut Bencher) {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(Vec2::cross(
                    black_box(Vec2::new(0.1, 0.2)),
                    black_box(Vec2::new(4.3, -1.8)),
                ));
            }
        });
    }
}
