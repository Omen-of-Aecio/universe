use crate::libs::geometry::{grid2d::Grid, vec::Vec2};

/// Check if multiple lines collide with some part of the grid given a predicate
///
/// Returns the first found collision with the grid. This collision does not convey
/// information about the distance travelled to cause the collision, so don't
/// think "this is the closest collision".
pub fn do_lines_collide_with_grid<T: Clone + Default>(
    grid: &Grid<T>,
    lines: &[(Vec2, Vec2)],
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    for line in lines {
        let result = does_line_collide_with_grid(grid, line.0, line.1, predicate);
        if result.is_some() {
            return result;
        }
    }
    None
}

/// Check if a float-based line collides with a predicate on a grid
///
/// This algorithm uses a modified version of bresenham's line algorithm.
/// It traces a line from start to finish and sees which indices it hits on
/// the grid. This version is the "supercover" algorithm, which means that
/// that going through diagonals will "touch" at least one of the grid points
/// on the side.
///
/// It's quite an unsafe algorithm. Right now, if you give a floating
/// point outside the range of i32, the code may overflow. To prevent an
/// infinite loop, a length counter in usize is used, but it's not perfect.
/// In the future, it may be an idea to bound the input to some maximum
/// values.
pub fn does_line_collide_with_grid<T: Clone + Default>(
    grid: &Grid<T>,
    start: Vec2,
    stop: Vec2,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    let new = stop - start;
    let (vx, vy) = (new.x, new.y);
    let slope_x = 1.0 + vy * vy / vx / vx;
    let slope_y = 1.0 + vx * vx / vy / vy;
    let (dx, dy) = (slope_x.sqrt(), slope_y.sqrt());

    let (mut ix, mut iy) = (start.x.floor() as i16 as i32, start.y.floor() as i16 as i32);

    let (sx, sy);
    let (mut ex, mut ey);

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

    let len = ((stop.x.floor() as i16 as i32 - start.x.floor() as i16 as i32).abs()
        + (stop.y.floor() as i16 as i32 - start.y.floor() as i16 as i32).abs())
        as u16;

    let mut it: u16 = 0;

    let dest_x = stop.x.floor() as i32;
    let dest_y = stop.y.floor() as i32;

    while it < len {
        it += 1;
        if ix >= 0 && iy >= 0 {
            if let Some(entry) = grid.get(ix as usize, iy as usize) {
                if predicate(entry) {
                    return Some((ix as usize, iy as usize));
                }
            }
        }
        if ex < ey {
            ex += dx;
            ix += sx;
        } else {
            ey += dy;
            iy += sy;
        }
    }
    if let Some(entry) = grid.get(dest_x as usize, dest_y as usize) {
        if predicate(entry) {
            return Some((dest_x as usize, dest_y as usize));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
                == do_lines_collide_with_grid(
                    &grid,
                    &[
                        (Vec2 { x: 0.0, y: 0.5 }, Vec2 { x: 3.0, y: 0.5 }),
                        (Vec2 { x: 0.0, y: 1.5 }, Vec2 { x: 3.0, y: 1.5 }),
                        (Vec2 { x: 0.0, y: 2.5 }, Vec2 { x: 3.0, y: 2.5 }),
                    ],
                    |x| *x,
                )
        ];
    }

    #[test]
    fn test_lines_no_hit() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(3, 3);
        assert![
            None == do_lines_collide_with_grid(
                &grid,
                &[
                    (Vec2 { x: 0.0, y: 0.5 }, Vec2 { x: 3.0, y: 0.5 }),
                    (Vec2 { x: 0.0, y: 1.5 }, Vec2 { x: 3.0, y: 1.5 }),
                    (Vec2 { x: 0.0, y: 2.5 }, Vec2 { x: 3.0, y: 2.5 }),
                ],
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

    use rand::prelude::*;
    use test::{black_box, Bencher};
    #[bench]
    fn long_distance_to_collision(b: &mut Bencher) {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(100, 10);
        let mut rng = rand::thread_rng();
        for i in 0..100 {
            *grid.get_mut(i, 0).unwrap() = rng.gen();
        }
        let end_x: u8 = rng.gen();
        let end_y: bool = rng.gen();
        b.iter(|| {
            let result = does_line_collide_with_grid(
                &grid,
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 {
                    x: end_x as f32,
                    y: if end_y { 8.0 } else { 0.0 },
                },
                |x| black_box(*x),
            );
            black_box(result);
        });
    }
}
