use geometry::polygon::{Polygon, PolygonState};
use geometry::vec::Vec2;
use world;
use world::jump::Jump;
use tile_net::TileNet;
use tile_net::Collable;
use global::Tile;

const JUMP_DURATION: u32 = 4;
const JUMP_DELAY: u32 = 20; // Delay before you can jump again
const JUMP_ACC: f32 = 3.0;
const AIR_FRI: Vec2 = Vec2 { x: 0.91, y: 0.95 };
const GROUND_FRI: f32 = 0.9;

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

        // Physics
        // - Until we reach end of frame:
        // - Do collision with remaining time
        // - While collision
        //   * move a bit away from the wall
        //   * try to move further
        //   * negate that movement away from wall

        // Possible improvements
        // - if it still sometimes gets stuc, maybe use the normal of the next real collision
        //   for moving a unit away, since it's this normal that really signifies the problem
        //   (increases complexity)

        let mut i = 0;
        let mut time_left = 1.0;

        let mut polygon_state = PolygonState::new(time_left, self.shape.vel);
        self.shape.solve(&tilenet, &mut polygon_state);

        while polygon_state.collision && time_left > 0.1 && i<= 10 {
            time_left -= polygon_state.toc;

            let normal = world::get_normal(&tilenet, world::i32_to_usize(polygon_state.poc), self.shape.color);
            assert!( !(normal.x == 0.0 && normal.y == 0.0));

            // Physical/interactive response
            self.collide_wall(normal, gravity);

            let moveaway_scale = 1.0;
            // Move away one unit from wall
            let mut moveaway_state = PolygonState::new(1.0, normal.normalize().scale(moveaway_scale));
            self.shape.solve(&tilenet, &mut moveaway_state);

            // Try to move further with the current velocity
            polygon_state = PolygonState::new(time_left, self.shape.vel);
            // TODO ^ are we using more time than we have here? We decrease it one too many
            self.shape.solve(&tilenet, &mut polygon_state);

            // Move back one unit
            let mut moveback_state = PolygonState::new(1.0, normal.normalize().scale(-moveaway_scale));
            self.shape.solve(&tilenet, &mut moveback_state);

            i += 1;
        }

        if polygon_state.collision {
            // One last physical response for the last collision
            let normal = world::get_normal(&tilenet, world::i32_to_usize(polygon_state.poc), self.shape.color);
            self.collide_wall(normal, gravity);
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

    /// - Physical response of the shape.
    /// - Regain jump.
    /// - Climb hills - this is the hardest part
    /// - Stay put on hills.
    fn collide_wall(&mut self, normal: Vec2, gravity: Vec2) {
        let tangent = Vec2::new(-normal.y, normal.x).normalize();
        // Collide wall
        self.shape.collide_wall(normal);

        // If on "ground"...
        let up = Vec2::new(0.0, 1.0);
        if Vec2::dot(normal, up) > 0.0 {
            // Regain jump
            self.jump = None;
            // If player wants to go up that direction
            if normal.x.signum() != self.force.x.signum() {
                // Help player climb hill by working against gravity
                // The current solution makes it kinda jump when it goes over just a little bump ..
                self.shape.vel -= gravity * Vec2::cross(normal, up);
            }
        }

        // If player isn't trying to move the character...
        if self.force.length() == 0.0 { 
            // Just friction from ground
            // Idea: Make friction greater when speed (WRT Normal) is greater, and probably outside of this if.
            self.shape.vel = self.shape.vel * GROUND_FRI;

            // Extra friction to help player stay put
            if Vec2::dot(normal, up) > 0.0 {
                self.shape.vel = Vec2::null_vec();
            }

        }
    }
}
