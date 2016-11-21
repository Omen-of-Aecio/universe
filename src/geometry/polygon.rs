use tile_net::*;
use geometry::vec::Vec2;
use global::Tile;

pub struct Polygon {
    pub points: Vec<(f32, f32)>, // Vec<Vec2> later. Now: for convenience with TileNet
    pub pos: Vec2,
    pub ori: f32,

    pub vel: Vec2, // rot: f32,
}

impl Polygon {
    pub fn new_quad(start_x: f32, start_y: f32, width: f32, height: f32) -> Polygon {
        let mut result = Polygon {
            points: Vec::new(),
            pos: Vec2::new(start_x, start_y),
            ori: 0.0,
            vel: Vec2::null_vec(),
        };
        result.points.push((0.0, 0.0));
        result.points.push((0.0, height));
        result.points.push((width, height));
        result.points.push((width, 0.0));
        result
    }

    /// Physical response to collision - i.e. bounce in direction of normal
    pub fn collide_wall(&mut self, normal: Vec2) -> (Vec2, Vec2) {
        // TODO can probably be optimized
				/*
        let normal = normal.normalize();
        let normal_90 = Vec2::new(-normal.y, normal.x);
        self.vel = normal_90.scale(Vec2::dot(self.vel, normal)) -
                   normal.scale(Vec2::dot(self.vel, normal));
				self.vel.scale(10.0)
				*/
        let normal = normal.normalize();
        let tangent_90 = Vec2::new(-normal.y, normal.x);
        let i_tangent_90 = Vec2::new(normal.y, -normal.x);
				if Vec2::dot(self.vel, tangent_90) > 0.0 {
					// Positive direction in this tangent, so we should move in this direction
					// I think we need to check the angle on the tangents:
					// 0. Normalize the tangents
					// 1. Find tangent such that vel*tangent>0.0, then we move in:
					//   1. 1 X to the left if vel.y > tangent.y && tangent.x < 0
					//   2. Else move 1 X right if vel.y > tangent.y && tangent.x > 0
					//   3. Move 1 Y down If vel.x > tangent.x && tangent.y < 0
					//   3. Move 1 Y up If vel.x > tangent.x && tangent.y > 0
					// self.vel = tangent_90;
				} else if Vec2::dot(self.vel, i_tangent_90) > 0.0 {
					// self.vel = i_tangent_90;
				}
				(tangent_90.scale(10.0), i_tangent_90.scale(10.0))
    }
}

#[derive(Default)]
pub struct PolygonState {
    current_try: usize,
    vel_backup: Vec2,
    pub collision: bool,
    pub poc: (i32, i32),
    pub debug_vectors: Vec<(Vec2, Vec2)>,
}

impl Collable<u8, PolygonState> for Polygon {
    fn points(&self) -> Points {
        Points::new(Vector(self.pos.x, self.pos.y), &self.points)
    }

    fn queued(&self) -> Vector {
        // Returns velocity vector (new name?)
        Vector(self.vel.x, self.vel.y)
    }

    fn presolve(&mut self, state: &mut PolygonState) {
        state.vel_backup = self.vel;
    }

    fn resolve<I>(&mut self, mut set: TileSet<Tile, I>, state: &mut PolygonState) -> bool
        where I: Iterator<Item = (i32, i32)>
    {
        if set.all(|x| *x == 0) {
            // If there is no collision (we only collide with non-zero tiles)
            self.pos += self.vel;
            true
        } else {
            // There was collision, but our speed isn't tiny
            self.vel = self.vel * 0.9;

            // Find normal
            state.poc = set.get_coords(); // point of collision
            state.collision = true;


            /*
            // Allow gliding on tiles
            if state.current_try == 10 {
                self.vel = Vec2::new(state.vel_backup.x, 0.0);
            } else if state.current_try == 20 {
                self.vel = Vec2::new(0.0, state.vel_backup.y);
            }
            */

            state.current_try += 1;
            false
        }
    }

    fn postsolve(&mut self, collided_once: bool, resolved: bool, state: &mut PolygonState) {
        self.vel = state.vel_backup;
    }
}
