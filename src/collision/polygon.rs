use component::*;
use geometry::vec::Vec2;
use global::Tile;
use tilenet::*;

pub struct PolygonCollable<'a> {
    pub shape: &'a Shape,
    pub color: &'a Color,
    pub pos: &'a mut Pos,

    // State for collision alg
    pub queued_vel: Vec2,
    current_try: usize,

    /// Resulting collision
    pub collision: bool,
    /// Resulting oint of collision
    pub poc: (i32, i32),
    /// Resulting time of collision - between this and next frame
    pub toc: f32,

    pub debug_vectors: Vec<(Vec2, Vec2)>,
}

impl<'a> PolygonCollable<'a> {
    /// start_toc is what fraction of the velocity we start the algorithm with
    pub fn new(
        shape: &'a Shape,
        color: &'a Color,
        pos: &'a mut Pos,
        vel: Vec2,
        start_toc: f32,
    ) -> PolygonCollable<'a> {
        PolygonCollable {
            shape: shape,
            color: color,
            pos: pos,

            queued_vel: vel,
            current_try: 0,

            collision: false,
            toc: start_toc,
            poc: (0, 0),
            debug_vectors: Vec::new(),
        }
    }

    //TODO REWRITE - notice that this is came from `Polygon`!
    /*
    /// Physical response to collision - i.e. bounce in direction of normal
    pub fn collide_wall(&mut self, normal: Vec2) {
        const RESTITUTION: f32 = 0.0;
        let normal = normal.normalize();
        let tangent = Vec2::new(-normal.y, normal.x);
        self.vel.pos = tangent.scale_uni(Vec2::dot(self.vel, tangent)) +
                   normal.scale_uni(Vec2::dot(self.vel, normal).abs() * RESTITUTION);
    }
    */
}

impl<'a> Collable<Tile> for PolygonCollable<'a> {
    fn points(&self) -> Points {
        Points::new(
            Vector(self.pos.transl.x, self.pos.transl.y),
            &self.shape.points,
        )
    }
    fn queued(&self) -> Vector {
        (self.queued_vel * self.toc).into()
    }

    // fn presolve(&mut self) { }

    fn resolve<I>(&mut self, mut set: TileSet<Tile, I>) -> bool
    where
        I: Iterator<Item = (i32, i32)>,
    {
        let no_collision = match *self.color {
            Color::White => set.all(|x| *x == 0),
            Color::Black => set.all(|x| *x != 0),
        };
        if no_collision {
            // If there is no collision (we only collide with non-zero tiles)
            self.pos.transl += Vec2::from(self.queued());
            true
        } else {
            // Collision.

            self.collision = true;
            self.poc = set.get_last_coord();
            self.toc *= 0.9;
            self.current_try += 1;

            false
        }
    }

    // fn postsolve(&mut self, _collided_once: bool, _resolved: bool) { }
}
