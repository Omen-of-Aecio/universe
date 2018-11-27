use component::Color;
use geometry::vec::Vec2;
use global::Tile;
use tilenet::*;

pub struct RayCollable {
    pos: Vec2,
    dir: Vec2,
    color: Color,

    // Results (?)
    pub collision: bool,
    pub hit_tile: Option<(i32, i32)>,
}

impl RayCollable {
    pub fn new(start_pos: Vec2, direction: Vec2, color: Color) -> RayCollable {
        RayCollable {
            pos: start_pos,
            dir: direction,
            color: color,
            collision: false,
            hit_tile: None,
        }
    }
}
static NULL_VECTOR: &'static [(f32, f32)] = &[(0.0, 0.0)];

impl Collable<Tile> for RayCollable {
    fn queued(&self) -> Vector {
        self.dir.into()
    }
    fn points(&self) -> Points {
        Points::new(Vector(self.pos.x, self.pos.y), NULL_VECTOR)
    }
    fn resolve<I>(&mut self, mut set: TileSet<Tile, I>) -> bool
    where
        I: Iterator<Item = (i32, i32)>,
    {
        let no_collision = match self.color {
            Color::White => set.all(|x| *x == 0),
            Color::Black => set.all(|x| *x != 0),
        };
        self.hit_tile = Some(set.get_last_coord());
        self.collision = !no_collision;
        no_collision
    }
}
