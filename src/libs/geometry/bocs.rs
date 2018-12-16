use super::vec::Vec2;

#[derive(Clone, Copy)]
pub struct Bocs {
    pub start: Vec2,
    pub difference: Vec2,
}

impl Bocs {
    pub fn is_point_inside(&self, point: Vec2) -> bool {
        let stop = self.start + self.difference;
        point.x <= stop.x && point.y <= stop.y && point.x >= self.start.x && point.y >= self.start.y
            || point.x >= stop.x
                && point.y >= stop.y
                && point.x <= self.start.x
                && point.y <= self.start.y
    }

    pub fn is_bocs_inside(&self, bocs: Bocs) -> bool {
        self.is_point_inside(bocs.start)
            || self.is_point_inside(bocs.start + bocs.difference)
            || bocs.is_point_inside(self.start)
            || bocs.is_point_inside(self.start + self.difference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn is_point_inside() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 0.0, y: 0.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 0.0, y: 5.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 10.0, y: 0.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 10.0, y: 5.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 5.0, y: 2.5 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 3.0, y: 1.5 })];

        assert_eq![false, bocs.is_point_inside(Vec2 { x: -1.0, y: 0.0 })];
        assert_eq![false, bocs.is_point_inside(Vec2 { x: 0.0, y: -1.0 })];
        assert_eq![false, bocs.is_point_inside(Vec2 { x: -1.0, y: -1.0 })];
        assert_eq![false, bocs.is_point_inside(Vec2 { x: -1.0, y: 5.0 })];
        assert_eq![false, bocs.is_point_inside(Vec2 { x: 10.0, y: -1.0 })];
    }

    #[test]
    fn is_point_inside_negative_difference() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: -10.0, y: -5.0 },
        };
        assert_eq![true, bocs.is_point_inside(Vec2 { x: 0.0, y: 0.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: -10.0, y: -5.0 })];
        assert_eq![true, bocs.is_point_inside(Vec2 { x: -5.0, y: -2.5 })];

        assert_eq![false, bocs.is_point_inside(Vec2 { x: 5.0, y: 2.5 })];
        assert_eq![false, bocs.is_point_inside(Vec2 { x: 10.0, y: 5.0 })];
    }

    #[test]
    fn is_overlapping_bocs_inside() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn is_caller_inside() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let inside = Bocs {
            start: Vec2 { x: 1.0, y: 1.0 },
            difference: Vec2 { x: 8.0, y: 3.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(inside)];
    }

    #[test]
    fn is_callee_inside() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: -1.0, y: -1.0 },
            difference: Vec2 { x: 12.0, y: 7.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_touching_lines() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 10.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_touching_points() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 10.0, y: 5.0 },
            difference: Vec2 { x: 1.0, y: 1.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_touching_line_left() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: -10.0, y: -5.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_touching_points_corner() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 0.0, y: 5.0 },
            difference: Vec2 { x: -10.0, y: -5.0 },
        };
        assert_eq![true, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_not_touching_points_corner() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let overlap = Bocs {
            start: Vec2 { x: 0.0, y: 5.00001 },
            difference: Vec2 { x: -10.0, y: -5.0 },
        };
        assert_eq![false, bocs.is_bocs_inside(overlap)];
    }

    #[test]
    fn juxtaposed_bocs_not_touching() {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let juxta = Bocs {
            start: Vec2 {
                x: 10.000001,
                y: 0.0,
            },
            difference: Vec2 { x: 1.0, y: 5.0 },
        };
        assert_eq![false, bocs.is_bocs_inside(juxta)];
    }

    #[bench]
    fn check_aabb_when_no_collision(b: &mut Bencher) {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        let juxta = Bocs {
            start: Vec2 {
                x: 10.000001,
                y: 0.0,
            },
            difference: Vec2 { x: 1.0, y: 5.0 },
        };
        b.iter(|| {
            black_box(black_box(bocs).is_bocs_inside(black_box(juxta)));
        });
    }

    #[bench]
    fn check_aabb_when_collision(b: &mut Bencher) {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        b.iter(|| {
            black_box(black_box(bocs).is_bocs_inside(black_box(bocs)));
        });
    }

    #[bench]
    fn check_point_when_no_collision(b: &mut Bencher) {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        b.iter(|| {
            black_box(black_box(bocs).is_point_inside(black_box(Vec2 { x: -5.0, y: -2.5 })));
        });
    }

    #[bench]
    fn check_point_when_collision(b: &mut Bencher) {
        let bocs = Bocs {
            start: Vec2 { x: 0.0, y: 0.0 },
            difference: Vec2 { x: 10.0, y: 5.0 },
        };
        b.iter(|| {
            black_box(black_box(bocs).is_point_inside(black_box(Vec2 { x: 5.0, y: 2.5 })));
        });
    }
}
