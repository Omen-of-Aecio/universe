use std::vec::Vec;

use glium;
use glium::{Display, Surface};

use geometry::polygon::Polygon;
use geometry::vec::Vec2;

const VERTEX_SIZE: i32 = 2; // xy


pub struct Ren {
    display: Display,
    end_indices: Vec<usize>,
    pos: Vec<Vec2>,
    ori: Vec<f32>,
    // OpenGL
    vertex_buffer: glium::VertexBuffer<Vertex>,
    prg: glium::Program,
}

impl Ren {
    pub fn new(display: Display, polygons: &Vec<Polygon>) -> Ren {
        let mut end_indices = Vec::new();
        let mut pos = Vec::new();
        let mut ori = Vec::new();


        let vert_src = include_str!("../../../shaders/xy_tr.vert");
        let frag_src = include_str!("../../../shaders/xy_tr.frag");
        let prg = glium::Program::from_source(&display, vert_src, frag_src, None).unwrap();
        let mut vertices = Vec::new();
        //// Upload vertices
        for p in polygons {
            for v in &p.points { // v: (f32, f32)
                vertices.push(Vertex{pos: [v.0, v.1]});
                print!("{}, {}\n", v.0, v.1);
            }
            end_indices.push(vertices.len() - 1);
            pos.push(p.pos);
            ori.push(p.ori);
        }
        let vertex_buffer = glium::VertexBuffer::new(&display, &vertices).unwrap();

        Ren {
            display: display,
            end_indices: end_indices,
            pos: pos,
            ori: ori,
            vertex_buffer: vertex_buffer,
            prg: prg,
        }

    }

    pub fn render(&self, target: &mut glium::Frame, center_x: f32, center_y: f32, zoom: f32, width: u32, height: u32) {
        let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);
        for i in 0..self.end_indices.len() {

            let uniforms = uniform! {
                center: [self.pos[i].x, self.pos[i].y],
                orientation: self.ori[i],
                color: [0.5, 0.5, 0.5],
                proj: super::proj_matrix(width as f32, height as f32, 0.0, 1.0),
                view: super::view_matrix(center_x, center_y, zoom, zoom),
            };

            target.draw(&self.vertex_buffer, &index_buffer, &self.prg, &uniforms, &Default::default()).unwrap();
        }
    }
}


#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);
