use std::f32::consts::PI;
use glium;
use glium::{Display, Surface, DisplayBuild};
use glium::glutin;

use tile_net::TileNet;


pub fn proc1(tiles: &mut TileNet<u8>) { // TODO I'm here
    let display = glutin::WindowBuilder::new().build_glium().unwrap();
    
    let vert_src = include_str!("../../shaders/proc1.vert");
    let frag_src = include_str!("../../shaders/proc1.frag");
    let shader_prg = glium::Program::from_source(&display, vert_src, frag_src, None).unwrap();
    let fullscreen_quad = vec![ Vertex { pos: [-1.0, -1.0]},
                                Vertex { pos: [1.0, -1.0]},
                                Vertex { pos: [1.0, 1.0]},

                                Vertex { pos: [1.0, 1.0]},
                                Vertex { pos: [-1.0, 1.0]},
                                Vertex { pos: [-1.0, -1.0]}];

    let quad_vbo = ::glium::VertexBuffer::new(&display, &fullscreen_quad).unwrap();
    let texture_data: Vec<Vec<u8>> = vec!(vec!(0; tiles.get_size().0); tiles.get_size().1);
    let texture = glium::texture::Texture2d::new(&display, texture_data).unwrap();

    let ebo = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

    let mut fbo = glium::framebuffer::SimpleFrameBuffer::new(&display, &texture).unwrap();

    let uniforms = uniform!{
    };

    fbo.draw(&quad_vbo, &ebo, &shader_prg, &uniforms, &Default::default()).unwrap();
}

#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);



pub fn xor_pattern(tiles: &mut TileNet<u8>) {
    let net_size = tiles.get_size();
    for x in 0..net_size.0 {
        for y in 0..net_size.1 {
            tiles.set(&( (x ^ y) as u8 ), (x, y));
        }
    }
}

pub fn rings(tiles: &mut TileNet<u8>, num_rings: i32) {
    let net_size = tiles.get_size();
    let center = ((net_size.0 as i32)/2, (net_size.1 as i32)/2);
    let sine_freq = 2.0 * PI * (num_rings as f32) / (net_size.0 as f32);
    for x in 0..net_size.0 {
        for y in 0..net_size.1 {
            let dist = (((x as i32 - center.0) as f32).powi(2) + ((y as i32 - center.1) as f32).powi(2)).sqrt();
            let f_value = (dist * sine_freq).sin();
            let i_value = ((f_value >= 0.0) as i32) * 255;
            tiles.set(&(i_value as u8), (x, y));
        }
    }
}
