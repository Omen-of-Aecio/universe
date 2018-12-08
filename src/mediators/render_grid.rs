use crate::glocals::{GridU8RenderData, Vertex};
use crate::libs::geometry::grid2d::Grid;
use glium;
use glium::texture::{ClientFormat, RawImage2d};
use glium::{uniform, Display, Surface};
use std::borrow::Cow;

// Re-export for configuration
pub use glium::uniforms::MagnifySamplerFilter;
pub use glium::uniforms::MinifySamplerFilter;

pub fn create_grid_u8_render_data(display: &Display, net: &Grid<u8>) -> GridU8RenderData {
    let vert_src = include_str!("../../shaders/xyuv_tex.vert");
    let frag_src = include_str!("../../shaders/xyuv_tex.frag");
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
    let texture_data: Vec<Vec<u8>> = vec![vec![0; net.get_size().0]; net.get_size().1];
    let texture = glium::texture::Texture2d::new(display, texture_data).unwrap();

    let mut new = GridU8RenderData {
        net_width: net.get_size().0,
        net_height: net.get_size().1,

        shader_prg: shader_prg,
        quad_vbo: quad_vbo,
        texture: texture,

        bg_col: [0.5, 0.5, 0.5],
        minify_filter: MinifySamplerFilter::Nearest,
        magnify_filter: MagnifySamplerFilter::Nearest,
        smooth: false,
    };
    upload_entire_texture(&mut new, net);
    new
}
pub fn set_bg_col(s: &mut GridU8RenderData, r: f32, g: f32, b: f32) {
    s.bg_col = [r, g, b];
}

pub fn set_minify_filter(s: &mut GridU8RenderData, filter: MinifySamplerFilter) {
    s.minify_filter = filter;
}

pub fn set_magnify_filter(s: &mut GridU8RenderData, filter: MagnifySamplerFilter) {
    s.magnify_filter = filter;
}

pub fn set_smooth(s: &mut GridU8RenderData, to: bool) {
    s.smooth = to;
}

pub fn get_smooth(s: &mut GridU8RenderData) -> bool {
    s.smooth
}

pub fn toggle_smooth(s: &mut GridU8RenderData) {
    s.smooth = !s.smooth;
}

pub fn render(
    s: &mut GridU8RenderData,
    target: &mut glium::Frame,
    center: (f32, f32),
    zoom: f32,
    width: u32,
    height: u32,
) {
    let uniforms = uniform! (
        sampler: glium::uniforms::Sampler::new(&s.texture)
                .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp)
                .minify_filter(s.minify_filter)
                .magnify_filter(s.magnify_filter),
        view_size: [width as f32 / zoom, height as f32 / zoom],
        texsize: [s.net_width as f32, s.net_height as f32],
        screen_center: [center.0, center.1],
        bg_col: s.bg_col,
        smooth_: s.smooth,
    );
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
    target
        .draw(
            s.quad_vbo.slice(0..6).unwrap(),
            indices,
            &s.shader_prg,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
}

pub fn upload_entire_texture(s: &mut GridU8RenderData, net: &Grid<u8>) {
    let net_size = net.get_size();
    upload_texture(s, net, 0, 0, net_size.0, net_size.1);
}

pub fn upload_texture(
    s: &mut GridU8RenderData,
    net: &Grid<u8>,
    left: usize,
    bottom: usize,
    width: usize,
    height: usize,
) {
    let upload_area = glium::Rect {
        left: left as u32,
        bottom: bottom as u32,
        width: width as u32,
        height: height as u32,
    };

    let mut pixels: Vec<u8> = Vec::new();
    for j in bottom..bottom + height {
        for i in left..left + width {
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

    s.texture.write(upload_area, upload_data);
}
