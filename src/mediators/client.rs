use glocals::Client;
use libs::geometry::{grid2d::Grid, vec::Vec2};

/// Check if a float-based line collides with a predicate on a grid
///
/// This algorithm uses a modified version of bresenham's line algorithm.
/// It traces a line from start to finish and sees which indices it hits on
/// the grid. This version is the "supercover" algorithm, which means that
/// that going through diagonals will "touch" at least one of the grid points
/// on the side.
pub fn does_line_collide_with_grid<T: Clone + Default>(
    grid: &Grid<T>,
    from: Vec2,
    to: Vec2,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    let (start, stop) = (from, to);
    let new = stop - start;
    let (vx, vy) = (new.x, new.y);
    let slope_x = 1.0 + vy * vy / vx / vx;
    let slope_y = 1.0 + vx * vx / vy / vy;
    let (dx, dy) = (slope_x.sqrt(), slope_y.sqrt());

    let (mut ix, mut iy) = (start.x.floor() as i32, start.y.floor() as i32);

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

    let len = (stop.x.floor() as i64 - start.x.floor() as i64).abs() as usize
        + (stop.y.floor() as i64 - start.y.floor() as i64).abs() as usize;

    let mut it = 0;

    let dest_x = stop.x.floor() as i32;
    let dest_y = stop.y.floor() as i32;

    while it < len {
        it += 1;
        if ix >= 0 && iy >= 0 {
            if let Some(entry) = grid.get(ix as usize, iy as usize) {
                if predicate(entry) {
                    return Some((ix as usize, iy as usize))
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
    if ix >= 0 && iy >= 0 {
        if let Some(entry) = grid.get(ix as usize, iy as usize) {
            if predicate(entry) {
                return Some((dest_x as usize, dest_y as usize));
            }
        }
    }
    None
}

pub fn entry_point_client(s: &mut Client) {
    loop {}
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
}
