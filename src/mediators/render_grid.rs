use glium;
use glium::{Display, Surface};
use glium::texture::{Texture2d, ClientFormat, RawImage2d};
use std::borrow::Cow;
use libs::geometry::grid2d::Grid;
use std;

// Re-export for configuration
pub use glium::uniforms::MinifySamplerFilter;
pub use glium::uniforms::MagnifySamplerFilter;

pub struct Renderer {
    net_width: usize,
    net_height: usize,

    // OpenGL
    shader_prg: glium::Program,
    quad_vbo: glium::VertexBuffer<Vertex>,
    texture: Texture2d,
    // Uniforms/config
    bg_col: [f32; 3],
    minify_filter: MinifySamplerFilter,
    magnify_filter: MagnifySamplerFilter,
    smooth: bool,
}

impl Renderer {
    pub fn new(display: &Display, net: &Grid<u8>) -> Renderer {

        let vert_src = include_str!("../../shaders/xyuv_tex.vert");
        let frag_src = include_str!("../../shaders/xyuv_tex.frag");
        let shader_prg = glium::Program::from_source(display, vert_src, frag_src, None).unwrap();
        let fullscreen_quad = vec![Vertex { pos: [-1.0, -1.0] },
                                   Vertex { pos: [1.0, -1.0] },
                                   Vertex { pos: [1.0, 1.0] },

                                   Vertex { pos: [1.0, 1.0] },
                                   Vertex { pos: [-1.0, 1.0] },
                                   Vertex { pos: [-1.0, -1.0] }];

        let quad_vbo = ::glium::VertexBuffer::new(display, &fullscreen_quad).unwrap();
        let texture_data: Vec<Vec<u8>> = vec!(vec!(0; net.get_size().0); net.get_size().1);
        let texture = glium::texture::Texture2d::new(display, texture_data).unwrap();

        let mut new = Renderer {
            net_width: net.get_size().0,
            net_height: net.get_size().1,

            shader_prg: shader_prg,
            quad_vbo: quad_vbo,
            texture: texture,

            bg_col: [0.5, 0.5, 0.5],
            minify_filter: MinifySamplerFilter::Nearest,
            magnify_filter: MagnifySamplerFilter::Nearest,
            smooth: true,
        };
        new.upload_entire_texture(net);
        new
    }
    pub fn set_bg_col(&mut self, r: f32, g: f32, b: f32) {
        self.bg_col = [r, g, b];
    }
    pub fn set_minify_filter(&mut self, filter: MinifySamplerFilter) {
        self.minify_filter = filter;
    }
    pub fn set_magnify_filter(&mut self, filter: MagnifySamplerFilter) {
        self.magnify_filter = filter;
    }
    pub fn set_smooth(&mut self, to: bool) {
        self.smooth = to;
    }
    pub fn get_smooth(&mut self) -> bool{
        self.smooth
    }
    pub fn toggle_smooth(&mut self) {
        self.smooth = !self.smooth;
    }

    pub fn render(&mut self,
                  target: &mut glium::Frame,
                  center: (f32, f32),
                  zoom: f32,
                  width: u32,
                  height: u32) {

        let uniforms = uniform! (
            sampler: glium::uniforms::Sampler::new(&self.texture)
                    .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp)
                    .minify_filter(self.minify_filter)
                    .magnify_filter(self.magnify_filter),
            view_size: [width as f32 / zoom, height as f32 / zoom],
            texsize: [self.net_width as f32, self.net_height as f32],
            screen_center: [center.0, center.1],
            bg_col: self.bg_col,
            smooth_: self.smooth,
        );
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        target.draw(self.quad_vbo.slice(0..6).unwrap(),
                  indices,
                  &self.shader_prg,
                  &uniforms,
                  &Default::default())
            .unwrap();


        // END
    }

    pub fn upload_entire_texture(&mut self, net: &Grid<u8>) {
        let net_size = net.get_size();
        self.upload_texture(net, 0, 0, net_size.0, net_size.1);
    }

    pub fn upload_texture(&mut self, net: &Grid<u8>, left: usize, bottom: usize, width: usize, height: usize) {
        let upload_area = glium::Rect {
            left: left as u32,
            bottom: bottom as u32,
            width: width as u32,
            height: height as u32,
        };

        let mut pixels: Vec<u8> = Vec::new();
        for j in bottom..bottom+height{
            for i in left..left+width {
                pixels.push(*net.get(i, j).unwrap());
            }
        }
        assert!(pixels.len() == (width * height) as usize);

        let upload_data = RawImage2d {
            data: Cow::Borrowed(&pixels),
            width: width as u32,
            height: height as u32,
            format: ClientFormat::U8,
        };


        self.texture.write(upload_area, upload_data);
    }
}

// For rendering
#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);
