pub mod gen;
pub mod color;

use std::vec::Vec;

use glium::glutin::VirtualKeyCode;

use tile_net::TileNet;
use tile_net::Collable;

use global::Tile;
use geometry::polygon::{Polygon, PolygonState};
use geometry::vec::Vec2;
use input::Input;
use world::color::Color;

const ACCELERATION: f32 = 0.35;
const SUBJECT_POLYGON: usize = 1;

pub struct World {
    pub tilenet: TileNet<Tile>,
    pub polygons: Vec<Polygon>,
    pub exit: bool,
    width: usize,
    height: usize,
    cam_pos: Vec2,
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
            cam_pos: Vec2::new((width/2) as f32, (height/2) as f32),
            vectors: Vec::new(),
        }
    }


    pub fn update(&mut self, input: &Input) {
        self.vectors.clear(); // clear debug geometry

        self.handle_input(input);

        self.update_camera();

        // Physics
        for p in &mut self.polygons.iter_mut() {
            // - Until we reach end of frame:
            // - Do collision with remaining time
            // - While collision
            //   * move a bit away from the wall
            //   * try to move further
            //   * negate that movement away from wall
            
            // Possible improvements
            // - if it still sometimes gets stuc, maybe use the normal of the next real collision
            //   for moving a unit away, since it's this normal that really signifies the problem
            //   (increases complexity)

			let mut i = 0;
            let mut time_left = 1.0;

            let mut polygon_state = PolygonState::new(time_left, p.vel);
            p.solve(&self.tilenet, &mut polygon_state);

            while polygon_state.collision && time_left > 0.1 && i<= 10 {
                let normal = get_normal(&self.tilenet, i32_to_usize(polygon_state.poc), p.color);
                assert!( !(normal.x == 0.0 && normal.y == 0.0));

                // Physical response
                let (a, b) = p.collide_wall(normal);

                // Debug vectors
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32),
                           normal));
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32), a));
                self.vectors
                    .push((Vec2::new(polygon_state.poc.0 as f32, polygon_state.poc.1 as f32), b));

                // Move away one unit from wall
                let mut moveaway_state = PolygonState::new(1.0, normal.normalize());
                p.solve(&self.tilenet, &mut moveaway_state);

                // Try to move further with the current velocity
                polygon_state = PolygonState::new(time_left, p.vel);
                p.solve(&self.tilenet, &mut polygon_state);

                // Move back one unit
                let mut moveback_state = PolygonState::new(1.0, normal.normalize().scale(-1.0));
                p.solve(&self.tilenet, &mut moveback_state);


                i += 1;
                time_left -= polygon_state.toc;
            }

            if polygon_state.collision {
                // One last physical response for the last collision
                let normal = get_normal(&self.tilenet, i32_to_usize(polygon_state.poc), p.color);
                let _ = p.collide_wall(normal);
            }

            debug!("Position in world, "; "x" => p.pos.x, "y" => p.pos.y);

            // Add debug vectors
            self.vectors.extend(polygon_state.debug_vectors.iter().cloned());
        }
        // Friction
        for p in &mut self.polygons {
            p.vel = p.vel * 0.9;
        }

    }

    fn handle_input(&mut self, input: &Input) {
        // Ad hoc: input to control first polygon
        if input.key_down(VirtualKeyCode::Escape) {
            self.exit = true;
        }
        if input.key_down(VirtualKeyCode::Left) || input.key_down(VirtualKeyCode::A) || input.key_down(VirtualKeyCode::R) {
            self.polygons[SUBJECT_POLYGON].vel.x -= ACCELERATION;
        }
        if input.key_down(VirtualKeyCode::Right) || input.key_down(VirtualKeyCode::D) || input.key_down(VirtualKeyCode::T) {
            self.polygons[SUBJECT_POLYGON].vel.x += ACCELERATION;
        }
        if input.key_down(VirtualKeyCode::Up) || input.key_down(VirtualKeyCode::W) || input.key_down(VirtualKeyCode::F) {
            self.polygons[SUBJECT_POLYGON].vel.y += ACCELERATION;
        }
        if input.key_down(VirtualKeyCode::Down) || input.key_down(VirtualKeyCode::S) || input.key_down(VirtualKeyCode::S) {
            self.polygons[SUBJECT_POLYGON].vel.y -= ACCELERATION;
        }
    }
    fn update_camera(&mut self) {
        // Camera follows SUBJECT_POLYGON
        self.cam_pos = self.polygons[SUBJECT_POLYGON].pos;
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
pub fn get_normal(tilenet: &TileNet<Tile>, coord: (usize, usize), color: Color) -> Vec2 {
    let kernel = match color {
        Color::WHITE => [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]],
        Color::BLACK => [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]],
    };
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (y, row) in kernel.iter().enumerate() {
        for (x, _) in row.iter().enumerate() {
            if let (Some(x_coord), Some(y_coord)) = ((coord.0 + x).checked_sub(1),
                                                     (coord.1 + y).checked_sub(1)) {
                tilenet.get((x_coord, y_coord)).map(|&v| dx += kernel[y][x] * v as f32 / 255.0);
                tilenet.get((x_coord, y_coord)).map(|&v| dy += kernel[x][y] * v as f32 / 255.0);
            }
        }
    }
    Vec2::new(dx, dy)
}
fn i32_to_usize(mut from: (i32, i32)) -> (usize, usize) {
    if from.0 < 0 { from.0 = 0; }
    if from.1 < 0 { from.1 = 0; }
    (from.0 as usize, from.1 as usize)
}
