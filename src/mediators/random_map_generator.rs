use glium;
use glium::glutin;
use glium::{DisplayBuild, Surface};
use rand::Rng;
use std::f32::consts::PI;

use crate::libs::geometry::grid2d::Grid;

static vert_src: &str = include_str!("../../shaders/proc1.vert");
static frag_src: &str = include_str!("../../shaders/proc1.frag");
pub fn proc1(tiles: &mut Grid<u8>, mut display: &glium::Display) {
    let mut rng = rand::thread_rng();

    let shader_prg = glium::Program::from_source(display, vert_src, frag_src, None).unwrap();
    let fullscreen_quad = vec![
        Vertex { pos: [-1.0, -1.0] },
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0, 1.0] },
        Vertex { pos: [1.0, 1.0] },
        Vertex { pos: [-1.0, 1.0] },
        Vertex { pos: [-1.0, -1.0] },
    ];

    let quad_vbo = ::glium::VertexBuffer::new(display, &fullscreen_quad).unwrap();
    let texture_data: Vec<Vec<u8>> = vec![vec![0; tiles.get_size().0]; tiles.get_size().1];
    let texture = glium::texture::Texture2d::new(display, texture_data).unwrap();

    let ebo = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

    let mut fbo = glium::framebuffer::SimpleFrameBuffer::new(display, &texture).unwrap();

    let uniforms = uniform! {
        width: tiles.get_size().0 as f32,
        // Different world on each run:
        rand_seed: [rng.gen::<f32>(),rng.gen::<f32>(),rng.gen::<f32>()]
    };

    fbo.draw(&quad_vbo, &ebo, &shader_prg, &uniforms, &Default::default())
        .unwrap();

    // Download map from GPU.
    let texture_data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read();
    // debug!("texture data size"; "x" => texture_data.len(), "y" => texture_data[0].len());
    for (i, texdata) in texture_data.iter().enumerate() {
        for (j, texdata) in texdata.iter().enumerate() {
            *tiles.get_mut(i, j).unwrap() = texdata.0;
        }
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);

pub fn xor_pattern(tiles: &mut Grid<u8>) {
    let net_size = tiles.get_size();
    for x in 0..net_size.0 {
        for y in 0..net_size.1 {
            *tiles.get_mut(x, y).unwrap() = (x ^ y) as u8;
        }
    }
}

pub fn rings(tiles: &mut Grid<u8>, num_rings: i32) {
    let net_size = tiles.get_size();
    let center = ((net_size.0 as i32) / 2, (net_size.1 as i32) / 2);
    let sine_freq = 2.0 * PI * (num_rings as f32) / (net_size.0 as f32);
    for x in 0..net_size.0 {
        for y in 0..net_size.1 {
            let dist = (((x as i32 - center.0) as f32).powi(2)
                + ((y as i32 - center.1) as f32).powi(2))
            .sqrt();
            let f_value = (dist * sine_freq).sin();
            let i_value = ((f_value >= 0.0) as i32) * 255;
            *tiles.get_mut(x, y).unwrap() = i_value as u8;
        }
    }
}
