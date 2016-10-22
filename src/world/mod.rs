pub mod gen;

use std::vec::Vec;

use tile_net::TileNet;

use global::Tile;
use geometry::polygon::Polygon;

pub struct World {
    pub tiles: TileNet<Tile>,
    pub polygons: Vec<Polygon>,
    width: usize,
    height: usize,
}

impl World {
    pub fn new(width: usize, height: usize) -> World {
        World {
            tiles: TileNet::<Tile>::new(width, height),
            polygons: Vec::new(),
            width: width,
            height: height,
        }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn print(&self) {
        println!("{:?}", self.tiles);
    }
}
