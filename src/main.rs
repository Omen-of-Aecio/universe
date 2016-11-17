extern crate tile_net;
extern crate tilenet_ren;
#[macro_use]
extern crate glium;
extern crate rand;


use std::f32;
use std::thread;

use glium::{DisplayBuild, glutin};
use glium::glutin::{MouseScrollDelta, ElementState, MouseButton};

use input::Input;
use world::World;
use graphics::Graphics;
use geometry::polygon::Polygon;
use geometry::vec::Vec2;


pub mod global;
pub mod world;
pub mod graphics;
pub mod geometry;
pub mod input;

fn main() {
    let mut ctrl: Main = Main::new();
    ctrl.run();
}

const WORLD_SIZE: usize = 1200;

struct Main {
    display: glium::Display,
    input: Input,
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
            self.input.update();
            // Handle input events
            for ev in self.display.clone().poll_events() {
                match ev {
                    glutin::Event::Closed => return,
                    glutin::Event::MouseMoved(x, y) => self.mouse_moved(x, y),
                    glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(_, y), _) => {
                        self.mouse_wheel_line(y)
                    }
                    glutin::Event::MouseInput(ElementState::Pressed, button) => {
                        self.mouse_press(button)
                    }
                    glutin::Event::MouseInput(ElementState::Released, button) => {
                        self.mouse_release(button)
                    }
                    glutin::Event::KeyboardInput(_, _, _) => self.input.register_key(ev),
                    _ => (),
                }
            }

            // Logic
            self.world.update(&self.input);

            // Render
            let window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
            self.graphics.render(self.center.x, self.center.y, self.zoom, window_size.0, window_size.1, &self.world);

            thread::sleep_ms(15);
        }
    }

    fn mouse_moved(&mut self, x: i32, y: i32) {
        self.mouse_pos_past = self.mouse_pos;
        self.mouse_pos = Vec2::new(x as f32, y as f32);
        // Move the texture //
        if self.mouse_down {
            // let window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
            let mut offset = (self.mouse_pos - self.mouse_pos_past) / self.zoom;
            offset.x = -offset.x;
            offset.y = offset.y;
            self.center += offset;
        }
    }

    fn mouse_wheel_line(&mut self, y: f32) {
        // For each 'tick', it should *= factor
        const ZOOM_FACTOR: f32 = 1.2;
        if y > 0.0 {
            self.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
        } else if y < 0.0 {
            self.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
        }
    }

    fn mouse_press(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_down = true,
            _ => (),
        }
    }

    fn mouse_release(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_down = false,
            _ => (),
        }
    }

    fn new() -> Main {
        let mut world = World::new(WORLD_SIZE, WORLD_SIZE);
        // world::gen::rings(&mut world.tilenet, 2);
        world::gen::proc1(&mut world.tilenet);
        world.tilenet.set_box(&0, (0, 0), (100, 100));

        let collider = Polygon::new_quad(50.0, 50.0, 10.0, 10.0);
        world.polygons.push(collider);

        let display = glutin::WindowBuilder::new().build_glium().unwrap();
        let graphics = Graphics::new(display.clone(), &world);
        Main {
            display: display,
            input: Input::new(),
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
