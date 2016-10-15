use tile_net::TileNet;

pub type Tile = u8;

pub struct World {
    pub tiles: TileNet<Tile>,
    width: usize,
    height: usize,
}

impl World {
    pub fn new(width: usize, height: usize) -> World {
        World {
            tiles: TileNet::<Tile>::new(width, height),
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
