use glocals::component::*;
use libs::geometry::Vec2;
use glocals::Tile;
use tilenet::{Collable, TileNet};

mod polygon;
mod ray;

pub use self::polygon::PolygonCollable;
pub use self::ray::RayCollable;

pub fn bullet_move(
    pos: &mut Pos,
    vel: &mut Vel,
    shape: &Shape,
    color: Color,
    tilenet: &TileNet<Tile>,
    delta_time: f32,
) -> ((i32, i32), bool) {
    // solve once
    // if collision
    //      mutate tilenet (maybe return the result)
    //      delete entity
    let mut collable = PolygonCollable::new(shape, color, pos, vel.transl * delta_time, 1.0);
    collable.solve(&tilenet);
    // (collable.toc, collable.poc, collable.collision)
    // TODO (copied the above forthe most part)
    (collable.poc, collable.collision)
}

/// Returns true if collision happened
pub fn player_move(
    pos: &mut Pos,
    vel: &mut Vel,
    shape: &Shape,
    color: Color,
    tilenet: &TileNet<Tile>,
    delta_time: f32,
) -> bool {
    /* KISS algorithm for moving
     *  - not trying to be so physical
     * Tried several heuristics/meaasures to make it better. Keep them around to play with.
     * They all have some problems, but I think I'd prefer Heuristic 2 over Heuristic 1,
     * because H1 makes the maximum height we can climb dependent on velocity.
     * Heuristic 3 has potential problems.
     *
     * Right now, I only set vel.y to 0 when we have contact with ground.
     * Could also set vel.x to 0. (But might be necessary to check that it's ground and not
     * ceiling, by checking the normal).
     *
     * A small shortcoming: if we have great speed, we can only climb MAX_HEIGHT pixels in
     * y-direction in total total - desired: for every move in y direction you can climb MAX_HEIGHT pixels.
     *
     * TODO: Problem: When top of polygon hits a wall; it can climb high.
     */
    const HEURISTIC1: bool = false;
    const HEURISTIC2: bool = true;
    const HEURISTIC3: bool = true;

    // Move X
    const MAX_HEIGHT: f32 = 4.0;
    let q = vel.transl.scale(delta_time, 0.0);
    let mut time_left = 1.0;

    // To keep track of how much we have moved up in the attempt
    let mut offset = 0.0;

    let mut has_collided = false;
    while time_left > 0.0 {
        let (toc, poc, collision) = {
            let mut collable = PolygonCollable::new(shape, color, pos, q, time_left);
            collable.solve(&tilenet);
            (collable.toc, collable.poc, collable.collision)
        };

        if collision {
            has_collided = true;
            // If we cannot move, try one pixel up
            let mut moveup_collable =
                PolygonCollable::new(shape, color, pos, Vec2::new(0.0, 1.0), 1.0);
            moveup_collable.solve(&tilenet);
            if moveup_collable.collision {
                break;
            }

            if HEURISTIC1 {
                // Decrease time left when climbing up
                time_left -= 1.0 / vel.transl.length();
            }

            if HEURISTIC2 {
                // Decrease x speed based on how steep the hill is
                let up: Vec2 = Vec2::new(0.0, 1.0);
                let normal = get_normal(&tilenet, i32_to_usize(poc), color).normalize();
                let steepness = Vec2::dot(up, normal);
                if steepness > 0.0 {
                    // Can't do pow of negative number (ceiling)
                    vel.transl.x *= 0.5 + 0.5 * f32::powf(steepness, 0.5);
                }
            }

            time_left -= toc;
            offset += 1.0;

            if offset > MAX_HEIGHT {
                break;
            }
        } else {
            // We have moved all we wanted
            break;
        }
    }

    if HEURISTIC3 {
        // Try to climb down again whatever distance climbed up.
        let mut movedown_collable =
            PolygonCollable::new(shape, color, pos, Vec2::new(0.0, -offset), 1.0);
        movedown_collable.solve(&tilenet);
    }

    // Move Y
    let mut collable =
        PolygonCollable::new(shape, color, pos, vel.transl.scale(0.0, delta_time), 1.0);
    collable.solve(&tilenet);

    if collable.collision {
        vel.transl.y = 0.0;
    }

    has_collided
}

pub fn map_tile_value_via_color(tile: Tile, color: Color) -> Tile {
    match (tile, color) {
        (0, Color::Black) => 255,
        (255, Color::Black) => 0,
        _ => tile,
    }
}

pub fn get_normal(tilenet: &TileNet<Tile>, coord: (usize, usize), color: Color) -> Vec2 {
    let cmap = map_tile_value_via_color;
    /*
    let kernel = match color {
        Color::WHITE => [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]],
        Color::BLACK => [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]],
    };
    */
    let kernel = [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]];
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (y, row) in kernel.iter().enumerate() {
        for (x, _) in row.iter().enumerate() {
            if let (Some(x_coord), Some(y_coord)) =
                ((coord.0 + x).checked_sub(1), (coord.1 + y).checked_sub(1))
            {
                if let Some(&v) = tilenet.get((x_coord, y_coord)) {
                    dx += kernel[y][x] * f32::from(cmap(v, color)) / 255.0;
                }
                if let Some(&v) = tilenet.get((x_coord, y_coord)) {
                    dy += kernel[x][y] * f32::from(cmap(v, color)) / 255.0;
                }
            }
        }
    }
    Vec2::new(dx, dy)
}

pub fn i32_to_usize(mut from: (i32, i32)) -> (usize, usize) {
    if from.0 < 0 {
        from.0 = 0;
    }
    if from.1 < 0 {
        from.1 = 0;
    }
    (from.0 as usize, from.1 as usize)
}
