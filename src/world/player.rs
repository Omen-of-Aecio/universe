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
pub struct Player {
    pub shape: Polygon,
    jump: Option<Jump>,
}

impl Player {
    pub fn new(shape: Polygon) -> Player {
        Player {
            shape: shape,
            jump: None,
        }
    }

    pub fn update(&mut self, tilenet: &TileNet<Tile>, gravity: f32) {
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
        self.shape.vel += Vec2::new(0.0, -gravity);
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
            let normal = world::get_normal(&tilenet, world::i32_to_usize(polygon_state.poc), self.shape.color);
            self.regain_jump(normal);
            assert!( !(normal.x == 0.0 && normal.y == 0.0));

            // Physical response
            self.shape.collide_wall(normal);

            // Debug vectors (TODO needs $mut world)
            /*
            self.vectors
                .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32),
                       normal.scale(-1.0)));
           */


            // Move away one unit from wall
            let mut moveaway_state = PolygonState::new(1.0, normal.normalize());
            self.shape.solve(&tilenet, &mut moveaway_state);

            // Try to move further with the current velocity
            polygon_state = PolygonState::new(time_left, self.shape.vel);
            self.shape.solve(&tilenet, &mut polygon_state);

            // Move back one unit
            let mut moveback_state = PolygonState::new(1.0, normal.normalize().scale(-1.0));
            self.shape.solve(&tilenet, &mut moveback_state);

            i += 1;
            time_left -= polygon_state.toc;
        }

        if polygon_state.collision {
            // One last physical response for the last collision
            let normal = world::get_normal(&tilenet, world::i32_to_usize(polygon_state.poc), self.shape.color);
            self.regain_jump(normal);
            let _ = self.shape.collide_wall(normal);
        }

        //debug!("Position in world, "; "x" => self.shape.pos.x, "y" => self.shape.pos.y);

        // Add debug vectors
        /*
        self.vectors.extend(polygon_state.debug_vectors.iter().cloned());
        */

        // Friction
        self.shape.vel = self.shape.vel * 0.9;
    }

    pub fn jump(&mut self) {
        if self.jump.is_none() {
            self.jump = Some(Jump::new(JUMP_DURATION, JUMP_ACC));
        }
    }
    /// Temporary solution to regain jump: check if normal is somewhat up..
    fn regain_jump(&mut self, normal: Vec2) {
        let up = Vec2::new(0.0, 1.0);
        if Vec2::dot(normal, up) > 0.0 {
            self.jump = None;
        }
    }
}
