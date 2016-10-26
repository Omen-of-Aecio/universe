extern crate tile_net;
extern crate tilenet_ren;
#[macro_use]
extern crate glium;

use std::f32::consts::PI;
use std::f32;

use tile_net::TileNet;

use glium::{ DisplayBuild, glutin };
use glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton, WindowBuilder};
use glium::backend::glutin_backend::GlutinFacade;

use world::World;
use graphics::Graphics;
use geometry::polygon::Polygon;
use geometry::vec::Vec2;

use tile_net::Collable;

pub mod global;
pub mod world;
pub mod graphics;
pub mod geometry;

fn main() {
    let mut ctrl: Main = Main::new();
    ctrl.run();
}

const WORLD_SIZE: usize = 1200;

struct Main {

    display: glium::Display,
    graphics: Graphics,
    world: World,

    // Camera & input (for now)
    zoom: f32,
    center: Vec2,
    mouse_down: bool,
    mouse_pos: Vec2,
    mouse_pos_past: Vec2,
}

impl Main {
    fn run(&mut self) {



        loop {
            for ev in self.display.clone().poll_events() {
                match ev {
                    glutin::Event::Closed
                        => return,
                    glutin::Event::MouseMoved(x, y)
                        => self.mouse_moved(x, y),
                    glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(x, y), _)
                        => self.mouse_wheel_line(x, y),
                    glutin::Event::MouseInput(ElementState::Pressed, button)
                        => self.mouse_press(button),
                    glutin::Event::MouseInput(ElementState::Released, button)
                        => self.mouse_release(button), 
                    _ => ()
                }

            }
            // Resolve collision
            // if false {
            // loop {
                // let supercover = collider.tiles();
                // let tiles = world.tiles.collide_set(supercover);
                // if collider.resolve(tiles) {
                  // println!["Able to move"];
                  // break;
                // } else {
                  // println!["Unable to move"];
                // }
            // }
            // }
            let window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
            self.graphics.render(self.center.x, self.center.y, self.zoom, window_size.0, window_size.1);
        }
    }
    fn mouse_moved(&mut self , x: i32, y: i32) {
        self.mouse_pos_past = self.mouse_pos;
        self.mouse_pos = Vec2::new(x as f32, y as f32);
        // Move the texture //
        if self.mouse_down {
            let window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
            let mut offset = (self.mouse_pos - self.mouse_pos_past) / self.zoom;
            offset.x = -offset.x;
            offset.y =  offset.y;
            self.center += offset;
        }
    }

    fn mouse_wheel_line(&mut self, x: f32, y: f32) {
        // For each 'tick', it should *= factor
        const zoom_factor: f32 = 1.2;
        if y > 0.0 {
            self.zoom *= f32::powf(zoom_factor, y as f32);
        } else if y < 0.0 {
            self.zoom /= f32::powf(zoom_factor, -y as f32);
        }
    }

    fn mouse_press(&mut self, button: MouseButton) {
        self.mouse_down = true;
    }

    fn mouse_release(&mut self, button: MouseButton) {
        self.mouse_down = false;
    }

    fn new() -> Main {
        let mut world = World::new(WORLD_SIZE, WORLD_SIZE);
        // world::gen::rings(&mut world.tiles, 2);
        world::gen::proc1(&mut world.tiles);

        let collider = Polygon::new_quad(50.0, 50.0, 10.0, 10.0);
        world.polygons.push(collider);

        let display = glutin::WindowBuilder::new().build_glium().unwrap();
        let graphics = Graphics::new(display.clone(), &world);
        Main {
            display: display,
            graphics: graphics,
            world: world,
            zoom: 1.0,
            center: Vec2::new(0.0, 0.0),
            mouse_down: false,
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_pos_past: Vec2::new(0.0, 0.0),
        }
    }
}



