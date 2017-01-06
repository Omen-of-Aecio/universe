pub mod gen;
pub mod color;
pub mod jump;
pub mod player;
pub mod iter;

use std::vec::Vec;

use glium::glutin::VirtualKeyCode as KeyCode;

use tile_net::TileNet;

use global::Tile;
use geometry::polygon::Polygon;
use geometry::vec::Vec2;
use input::Input;
use world::color::Color;
use world::player::Player;
use world::iter::PolygonIter;

const ACCELERATION: f32 = 0.35;

pub struct World {
    pub tilenet: TileNet<Tile>,
    pub players: Vec<Player>,
    pub exit: bool,

    pub white_base: Vec2,
    pub black_base: Vec2,

    width: usize,
    height: usize,
    cam_pos: Vec2,
    pub gravity_on: bool,
    gravity: Vec2,
    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,
}


impl World {
    pub fn new(width: usize, height: usize, white_base: Vec2, black_base: Vec2) -> World {

        // let shape = Polygon::new_quad(player_pos.x, player_pos.y, 10.0, 10.0, Color::BLACK);
        // let players = vec![ Player::new(shape) ];
        let mut w = World {
            tilenet: TileNet::<Tile>::new(width, height),
            players: Vec::new(),
            exit: false,
            white_base: white_base,
            black_base: black_base,
            width: width,
            height: height,
            cam_pos: Vec2::new((width/2) as f32, (height/2) as f32),
            gravity_on: false,
            gravity: Vec2::new(0.0, -0.5),
            vectors: Vec::new(),
        };

        // Generate TileNet
        gen::proc1(&mut w.tilenet);
        // world::gen::rings(&mut world.tilenet, 2);
        
        // Create bases
        let base_size: usize = 50;

        let pos = (white_base.x as usize, white_base.y as usize);
        w.tilenet.set_box(&0, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));

        let pos = (black_base.x as usize, black_base.y as usize);
        w.tilenet.set_box(&255, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));

        w
    }

    /// Will default_initialize certain fields that aren't needed on client side
    pub fn new_for_client(width: usize, height: usize) -> World {
        World {
            tilenet: TileNet::<Tile>::new(width, height),
            players: Vec::new(),
            exit: false,
            white_base: Vec2::null_vec(),
            black_base: Vec2::null_vec(),
            width: width,
            height: height,
            cam_pos: Vec2::new((width/2) as f32, (height/2) as f32),
            gravity_on: false,
            gravity: Vec2::new(0.0, -0.5),
            vectors: Vec::new(),
        }
    }

    /// Returns index of the new player
    pub fn add_new_player(&mut self, col: Color) -> usize { 
        self.players.push(Player::new(
            match col {
                Color::White => Polygon::new_quad(self.white_base.x, self.white_base.y, 10.0, 10.0, Color::White),
                Color::Black => Polygon::new_quad(self.black_base.x, self.black_base.y, 10.0, 10.0, Color::Black),
            }
        ));
        self.players.len() - 1
    }

    pub fn update(&mut self) {
        self.vectors.clear(); // clear debug geometry
        for player in &mut self.players {
            player.update(&self.tilenet, if self.gravity_on { self.gravity } else { Vec2::null_vec() });
        }
    }

    pub fn polygons_iter<'a>(&'a self) -> PolygonIter<'a> {
        PolygonIter::new(self)
    }

    // Access //
    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }
    pub fn get_cam_pos(&self) -> Vec2 {
        self.cam_pos
    }

    pub fn print(&self) {
        info!("TileNet"; "content" => format!["{:?}", self.tilenet]);
    }
}

pub fn map_tile_value_via_color(tile: &Tile, color: Color) -> Tile {
	match (tile, color) {
		(&0, Color::Black) => 255,
		(&255, Color::Black) => 0,
		_ => *tile,
	}
}

pub fn get_normal(tilenet: &TileNet<Tile>, coord: (usize, usize), color: Color) -> Vec2 {
    let cmap = map_tile_value_via_color;
    /*
    let kernel = match color {
        Color::WHITE => [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]],
        Color::BLACK => [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]],
    };
    */
    let kernel = [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]];
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (y, row) in kernel.iter().enumerate() {
        for (x, _) in row.iter().enumerate() {
            if let (Some(x_coord), Some(y_coord)) = ((coord.0 + x).checked_sub(1),
                                                     (coord.1 + y).checked_sub(1)) {
                tilenet.get((x_coord, y_coord)).map(|&v| dx += kernel[y][x] * cmap(&v, color) as f32 / 255.0);
                tilenet.get((x_coord, y_coord)).map(|&v| dy += kernel[x][y] * cmap(&v, color) as f32 / 255.0);
            }
        }
    }
    Vec2::new(dx, dy)
}
pub fn i32_to_usize(mut from: (i32, i32)) -> (usize, usize) {
    if from.0 < 0 { from.0 = 0; }
    if from.1 < 0 { from.1 = 0; }
    (from.0 as usize, from.1 as usize)
}
