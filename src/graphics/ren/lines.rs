use geometry::vec::Vec2;
use glium;
use glium::{Display, Surface};

const MAX_NUM_VERTICES: usize = 8000;

/// Renderer for general lines in world space - e.g. for debugging vectors etc.
/// The geometry can be updated any time. (TODO)
pub struct Ren {
    geometry: Vec<Vertex>,
    draw_col: (f32, f32, f32),

    prg: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
}

impl Ren {
    pub fn new(display: Display) -> Ren {
        let vert_src = include_str!("../../../shaders/xy_col_tr.vert");
        let frag_src = include_str!("../../../shaders/xy_col_tr.frag");
        let prg = glium::Program::from_source(&display, vert_src, frag_src, None).unwrap();
        let vertex_buffer = glium::VertexBuffer::empty(&display, MAX_NUM_VERTICES).unwrap();
        Ren {
            geometry: Vec::new(),
            draw_col: (0.0, 1.0, 0.0),

            prg: prg,
            vertex_buffer: vertex_buffer,
        }
    }

    pub fn add_line(&mut self, start: Vec2, end: Vec2) {
        self.geometry.push(Vertex {
            pos: [start.x, start.y],
            col: [self.draw_col.0, self.draw_col.1, self.draw_col.2],
        });
        self.geometry.push(Vertex {
            pos: [end.x, end.y],
            col: [self.draw_col.0, self.draw_col.1, self.draw_col.2],
        });
    }
    pub fn add_vector(&mut self, mut start: Vec2, dir: Vec2) {
        let radius: f32 = dir.length() / 15.0;
        let arrow_angle: f32 = 2.7;

        let dir_angle: f32 = f32::atan2(dir.y, dir.x);
        let a1: Vec2 = Vec2::new(
            (dir_angle - arrow_angle).cos() * radius,
            (dir_angle - arrow_angle).sin() * radius,
        );
        let a2: Vec2 = Vec2::new(
            (dir_angle + arrow_angle).cos() * radius,
            (dir_angle + arrow_angle).sin() * radius,
        );

        self.add_line(start, start + dir);
        start += dir;
        self.add_line(start, start + a1);
        self.add_line(start, start + a2);
    }
    /// Clear all geometry.
    pub fn clear(&mut self) {
        self.geometry.clear();
    }
    pub fn set_color(&mut self, col: (f32, f32, f32)) {
        self.draw_col = col;
    }
    fn upload_vertices(&mut self) {
        if self.geometry.is_empty() {
            return;
        }
        // TODO: * VERTEX_SIZE?
        let slice = self.vertex_buffer.slice(0..(self.geometry.len())).unwrap();
        slice.write(&self.geometry);
    }

    pub fn render(
        &mut self,
        target: &mut glium::Frame,
        center: Vec2,
        zoom: f32,
        width: u32,
        height: u32,
    ) {
        self.upload_vertices();
        let uniforms = uniform! {
            proj: super::proj_matrix(width as f32, height as f32, 0.0, 1.0),
            view: super::view_matrix(center.x, center.y, zoom, zoom),
        };
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);
        target
            .draw(
                self.vertex_buffer.slice(0..self.geometry.len()).unwrap(),
                &indices,
                &self.prg,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
    }
}
#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
    col: [f32; 3],
}

implement_vertex!(Vertex, pos, col);
