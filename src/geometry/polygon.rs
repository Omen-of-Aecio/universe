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
        let normal = normal.normalize();
        let tangent = Vec2::new(-normal.y, normal.x);
        self.vel = tangent.scale(Vec2::dot(self.vel, tangent)) -
                   normal.scale(Vec2::dot(self.vel, normal));
        (tangent, tangent.scale(-1.0))
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
