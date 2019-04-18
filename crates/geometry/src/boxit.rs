#[derive(Clone, Copy, Debug)]
pub struct Boxit {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,

    current_x: usize,
    current_y: usize,

    done: bool,
}

impl Boxit {
    pub fn with_center(radii: (usize, usize), center: (usize, usize)) -> Boxit {
        let left = if radii.0 >= center.0 {
            0
        } else {
            center.0 - radii.0
        };
        let (right, right_overflow) = center.0.overflowing_add(radii.0);

        let top = if radii.1 >= center.1 {
            0
        } else {
            center.1 - radii.1
        };
        let (bottom, bottom_overflow) = center.1.overflowing_add(radii.1);

        Boxit {
            top,
            left,
            bottom: if bottom_overflow {
                usize::max_value()
            } else {
                bottom
            },
            right: if right_overflow {
                usize::max_value()
            } else {
                right
            },

            current_x: left,
            current_y: top,

            done: false,
        }
    }
}

impl Iterator for Boxit {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else if self.current_x == self.right && self.current_y == self.bottom {
            self.done = true;
            Some((self.right, self.bottom))
        } else {
            let old_x = self.current_x;
            let old_y = self.current_y;
            if self.current_x < self.right {
                self.current_x += 1;
            } else if self.current_x == self.right {
                self.current_x = self.left;
                self.current_y += 1;
            }
            Some((old_x, old_y))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn simple() {
        let bx = Boxit::with_center((1, 1), (0, 0));
        let result: Vec<(usize, usize)> = bx.collect();
        assert_eq![vec![(0, 0), (1, 0), (0, 1), (1, 1),], result];
    }

    #[test]
    fn single_line_horizontal() {
        let bx = Boxit::with_center((10, 0), (0, 0));
        let result: Vec<(usize, usize)> = bx.collect();
        assert_eq![
            vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
                (10, 0),
            ],
            result
        ];
    }

    #[test]
    fn single_line_vertical() {
        let bx = Boxit::with_center((0, 10), (0, 0));
        let result: Vec<(usize, usize)> = bx.collect();
        assert_eq![
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
                (0, 8),
                (0, 9),
                (0, 10),
            ],
            result
        ];
    }

    #[test]
    fn single_line_horizontal_off_center() {
        let bx = Boxit::with_center((10, 0), (1, 123));
        let result: Vec<(usize, usize)> = bx.collect();
        assert_eq![
            vec![
                (0, 123),
                (1, 123),
                (2, 123),
                (3, 123),
                (4, 123),
                (5, 123),
                (6, 123),
                (7, 123),
                (8, 123),
                (9, 123),
                (10, 123),
                (11, 123),
            ],
            result
        ];
    }

    #[bench]
    fn iterate_1000x1000(b: &mut Bencher) {
        b.iter(|| {
            let mut bx = black_box(Boxit::with_center((0, 0), (1000, 1000)));
            let mut value = 0;
            while black_box(bx.next()).is_some() {
                value = black_box(value + 0);
            }
            black_box(value)
        });
    }
}
