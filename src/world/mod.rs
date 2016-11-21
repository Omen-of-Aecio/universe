pub mod gen;

use std::vec::Vec;

use glium::glutin::VirtualKeyCode;

use tile_net::TileNet;
use tile_net::Collable;

use global::Tile;
use geometry::polygon::{Polygon, PolygonState};
use geometry::vec::Vec2;
use input::Input;

pub struct World {
    pub tilenet: TileNet<Tile>,
    pub polygons: Vec<Polygon>,
    pub exit: bool,
    width: usize,
    height: usize,
    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,
}

impl World {
    pub fn new(width: usize, height: usize) -> World {
        World {
            tilenet: TileNet::<Tile>::new(width, height),
            polygons: Vec::new(),
            exit: false,
            width: width,
            height: height,
            vectors: Vec::new(),
        }
    }


    pub fn update(&mut self, input: &Input) {
        self.vectors.clear(); // clear debug geometry
        // Ad hoc: input to control first polygon
        if input.key_down(VirtualKeyCode::Escape) {
            self.exit = true;
        }
        if input.key_down(VirtualKeyCode::Left) || input.key_down(VirtualKeyCode::A) {
            self.polygons[0].vel.x -= 1.0;
        }
        if input.key_down(VirtualKeyCode::Right) || input.key_down(VirtualKeyCode::D) {
            self.polygons[0].vel.x += 1.0;
        }
        if input.key_down(VirtualKeyCode::Up) || input.key_down(VirtualKeyCode::W) {
            self.polygons[0].vel.y += 1.0;
        }
        if input.key_down(VirtualKeyCode::Down) || input.key_down(VirtualKeyCode::S) {
            self.polygons[0].vel.y -= 1.0;
        }

        // Physics
        for p in &mut self.polygons.iter_mut() {
            let mut polygon_state = PolygonState::default();
            p.solve(&self.tilenet, &mut polygon_state);

            if polygon_state.collision {
                let normal = get_normal(&self.tilenet,
                                        polygon_state.poc.0 as usize,
                                        polygon_state.poc.1 as usize);
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32),
                           normal));
                let (a, b) = p.collide_wall(normal);
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32),
                           a));
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32),
                           b));
            }

            // Add debug vectors
            self.vectors.extend(polygon_state.debug_vectors.iter().cloned());
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
        info!("TileNet"; "content" => format!["{:?}", self.tilenet]);
    }
}
pub fn get_normal(tilenet: &TileNet<Tile>, world_x: usize, world_y: usize) -> Vec2 {
    let kernel = [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]];
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (y, row) in kernel.iter().enumerate() {
        for (x, _) in row.iter().enumerate() {
            if let (Some(x_coord), Some(y_coord)) = ((world_x + x).checked_sub(1),
                                                     (world_y + y).checked_sub(1)) {
                tilenet.get((x_coord, y_coord)).map(|&v| dx += kernel[y][x] * v as f32 / 255.0);
                tilenet.get((x_coord, y_coord)).map(|&v| dy += kernel[x][y] * v as f32 / 255.0);
            }
        }
    }
    Vec2::new(dx, dy)
}
