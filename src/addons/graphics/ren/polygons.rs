use std::vec::Vec;

use specs;
use specs::Join;

use glium;
use glium::{Display, Surface};

use libs::geometry::cam::Camera;
use glocals::component::*;

/// Renderer for polygons.
/// The polygons are given in the constructor, and never changes. (for now)
pub struct Ren {
    display: Display,
    prg: glium::Program,
}

impl Ren {
    pub fn new(display: Display) -> Ren {
        let vert_src = include_str!("../../../../shaders/xy_tr.vert");
        let frag_src = include_str!("../../../../shaders/xy_tr.frag");
        let prg = glium::Program::from_source(&display, vert_src, frag_src, None).unwrap();

        Ren { display, prg }
    }

    pub fn render(&self, target: &mut glium::Frame, cam: Camera, world: &specs::World) {
        let no_indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        // Every time just reupload everything...

        let (shape, pos, color) = (
            world.read_storage::<Shape>(),
            world.read_storage::<Pos>(),
            world.read_storage::<Color>(),
        );
        for (shape, pos, color) in (&shape, &pos, &color).join() {
            let mut vertices = Vec::new();
            for v in &shape.points {
                vertices.push(Vertex { pos: [v.0, v.1] });
            }
            let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();
            let uniforms = uniform! {
                center: [pos.transl.x, pos.transl.y],
                orientation: pos.angular,
                color: color.to_rgb(),
                proj: super::proj_matrix(cam.width as f32, cam.height as f32, 0.0, 1.0),
                view: super::view_matrix(cam.center.x, cam.center.y, cam.zoom, cam.zoom),
            };
            target
                .draw(
                    &vertex_buffer,
                    &no_indices,
                    &self.prg,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
        }
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);
