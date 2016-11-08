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

    pub fn update(&mut self, input: &Input) {
        // Ad hoc: input to control first polygon
        if input.key_down(VirtualKeyCode::Left) {
            self.polygons[0].vel.x += 1.0;
        }

        for p in &mut self.polygons {
            let supercover = p.tiles();
            let tiles = self.tilenet.collide_set(supercover);
            if p.resolve(tiles) {
                // println!["Able to move"];
            } else {
                // println!["Unable to move"];
            }
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
