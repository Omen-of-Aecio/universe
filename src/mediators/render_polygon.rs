use crate::glocals::{PolygonRenderData, Vertex};
use crate::libs::geometry::cam::Camera;
use glium::{self, Display, Surface};
use std::vec::Vec;

pub fn view_matrix(center_x: f32, center_y: f32, scale_x: f32, scale_y: f32) -> [[f32; 4]; 4] {
    [
        [scale_x, 0.0, 0.0, 0.0],
        [0.0, scale_y, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-center_x * scale_x, -center_y * scale_y, 0.0, 1.0],
    ]
}
pub fn proj_matrix(width: f32, height: f32, far: f32, near: f32) -> [[f32; 4]; 4] {
    let width = width as f32;
    let height = height as f32;
    let far = far as f32;
    let near = near as f32;
    [
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, 2.0 / height, 0.0, 0.0],
        [0.0, 0.0, -2.0 / (far - near), 0.0],
        [0.0, 0.0, -(far + near) / (far - near), 1.0],
    ]
}

pub fn create_render_polygon(display: &Display) -> PolygonRenderData {
    let vert_src = include_str!("../../shaders/xy_tr.vert");
    let frag_src = include_str!("../../shaders/xy_tr.frag");
    let prg = glium::Program::from_source(display, vert_src, frag_src, None).unwrap();
    PolygonRenderData { prg }
}

pub fn render(s: &PolygonRenderData, display: &Display, target: &mut glium::Frame, cam: &Camera) {
    let no_indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

    let mut vertices = Vec::new();
    vertices.push(Vertex { pos: [0.0, 0.0] });
    vertices.push(Vertex { pos: [0.0, 10.0] });
    vertices.push(Vertex { pos: [10.0, 10.0] });
    vertices.push(Vertex { pos: [10.0, 0.0] });

    let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();
    let uniforms = uniform! {
        center: [500.0, 350.0] as [f32; 2],
        orientation: 0.0 as f32,
        color: [0.0, 1.0, 0.5] as [f32; 3],
        // center: [pos.transl.x, pos.transl.y],
        // orientation: pos.angular,
        // color: color.to_rgb(),
        proj: proj_matrix(cam.width as f32, cam.height as f32, 0.0, 1.0),
        view: view_matrix(cam.center.x, cam.center.y, cam.zoom, cam.zoom),
    };
    target
        .draw(
            &vertex_buffer,
            &no_indices,
            &s.prg,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
}
