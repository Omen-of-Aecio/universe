pub mod gen;
pub mod color;
pub mod jump;
pub mod player;
pub mod iter;

use std::vec::Vec;

use glium::glutin::VirtualKeyCode as KeyCode;

use tile_net::TileNet;
use tile_net::Collable;

use global::Tile;
use geometry::polygon::{Polygon, PolygonState};
use geometry::vec::Vec2;
use input::Input;
use world::color::Color;
use world::jump::Jump;
use world::player::Player;
use world::iter::PolygonIter;

const ACCELERATION: f32 = 0.35;

pub struct World {
    pub tilenet: TileNet<Tile>,
    pub player: Player,
    pub exit: bool,
    width: usize,
    height: usize,
    cam_pos: Vec2,
    gravity_on: bool,
    gravity: f32,
    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,
}

impl World {
    pub fn new(width: usize, height: usize, player_pos: Vec2) -> World {
        let shape = Polygon::new_quad(player_pos.x, player_pos.y, 10.0, 10.0, Color::BLACK);
        World {
            tilenet: TileNet::<Tile>::new(width, height),
            player: Player::new(shape),
            exit: false,
            width: width,
            height: height,
            cam_pos: Vec2::new((width/2) as f32, (height/2) as f32),
            gravity_on: false,
            gravity: 0.5,
            vectors: Vec::new(),
        }
    }


    pub fn update(&mut self, input: &Input) {
        self.vectors.clear(); // clear debug geometry
        self.handle_input(input);
        self.update_camera();
        self.player.update(&self.tilenet, if self.gravity_on { self.gravity } else { 0.0 });
    }

    pub fn polygons_iter<'a>(&'a self) -> PolygonIter<'a> {
        PolygonIter::new(self)
    }

    fn handle_input(&mut self, input: &Input) {
        // Ad hoc: input to control first polygon
        if input.key_down(KeyCode::Escape) {
            self.exit = true;
        }
        if input.key_down(KeyCode::Left) || input.key_down(KeyCode::A) || input.key_down(KeyCode::R) {
            self.player.shape.vel.x -= ACCELERATION;
        }
        if input.key_down(KeyCode::Right) || input.key_down(KeyCode::D) || input.key_down(KeyCode::T) {
            self.player.shape.vel.x += ACCELERATION;
        }
        if input.key_down(KeyCode::Up) || input.key_down(KeyCode::W) || input.key_down(KeyCode::F) {
            if self.gravity_on {
                self.player.jump();
            } else {
                self.player.shape.vel.y += ACCELERATION;
            }
        }
        if input.key_down(KeyCode::Down) || input.key_down(KeyCode::S) || input.key_down(KeyCode::S) {
            if !self.gravity_on {
                self.player.shape.vel.y -= ACCELERATION;
            }
        }
        if input.key_toggled_down(KeyCode::G) {
            self.gravity_on = ! self.gravity_on;
        }
    }
    fn update_camera(&mut self) {
        // Camera follows player
        self.cam_pos = self.player.shape.pos;
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
		(&0, Color::BLACK) => 255,
		(&255, Color::BLACK) => 0,
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
