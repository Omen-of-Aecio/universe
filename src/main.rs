#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate bgjk;
#[macro_use]
extern crate glium;
extern crate isatty;
extern crate rand;
#[macro_use (o, slog_log, slog_trace, slog_debug, slog_info, slog_warn, slog_error)]
extern crate slog;
extern crate slog_json;
#[macro_use]
extern crate slog_scope;
extern crate slog_stream;
extern crate slog_term;
extern crate tile_net;
extern crate tilenet_ren;
extern crate time;

pub mod geometry;
pub mod global;
pub mod graphics;
pub mod input;
pub mod world;

use bgjk::{bgjk, Vec3};
use geometry::polygon::Polygon;
use geometry::vec::Vec2;
use glium::{DisplayBuild, glutin};
use glium::glutin::{ElementState, MouseButton, MouseScrollDelta};
use graphics::Graphics;
use graphics::screen_to_world;
use input::Input;
use slog::{DrainExt, Level};
use std::f32;
use world::World;
use world::color::Color;
use std::thread;
use std::time::Duration;

fn setup_logger() {
    let logger = if isatty::stderr_isatty() {
        let drain = slog_term::streamer()
            .async()
            .stderr()
            .full()
            .use_utc_timestamp()
            .build();
        let d = slog::level_filter(Level::Debug, drain);
        slog::Logger::root(d.fuse(), o![])
    } else {
        slog::Logger::root(slog_stream::stream(std::io::stderr(), slog_json::default()).fuse(),
                           o![])
    };
    slog_scope::set_global_logger(logger);
}

fn main() {
    setup_logger();
    info!["Logger initialized"];

    if false
    {
		/*
		Imagine a slope and a plane:
		\__I_
		where I is the character.
		Walking left needs to attach you to the slope.

		When colliding with an object:
			1. Check if it has a 'sticky' property
			2. Use sticky to compute position (hover slightly)
			3. When outside of domain, cause obj to fall

		This solves the cases:
		__
		  \__

		At the top of the hill. Walk right. Gravity pulls you into the slope. Slope function activates.
		Walk left, slope function unsticks, fall into plane.
		At bottom, same idea.

		Could use function to compute next wall function
		*/
		let slope = [
			Vec3(0.0, 0.0, 0.0),
			Vec3(1.0, 1.0, 0.0),
			Vec3(0.0, 0.0, 1.0),
			Vec3(1.0, 1.0, 1.0),
		];
		let movement = [
			Vec3(0.5, 0.6, 0.0),
			Vec3(0.599, 0.6, 1.0),
		];
		print!["kek\n"];
		for _ in 1..10000 {
			info!["Collision"; "bgjk" => bgjk(&slope, &movement)];
		}
		return;
    }

    let mut ctrl: Main = Main::new();
    ctrl.run();
}

const WORLD_SIZE: usize = 1200;

/* Should go, together with some logic, to some camera module (?) */
enum CameraMode {
    Interactive,
    FollowPlayer,
}

struct Main {
    display: glium::Display,
    input: Input,
    graphics: Graphics,
    world: World,

    // Camera & input (for now)
    cam_mode: CameraMode,
    //   following is used only if INTERACTIVE camera mode
    zoom: f32,
    center: Vec2,
    mouse_down: bool,
    mouse_pos: Vec2,
    mouse_pos_past: Vec2,
}



impl Main {
    fn run(&mut self) {
        let mut window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
        let mut oldpos = Vec2::null_vec();
        let mut tileren_smooth = true;
        while !self.world.exit {
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
                    glutin::Event::Resized(w, h) => window_size = (w, h),
                    _ => (),
                }
            }

            // Logic
            prof!["Logic", self.world.update(&self.input)];

            // Some interactivity for debugging
            if self.input.key_down(glutin::VirtualKeyCode::Comma) && self.input.key_toggled(glutin::VirtualKeyCode::Comma) {
                self.graphics.tilenet_renderer.toggle_smooth();
            }

            // Render
            let cam_pos = match self.cam_mode {
                CameraMode::Interactive => self.center,
                CameraMode::FollowPlayer => self.world.get_cam_pos(),
            };
            prof!["Render",
                  self.graphics.render(cam_pos,
                                       self.zoom,
                                       window_size.0,
                                       window_size.1,
                                       &self.world)];

            // TEST screen to world.
            let pos = screen_to_world(self.mouse_pos,
                                      Vec2::new(self.center.x, self.center.y),
                                      self.zoom,
                                      window_size.0,
                                      window_size.1);

            if pos != oldpos {
                debug!["Position in world"; "x" => pos.x, "y" => pos.y];
            }
            oldpos = pos;

            
            // vsync doesn't seem to work on Windows
            thread::sleep(Duration::from_millis(15));
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
        if let MouseButton::Left = button {
            self.mouse_down = true;
        }
    }

    fn mouse_release(&mut self, button: MouseButton) {
        if let MouseButton::Left = button {
            self.mouse_down = false;
        }
    }

    fn new() -> Main {
        let mut world = World::new(WORLD_SIZE, WORLD_SIZE);
        world::gen::proc1(&mut world.tilenet);

        let pos = (50, WORLD_SIZE/3);
        world.polygons.push(Polygon::new_quad(pos.0 as f32, pos.1 as f32, 10.0, 10.0, Color::WHITE));
        world.tilenet.set_box(&0, (pos.0-50, pos.1-50), (pos.0+50, pos.1+50));

        let pos = (WORLD_SIZE - 50, WORLD_SIZE/3);
        world.polygons.push(Polygon::new_quad(pos.0 as f32, pos.1 as f32, 10.0, 10.0, Color::BLACK));
        world.tilenet.set_box(&255, (pos.0-50, pos.1-50), (pos.0+50, pos.1+50));

        let display = glutin::WindowBuilder::new().build_glium().unwrap();
        let graphics = Graphics::new(display.clone(), &world);
        Main {
            display: display,
            input: Input::new(),
            graphics: graphics,
            world: world,
            cam_mode: CameraMode::FollowPlayer,
            zoom: 1.0,
            center: Vec2::new(0.0, 0.0),
            mouse_down: false,
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_pos_past: Vec2::new(0.0, 0.0),
        }
    }
}
