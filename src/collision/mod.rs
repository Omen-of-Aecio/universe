use tilenet::{TileNet, Collable};
use global::Tile;
use component::*;
use geometry::Vec2;

mod polygon;
mod ray;

pub use self::ray::RayCollable;
pub use self::polygon::PolygonCollable;


const AIR_FRI: Vec2 = Vec2 { x: 0.91, y: 0.95 };

pub fn player_move(pos: &mut Pos, vel: &mut Vel, force: &mut Force, shape: &Shape, color: &Color, tilenet: &TileNet<Tile>, gravity: Vec2) {
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
        let q = vel.transl.scale(1.0, 0.0);
        let mut time_left = 1.0;

        // To keep track of how much we have moved up in the attempt
        let mut offset = 0.0;

        while time_left > 0.0 {
            let (toc, poc, collision) = {
                let mut collable = PolygonCollable::new(shape, color, pos, q, time_left);
                collable.solve(&tilenet);
                (collable.toc, collable.poc, collable.collision)
                // TODO necessary because of mutability aliasing. Look for other solution?
                //      e.g. make solve() return result or something
            };


            if collision {
                // If we cannot move, try one pixel up
                let mut moveup_collable = PolygonCollable::new(shape, color, pos, Vec2::new(0.0, 1.0), 1.0);
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
                    let normal = ::get_normal(&tilenet, ::i32_to_usize(poc), *color)
                        .normalize();
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
            let mut movedown_collable = PolygonCollable::new(shape, color, pos, Vec2::new(0.0, -offset), 1.0);
            movedown_collable.solve(&tilenet);
        }


        // Move Y
        let mut collable = PolygonCollable::new(shape, color, pos, vel.transl.scale(0.0, 1.0), 1.0);
        collable.solve(&tilenet);

        if collable.collision {
            vel.transl.y = 0.0;
        }


        // Friction
        vel.transl = vel.transl * AIR_FRI;

        // Gravity
        vel.transl += gravity;

        //
        force.transl = Vec2::null_vec();
}