use geometry::polygon::Polygon;
use geometry::vec::Vec2;
use world::jump::Jump;

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
}
