use geometry::{grid2d::Grid, vec::Vec2};

fn does_line_collide_with_grid<T: Clone + Default>(
    grid: &Grid<T>,
    start: Vec2,
    end: Vec2,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    let line = Supercover::new(start, end);
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

/// returns alpha along line segment if collision happened
pub fn intersect_line_line_segment(
    line_start: Vec2,
    line_direction: Vec2,
    segment_start: Vec2,
    segment_end: Vec2,
) -> Option<f32> {
    let start_diff = line_start - segment_start;
    let segment1_vec = segment_end - segment_start;
    let segment2_vec = line_direction;
    let cross_vec = Vec2::cross(segment1_vec, segment2_vec);
    if cross_vec == 0.0 {
        return None; // This means colinear (if also cross(a_diff, line2_vec) == 0) OR parallel (else)
    }
    let alpha1 = Vec2::cross(start_diff, segment2_vec) / cross_vec; // alpha of line segment
    let alpha2 = Vec2::cross(start_diff, segment1_vec) / cross_vec; // alpha of line
    if alpha1 >= 0.0 && alpha1 <= 1.0 {
        Some(alpha1)
    } else {
        None
    }
}

/// Get extent of polygon projected onto the line of `dir`. Returns min and max
fn get_projected_extent(vertices: &[Vec2], dir: Vec2) -> (f32, f32) {
    let dir = dir.normalize();
    let mut min = std::f32::MAX;
    let mut max = std::f32::MIN;
    for vertex in vertices {
        let projected = Vec2::dot(*vertex, dir);
        if projected < min {
            min = projected;
        }
        if projected > max {
            max = projected;
        }
    }
    assert!(min != std::f32::MAX);
    assert!(max != std::f32::MIN);
    (min, max)
}

/// A line is defined by `point` and `dir`. Returns the intersection of this line and the polygon,
/// that is furthest in the direction of `dir`
fn furthest_line_shape_intersection(vertices: &[Vec2], point: Vec2, dir: Vec2) -> Option<Vec2> {
    let mut best_point = None;
    let mut best_extent = std::f32::MIN;
    let normal_dir = dir.normalize();
    let prev_vertices = vertices
        .iter()
        .cycle()
        .skip(vertices.len() - 1)
        .take(vertices.len());
    for (vertex, prev_vertex) in vertices.iter().zip(prev_vertices) {
        if let Some(alpha) = intersect_line_line_segment(point, dir, *prev_vertex, *vertex) {
            let point = *prev_vertex + (*vertex - *prev_vertex).scale_uni(alpha);
            let extent = Vec2::dot(dir, point);
            if extent > best_extent {
                best_extent = extent;
                best_point = Some(point)
            }
        }
    }
    best_point
}

/// Check if multiple lines collide with some part of the grid given a predicate
///
/// If `coarseness` is present, it determines how many points will be created, by signifying the
/// distance between each point generated in the direction of velocity.
///
/// Returns the first found collision with the grid. This collision does not convey
/// information about the distance travelled to cause the collision, so don't
/// think "this is the closest collision".
/// Check if vertices will collide with the grid when moved with `velocity`.
/// Returns the new movement vector.
pub fn collision_test<T: Clone + Default>(
    vertices: &[Vec2],
    coarseness: Option<f32>,
    velocity: Vec2,
    grid: &Grid<T>,
    predicate: fn(&T) -> bool,
) -> Option<(usize, usize)> {
    if velocity.length_squared() == 0.0 {
        return None;
    }

    let vertices = &match coarseness {
        Some(coarseness) => {
            // Create new points.
            // - project vertices in velocity direction to get extent
            let dir = velocity.normalize();
            let project_dir = Vec2::new(-dir.y, dir.x);
            let (min, max) = get_projected_extent(vertices, project_dir);
            // - calculate number of points
            let n_points = ((max - min) / coarseness).max(2.0).round();
            let dist_between_points = (max - min) / n_points;
            // - sample lines and find intersection with polygon
            let points: Vec<Vec2> = (0..n_points as i32)
                .filter_map(|i| {
                    let i = i as f32;
                    let point = project_dir.scale_uni(min + i * dist_between_points);
                    let a = furthest_line_shape_intersection(vertices, point, dir);
                    a
                })
                .collect();
            points
        }
        None => vertices.to_owned(), // TODO unnecessary clone
    };

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

#[derive(Clone, Copy)]
struct Supercover {
    // Variables
    ex: f32,
    ey: f32,
    ix: i32,
    iy: i32,
    progress: u32,
    // Constant throughout algorithm
    len: u32,
    dx: f32,
    dy: f32,
    sx: i8, // Directions (-1, 1)
    sy: i8,
    dest_x: i32,
    dest_y: i32,
}
impl Supercover {
    pub fn new(start: Vec2, stop: Vec2) -> Self {
        let new = stop - start;
        let (vx, vy) = (new.x, new.y);
        let slope_x = 1.0 + vy * vy / vx / vx;
        let slope_y = 1.0 + vx * vx / vy / vy;
        let (dx, dy) = (slope_x.sqrt(), slope_y.sqrt());

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
            .max(f32::from(i16::min_value()))
            .min(f32::from(i16::max_value()))
            .floor() as i16;
        let stopy = stop
            .y
            .max(f32::from(i16::min_value()))
            .min(f32::from(i16::max_value()))
            .floor() as i16;
        let startx = start
            .x
            .max(f32::from(i16::min_value()))
            .min(f32::from(i16::max_value()))
            .floor() as i16;
        let starty = start
            .y
            .max(f32::from(i16::min_value()))
            .min(f32::from(i16::max_value()))
            .floor() as i16;

        let xdiff = (i32::from(stopx) - i32::from(startx)).abs();
        let ydiff = (i32::from(stopy) - i32::from(starty)).abs();
        let len = (xdiff + ydiff) as u32;

        let (ix, iy) = (i32::from(startx), i32::from(starty));

        Supercover {
            progress: 0,
            dest_x: stop
                .x
                .min(f32::from(i16::max_value()))
                .max(f32::from(i16::min_value()))
                .floor() as i32,
            dest_y: stop
                .y
                .min(f32::from(i16::max_value()))
                .max(f32::from(i16::min_value()))
                .floor() as i32,
            len,
            ix,
            iy,
            dx,
            dy,
            sx,
            sy,
            ex,
            ey,
        }
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
                self.ix += i32::from(self.sx);
            } else {
                self.ey += self.dy;
                self.iy += i32::from(self.sy);
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
        assert_eq![(32767, 0), values[0]];

        let values = Supercover::new(
            Vec2 {
                x: 327670.0,
                y: 0.0,
            },
            Vec2 {
                x: 327680.0,
                y: 0.0,
            },
        )
        .collect::<Vec<_>>();
        assert_eq![1, values.len()];
        assert_eq![(32767, 0), values[0]];

        let values = Supercover::new(
            Vec2 {
                x: 327670.0,
                y: 0.0,
            },
            Vec2 { x: 1e12, y: 0.0 },
        )
        .collect::<Vec<_>>();
        assert_eq![1, values.len()];
        assert_eq![(32767, 0), values[0]];

        let values = Supercover::new(Vec2 { x: -1e12, y: 0.0 }, Vec2 { x: 1e12, y: 0.0 })
            .collect::<Vec<_>>();
        assert_eq![65536, values.len()];
        assert_eq![(-32768, 0), values[0]];
        assert_eq![(-32767, 0), values[1]];
        assert_eq![(-1, 0), values[32767]];
        assert_eq![(0, 0), values[32768]];
        assert_eq![(32764, 0), values[65532]];
        assert_eq![(32765, 0), values[65533]];
        assert_eq![(32766, 0), values[65534]];
        assert_eq![(32767, 0), values[65535]];

        let values = Supercover::new(Vec2 { x: -1e12, y: -1e12 }, Vec2 { x: 1e12, y: -1e12 })
            .collect::<Vec<_>>();
        assert_eq![65536, values.len()];
    }

    #[test]
    fn almost_zero_slope_contains_stop() {
        let cover = Supercover::new(
            Vec2 {
                x: -32768.0,
                y: -32767.5,
            },
            Vec2 {
                x: 32767.0,
                y: -32766.5,
            },
        );
        let first = cover.clone().next().unwrap();
        let last = cover.clone().last().unwrap();
        assert_eq![(-32768, -32768), first];
        assert_eq![(32767, -32767), last];

        let last = Supercover::new(
            Vec2 {
                x: -32768.0,
                y: -32767.5,
            },
            Vec2 {
                x: 32767.0,
                y: -32768.5,
            },
        )
        .last()
        .unwrap();
        assert_eq![(32767, -32768), last];

        let cover = Supercover::new(
            Vec2 { x: -1e13, y: 0.0 },
            Vec2 {
                x: -1.01e13,
                y: 5.0,
            },
        );
        let first = cover.clone().next().unwrap();
        let last = cover.clone().last().unwrap();
        assert_eq![(-32768, 0), first];
        assert_eq![(-32768, 5), last];

        let cover = Supercover::new(
            Vec2 {
                x: -50_000.0,
                y: -40_000.0,
            },
            Vec2 {
                x: 50_000.0,
                y: 40_000.0,
            },
        );
        let first = cover.clone().next().unwrap();
        let last = cover.clone().last().unwrap();
        for i in cover {
            // TODO: Maybe ensure this holds, the current clamping method does not ensure that
            // values are inside.
            // assert![i.0 <= i16::max_value() as i32];
            assert![i.0 <= u16::max_value() as i32 * 2];
        }
        assert_eq![131071, cover.count()];
        assert_eq![(-32768, -32768), first];
        assert_eq![(32767, 32767), last];

        let cover = Supercover::new(
            Vec2 {
                x: std::f32::NEG_INFINITY,
                y: -40_000.0,
            },
            Vec2 {
                x: std::f32::INFINITY,
                y: 40_000.0,
            },
        );
        let first = cover.clone().next().unwrap();
        let second_last = cover.clone().skip(131069).next().unwrap();
        let last = cover.clone().last().unwrap();
        assert![second_last.1 - first.1 <= u16::max_value() as i32 * 2 - 1];
        assert_eq![131071, cover.count()];
        assert_eq![(-32768, -32768), first];
        assert_eq![(-32768, 98301), second_last];
        assert_eq![(32767, 32767), last];
    }

    #[test]
    fn corner_to_corner() {
        let iterator = Supercover::new(
            Vec2 {
                x: i16::min_value() as f32,
                y: i16::min_value() as f32,
            },
            Vec2 {
                x: i16::max_value() as f32,
                y: i16::max_value() as f32,
            },
        );
        assert_eq![
            (u16::max_value() as u32 * 2 + 1) as usize,
            iterator.collect::<Vec<_>>().len()
        ];
        assert_eq![Some((-32768, -32768)), iterator.clone().next()];
        assert_eq![Some((32767, 32767)), iterator.clone().last()];

        let iterator = Supercover::new(
            Vec2 {
                x: i16::max_value() as f32,
                y: i16::max_value() as f32,
            },
            Vec2 {
                x: i16::min_value() as f32,
                y: i16::min_value() as f32,
            },
        );
        assert_eq![
            (u16::max_value() as u32 * 2 + 1) as usize,
            iterator.collect::<Vec<_>>().len()
        ];
        assert_eq![Some((32767, 32767)), iterator.clone().next()];
        assert_eq![Some((-32768, -32768)), iterator.clone().last()];
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
    fn test_top_collision() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        use geometry::boxit::*;
        for (x, y) in Boxit::with_center((9, 0), (0, 0)) {
            grid.set(x, y, true);
        }
        assert_eq![
            1,
            Supercover::new(Vec2 { x: 5.0, y: -1.0 }, Vec2 { x: 5.0, y: -0.1 }).count()
        ];
        assert![
            None == does_line_collide_with_grid(
                &grid,
                Vec2 { x: 5.0, y: -1.0 },
                Vec2 { x: 5.0, y: -0.1 },
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

    #[bench]
    fn maximum_distance_to_collision(b: &mut Bencher) {
        let iterator = Supercover::new(
            Vec2 {
                x: i16::min_value() as f32,
                y: i16::min_value() as f32,
            },
            Vec2 {
                x: i16::max_value() as f32,
                y: i16::max_value() as f32,
            },
        );
        b.iter(|| {
            let mut count: u32 = 0;
            for _ in iterator {
                count += 1;
            }
            assert_eq![u16::max_value() as u32 * 2 + 1, count];
        });
    }
}
