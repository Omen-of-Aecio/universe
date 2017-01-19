use tile_net::*;
use geometry::vec::Vec2;

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
    pub fn new_state(color: Color) -> RayState {
        RayState {
            hit_tile: None,
            color: color,
        }
    }
}

impl Collable for Ray {
    fn points(&self) -> Points {
        Points::new(Vector(self.pos.x, self.pos.y), &(0.0, 0.0))
    }
    fn queued(&self) -> Vector {
        Vector(self.dir.x, self.dir.y)
    }
    fn resolve<I>(&mut self, set: TileSet<Tile, I>, state: &mut RayState) -> bool {
        state.hit_tile = state.get_last();

        let no_collision = match state.color {
            Color::White => set.all(|x| *x == 0),
            Color::Black => set.all(|x| *x != 0),
        };
        state.hit_tile = Some(set.get_last());
        no_collision
    }
}

pub struct RayState {
    pub hit_tile: Option<(usize, usize)>,
    pub color: Color,
}
