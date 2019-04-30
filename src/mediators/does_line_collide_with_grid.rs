use geometry::{grid2d::Grid, vec::Vec2};

fn does_line_collide_with_grid<T: Clone + Default>(
    grid: &Grid<T>,
    start: Vec2,
    end: Vec2,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    let mut line = Supercover::new(start, end);
    for (xi, yi) in line {
        if xi >= 0 && yi >= 0 {
            if let Some(entry) = grid.get(xi as usize, yi as usize) {
                if predicate(entry) {
                    return Some((xi as usize, yi as usize));
                }
            }
        }
    }
    None
}

/// Check if multiple lines collide with some part of the grid given a predicate
///
/// Returns the first found collision with the grid. This collision does not convey
/// information about the distance travelled to cause the collision, so don't
/// think "this is the closest collision".
/// Check if vertices will collide with the grid when moved with `velocity`.
/// Returns the new movement vector.
pub fn collision_test<T: Clone + Default>(
    vertices: &[Vec2],
    velocity: Vec2,
    grid: &Grid<T>,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    for vertex in vertices {
        let collision = does_line_collide_with_grid(grid, *vertex, *vertex + velocity, predicate);
        if collision.is_some() {
            return collision;
        }
    }
    None
    /*
    let mut lines: Vec<Supercover> = vertices.iter()
        .map(|vertex| Supercover::new(*vertex, *vertex+velocity)).collect();
    let mut prev: Vec<(i32, i32)> = vertices.iter()
        .map(|vertex| (vertex.x as i32, vertex.y as i32)).collect();
    let len = lines[0].len();
    for _ in 0..len {
        for (i, line) in lines.iter_mut().enumerate() {
            match line.next() {
                Some((xi, yi)) => {
                    if xi >= 0 && yi >= 0 {
                        if let Some(entry) = grid.get(xi as usize, yi as usize) {
                            if predicate(entry) {
                                let start = vertices[i];
                                let end = Vec2::new(prev[i].0 as f32, prev[i].1 as f32) ;
                                return (end - start, (xi as usize, yi as usize), true);
                            }
                        }
                    }
                    prev[i] = (xi, yi);

                }
                None => unreachable!(),
            }
        }
    }
    (velocity, (0,0), false)
    */
}

struct Supercover {
    // Variables
    ex: f32,
    ey: f32,
    ix: i32,
    iy: i32,
    progress: u16,
    // Constant throughout algorithm
    len: u16,
    dx: f32,
    dy: f32,
    sx: i8, // Directions (-1, 1)
    sy: i8,
    dest_x: i32,
    dest_y: i32,
    done: bool,
}
impl Supercover {
    pub fn new(start: Vec2, stop: Vec2) -> Self {
        let new = stop - start;
        let (vx, vy) = (new.x, new.y);
        let slope_x = 1.0 + vy * vy / vx / vx;
        let slope_y = 1.0 + vx * vx / vy / vy;
        let (dx, dy) = (slope_x.sqrt(), slope_y.sqrt());

        let (mut ix, mut iy) = (
            i32::from(start.x.floor() as i16),
            i32::from(start.y.floor() as i16),
        );

        let (sx, sy);
        let (ex, ey);

        if vx < 0.0 {
            sx = -1;
            ex = start.x.fract() * dx;
        } else {
            sx = 1;
            ex = (1.0 - start.x.fract()) * dx;
        }

        if vy < 0.0 {
            sy = -1;
            ey = start.y.fract() * dy;
        } else {
            sy = 1;
            ey = (1.0 - start.y.fract()) * dy;
        }

        let stopx = stop
            .x
            .max(i16::max_value() as f32)
            .min(i16::min_value() as f32) as i16;
        let stopy = stop
            .y
            .max(i16::max_value() as f32)
            .min(i16::min_value() as f32) as i16;
        let startx = start
            .x
            .max(i16::max_value() as f32)
            .min(i16::min_value() as f32) as i16;
        let starty = start
            .y
            .max(i16::max_value() as f32)
            .min(i16::min_value() as f32) as i16;

        let xdiff = (i32::from(stopx) - i32::from(startx)).abs();
        let ydiff = (i32::from(stopy) - i32::from(starty)).abs();
        let len = ((xdiff + ydiff) as u16).min(u16::max_value() - 1);

        Supercover {
            progress: 0,
            dest_x: stop.x.floor() as i32,
            dest_y: stop.y.floor() as i32,
            len,
            ix,
            iy,
            dx,
            dy,
            sx,
            sy,
            ex,
            ey,
            done: false,
        }
    }
    pub fn len(&self) -> u16 {
        self.len + 1
    }

