use geometry::polygon::{Polygon, PolygonState};
use geometry::vec::Vec2;
use world;
use world::jump::Jump;
use tile_net::TileNet;
use tile_net::Collable;
use global::Tile;
use world::color::Color;

const JUMP_DURATION: u32 = 4;
const JUMP_DELAY: u32 = 20; // Delay before you can jump again
const JUMP_ACC: f32 = 3.0;
const AIR_FRI: Vec2 = Vec2 { x: 0.91, y: 0.95 };
// (TODO extra friction when on ground?)


#[derive(Clone)]
pub struct Player {
    pub shape: Polygon,
    jump: Option<Jump>,
    force: Vec2, // Only where the player wants to move
}

impl Player {
    pub fn new(shape: Polygon) -> Player {
        Player {
            shape: shape,
            jump: None,
            force: Vec2::null_vec(),
        }
    }
    pub fn with_color(col: Color) -> Player { 
        Player {
            shape: match col {
                Color::White => Polygon::new_quad(0.0, 0.0, 10.0, 10.0, Color::White),
                Color::Black => Polygon::new_quad(0.0, 0.0, 10.0, 10.0, Color::Black),
            },
            jump: None,
            force: Vec2::null_vec(),
        }
    }

    pub fn update(&mut self, tilenet: &TileNet<Tile>, gravity: Vec2) {
        // Jump
        let mut acc = None;
        let mut progress = None;
        if let Some(ref mut jump) = self.jump {
            acc = jump.tick();
            progress = Some(jump.get_progress());
        }
        if let Some(acc) = acc {
            self.shape.vel.y += acc;
        }
        if let Some(progress) = progress {
            if progress > JUMP_DELAY {
                self.jump = None; // Regain jumping (like a sort of double jump)
            }
        }


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
        const MAX_HEIGHT: f32 = 8.0;
        let q = self.shape.vel.scale(1.0, 0.0);
        let mut time_left = 1.0;

        // To keep track of how much we have moved up in the attempt
        let mut offset = 0.0;

        while time_left > 0.0 {
            let mut polygon_state = PolygonState::new(q, time_left);
            self.shape.solve(&tilenet, &mut polygon_state);


            if polygon_state.collision {
                // If we cannot move, try one pixel up
                let mut moveup_state = PolygonState::new(Vec2::new(0.0, 1.0), 1.0);
                self.shape.solve(&tilenet, &mut moveup_state);
                if moveup_state.collision {
                    break;
                }

                if HEURISTIC1 {
                    // Decrease time left when climbing up
                    time_left -= 1.0 / self.shape.vel.length();
                }

                if HEURISTIC2 {
                    // Decrease x speed based on how steep the hill is
                    let up: Vec2 = Vec2::new(0.0, 1.0);
                    let normal = world::get_normal(&tilenet, world::i32_to_usize(polygon_state.poc), self.shape.color)
                        .normalize();
                    let steepness = Vec2::dot(up, normal);
                    if steepness > 0.0 {
                        // Can't do pow of negative number (ceiling)
                        self.shape.vel.x *= 0.5 + 0.5 * f32::powf(steepness, 0.5);
                    }
                }

                time_left -= polygon_state.toc;
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
            let mut movedown_state = PolygonState::new(Vec2::new(0.0, -offset), 1.0);
            self.shape.solve(&tilenet, &mut movedown_state);
        }


        // Move Y
        let mut polygon_state = PolygonState::new(self.shape.vel.scale(0.0, 1.0), 1.0);
        self.shape.solve(&tilenet, &mut polygon_state);

        if polygon_state.collision {
            self.shape.vel.y = 0.0;
        }


        // Friction
        self.shape.vel = self.shape.vel * AIR_FRI;

        // Gravity
        self.shape.vel += gravity;

        //
        self.force = Vec2::null_vec();
    }

    pub fn accelerate(&mut self, force: Vec2) {
        self.force += force;
        self.shape.vel += force;
    }

    pub fn jump(&mut self) {
        if self.jump.is_none() {
            self.jump = Some(Jump::new(JUMP_DURATION, JUMP_ACC));
        }
    }
}
