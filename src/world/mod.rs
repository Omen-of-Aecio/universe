pub mod gen;

use std::vec::Vec;

use glium::glutin::VirtualKeyCode;

use tile_net::TileNet;
use tile_net::Collable;

use global::Tile;
use geometry::polygon::Polygon;
use input::Input;

pub struct World {
    pub tilenet: TileNet<Tile>,
    pub polygons: Vec<Polygon>,
    width: usize,
    height: usize,
}

impl World {
    pub fn new(width: usize, height: usize) -> World {
        World {
            tilenet: TileNet::<Tile>::new(width, height),
            polygons: Vec::new(),
            width: width,
            height: height,
        }
    }

    pub fn get_normal(world_x: usize, world_y: usize) -> Vec2 {
        static kernel = [[-1.0, 0.0, 1.0],
                         [-2.0, 0.0, 2.0],
                         [-1.0, 0.0, 1.0]];
        let dx = 0;
        let dy = 0;
        for y in 0..3 {
            for x in 0..3 {
                dx += kernel[y][x] * tilenet.get((world_x + x - 1, world_y + y - 1));
                dy += kernel[x][y] * tilenet.get((world_x + x - 1, world_y + y - 1));
            }
        }
        Vec2::new(dx, dy)
    }

    pub fn update(&mut self, input: &Input) {
        // Ad hoc: input to control first polygon
        if input.key_down(VirtualKeyCode::Left) {
            self.polygons[0].vel.x -= 1.0;
        }
        if input.key_down(VirtualKeyCode::Right) {
            self.polygons[0].vel.x += 1.0;
        }
        if input.key_down(VirtualKeyCode::Up) {
            self.polygons[0].vel.y += 1.0;
        }
        if input.key_down(VirtualKeyCode::Down) {
            self.polygons[0].vel.y -= 1.0;
        }

        for p in &mut self.polygons {
            // p.queued = p.vel;
            let mut i = 0;
            const MAX_ITER: i32 = 10;
            p.solve(&self.tilenet);
            // loop {
            // let supercover = p.tiles();
            // let tiles = self.tilenet.collide_set(supercover);
            // if p.resolve(tiles) {
            // break;
            // } else {
            // }
            // i += 1;
            // if i > MAX_ITER {
            // println!("WARNING: max iterations reached.");
            // break;
            // }
            // }
            //
        }
        // Friction
        for p in &mut self.polygons {
            p.vel = p.vel * 0.9;
        }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn print(&self) {
        println!("{:?}", self.tilenet);
    }
}