    pub fn progress(&self) -> u16 {
        self.progress
    }
}
impl Iterator for Supercover {
    type Item = (i32, i32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.progress < self.len {
            self.progress += 1;
            let point = (self.ix as i32, self.iy as i32);
            if self.ex < self.ey {
                self.ex += self.dx;
                self.ix += self.sx as i32;
            } else {
                self.ey += self.dy;
                self.iy += self.sy as i32;
            }
            Some(point)
        } else if self.progress == self.len {
            self.progress += 1;
            Some((self.dest_x, self.dest_y))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use test::{black_box, Bencher};

    #[test]
    fn i16_boundary_does_not_overflow() {
        let values = Supercover::new(Vec2 { x: 32767.0, y: 0.0 }, Vec2 { x: 32768.0, y: 0.0 })
            .collect::<Vec<_>>();
        assert_eq![1, values.len()];
    }

    #[test]
    fn test_no_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 10.0, y: 10.0 },
                |x| *x
            )
        ];
    }

    #[test]
    fn test_diagonal_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        *grid.get_mut(5, 5).unwrap() = true;
        assert![
            Some((5, 5))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 10.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_begin_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        *grid.get_mut(0, 0).unwrap() = true;
        assert![
            Some((0, 0))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 10.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_end_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        *grid.get_mut(9, 9).unwrap() = true;
        assert![
            Some((9, 9))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 10.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_end_but_stop_just_on_the_tile_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        *grid.get_mut(9, 9).unwrap() = true;
        assert![
            Some((9, 9))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 9.0, y: 9.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_skitting_along_low_portion() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(100, 2);
        for i in 0..99 {
            *grid.get_mut(i, 1).unwrap() = true;
        }
        assert![
            Some((50, 1))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 100.0, y: 2.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_skitting_along_low_portion_until_end() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(100, 2);
        for i in 0..100 {
            *grid.get_mut(i, 1).unwrap() = true;
        }
        assert![
            Some((99, 1))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 100.0, y: 1.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_collision_from_center_in_9x9_square_to_0_0() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        *grid.get_mut(0, 0).unwrap() = true;
        *grid.get_mut(0, 1).unwrap() = true;
        *grid.get_mut(1, 0).unwrap() = true;
        assert![
            Some((1, 0))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 1.5, y: 1.5 },
                    Vec2 { x: 0.0, y: 0.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_collision_from_center_in_9x9_square_to_0_3() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        *grid.get_mut(0, 2).unwrap() = true;
        *grid.get_mut(1, 2).unwrap() = true;
        *grid.get_mut(0, 1).unwrap() = true;
        assert![
            Some((1, 2))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 1.5, y: 1.5 },
                    Vec2 { x: 0.0, y: 3.0 },
                    |x| *x
                )
        ];
    }

    #[test]
    fn test_already_colliding() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        *grid.get_mut(0, 0).unwrap() = true;
        assert![
            Some((0, 0))
                == does_line_collide_with_grid(
                    &grid,
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 0.0, y: 0.0 },
                    |x| *x,
                )
        ];
    }

    #[test]
    fn test_already_colliding_but_inverted_predicate() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        *grid.get_mut(0, 0).unwrap() = true;
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 0.0, y: 0.0 },
                |x| !*x,
            )
        ];
    }

    #[test]
    fn test_lines() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        *grid.get_mut(2, 1).unwrap() = true;
        assert![
            Some((2, 1))
                == collision_test(
                    &[
                        Vec2 { x: 0.0, y: 0.5 },
                        Vec2 { x: 0.0, y: 1.5 },
                        Vec2 { x: 0.0, y: 2.5 }
                    ],
                    Vec2::new(3.0, 0.0),
                    &grid,
                    |x| *x,
                )
        ];
    }

    #[test]
    fn test_lines_no_hit() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        assert![
            None == collision_test(
                &[
                    Vec2 { x: 0.0, y: 0.5 },
                    Vec2 { x: 0.0, y: 1.5 },
                    Vec2 { x: 0.0, y: 2.5 }
                ],
                Vec2::new(3.0, 0.0),
                &grid,
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_extremely_long_vector() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 1e10, y: 0.0 },
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_infinity_vector() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 {
                    x: std::f32::INFINITY,
                    y: 0.0
                },
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_negative_infinity_vector() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 {
                    x: std::f32::NEG_INFINITY,
                    y: 0.0
                },
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_negative_infinity_vector_from_1_1() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 1.0, y: 1.0 },
                Vec2 {
                    x: std::f32::NEG_INFINITY,
                    y: 0.0
                },
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_multiple_infinities() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 {
                    x: std::f32::NEG_INFINITY,
                    y: 1.0
                },
                Vec2 {
                    x: std::f32::NEG_INFINITY,
                    y: 0.0
                },
                |x| *x,
            )
        ];
    }

    #[test]
    fn test_semi_infinite_distance() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 1e18, y: 1.0 },
                Vec2 { x: -1e18, y: 0.0 },
                |x| *x,
            )
        ];
    }

    #[test]
    fn as_i32_as_expected() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert_eq![0i32, 0.99999 as i32];
        assert_eq![0i32, 0.09999 as i32];
        assert_eq![0i32, -0.09999 as i32];
        assert_eq![0i32, -0.99999 as i32];
        assert_eq![2147483647i32, 1e30 as i32];
    }

    #[test]
    fn test_i32_overflow() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(1, 1);
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 {
                    x: 2_147_483_647.0,
                    y: 1.0
                },
                Vec2 {
                    x: 3_000_000_000.0,
                    y: 0.0
                },
                |x| *x,
            )
        ];
    }

    #[bench]
    fn long_distance_to_collision(b: &mut Bencher) {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(100, 10);
        let mut rng = rand::thread_rng();
        for i in 0..100 {
            *grid.get_mut(i, 0).unwrap() = black_box(rng.gen());
        }
        let end_x: u8 = rng.gen();
        let end_y: bool = rng.gen();
        b.iter(|| {
            let result = does_line_collide_with_grid(
                black_box(&grid),
                black_box(Vec2 { x: 0.0, y: 0.0 }),
                black_box(Vec2 {
                    x: end_x as f32,
                    y: if end_y { 8.0 } else { 0.0 },
                }),
                black_box(|x| *x),
            );
            black_box(result);
        });
    }
}
