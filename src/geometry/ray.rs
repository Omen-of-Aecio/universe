use tile_net::*;
use geometry::vec::Vec2;
use world::color::Color;
use global::Tile;

pub struct Ray {
    pos: Vec2,
    dir: Vec2,
}

impl Ray {
    pub fn new(start_pos: Vec2, direction: Vec2) -> Ray {
        Ray {
            pos: start_pos,
            dir: direction,
        }
    }
    pub fn new_state(&self, color: Color) -> RayState {
        RayState {
            collision: false,
            hit_tile: None,
            color: color,
            dir: self.dir,
        }
    }
}
static a: &'static [(f32, f32)] = &[(0.0, 0.0)];

impl Collable<Tile, RayState> for Ray {
    fn points(&self) -> Points {
        Points::new(Vector(self.pos.x, self.pos.y), a)
    }
    fn resolve<I>(&mut self, mut set: TileSet<Tile, I>, state: &mut RayState) -> bool
        where I: Iterator<Item = (i32, i32)>
    {

        let no_collision = match state.color {
            Color::White => set.all(|x| *x == 0),
            Color::Black => set.all(|x| *x != 0),
        };
        state.hit_tile = Some(set.get_coords());
        no_collision
    }
}

pub struct RayState {
    pub collision: bool,
    pub hit_tile: Option<(i32, i32)>,
    pub color: Color,
    pub dir: Vec2,
}

impl CollableState for RayState {
    fn queued(&self) -> Vector {
        self.dir.into()
    }
}
