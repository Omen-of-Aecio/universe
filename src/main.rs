extern crate tile_net;
extern crate tilenet_render;
#[macro_use]
extern crate glium;

use tile_net::TileNet;

use glium::{ DisplayBuild, glutin };
use glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton, WindowBuilder};
use glium::backend::glutin_backend::GlutinFacade;

use world::World;
use graphics::Graphics;

pub mod world;
pub mod graphics;


fn main() {
    let mut world = World::new(300, 300);
    xor_pattern(&mut world.tiles);

    let display = glutin::WindowBuilder::new().build_glium().unwrap();
    let mut graphics = Graphics::new(&display, &world);
    loop {
        for ev in display.poll_events() {
            match ev {
                glutin::Event::Closed
                    => return,
                /* glutin::Event::MouseMoved(x, y)
                    => self.mouse_moved(&display, x, y),
                glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(x, y), _)
                    => self.mouse_wheel_line(x, y),
                glutin::Event::MouseInput(ElementState::Pressed, button)
                    => self.mouse_press(button),
                glutin::Event::MouseInput(ElementState::Released, button)
                    => self.mouse_release(button), */
                _ => ()
            }
        }
        let window_size = display.get_window().unwrap().get_inner_size().unwrap();
        graphics.render(-30.0, -30.0, window_size.0, window_size.1);
    }
}

fn xor_pattern(tiles: &mut TileNet<u8>) {
    let tile_size = tiles.get_size();
    for x in 0..tile_size.0 {
        for y in 0..tile_size.1 {
            tiles.set(&( (x ^ y) as u8 ), (x, y));
        }
    }
}
