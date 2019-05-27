//! Methods and types to control debug triangles
//!
//! A debug triangle is a triangle that ignores all transformations and is always shown on the
//! screen (except when a triangle's coordinates are outisde the screen). Debug triangles are meant
//! to be used to quickly find out if a state has been reached (for instance, change the color of a
//! debug triangle if collision is detected).
//!
//! Debug triangles always ignore all layers, and are always shown on top of the entire scene.
//!
//! See [debtri::Debtri] for all operations supported on debug triangles.
//! ```
//! use cgmath::{prelude::*, Deg, Matrix4};
//! use logger::{Generic, GenericLogger, Logger};
//! use vxdraw::{ShowWindow, VxDraw};
//! fn main() {
//!     let mut vx = VxDraw::new(Logger::<Generic>::spawn_test().to_logpass(),
//!         ShowWindow::Headless1k); // Change this to ShowWindow::Enable to show the window
//!
//!     let tri = vx.debtri().push(vxdraw::debtri::DebugTriangle::default());
//!
//!     // Turn the triangle white
//!     vx.debtri().set_color(&tri, [255, 255, 255, 255]);
//!
//!     // Rotate the triangle 90 degrees (counter clockwise)
//!     vx.debtri().set_rotation(&tri, Deg(90.0));
//!
//!     // Draw the frame with the identity matrix transformation (meaning no transformations)
//!     vx.draw_frame(&Matrix4::identity());
//!
//!     // Sleep here so the window does not instantly disappear
//!     std::thread::sleep(std::time::Duration::new(3, 0));
//! }
use super::utils::*;
use crate::data::{DebugTriangleData, VxDraw};
use cgmath::Rad;
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
use gfx_hal::{device::Device, format, image, pass, pso, Adapter, Backend, Primitive};
use std::mem::ManuallyDrop;

// ---

/// Debug triangles accessor object returned by [VxDraw::debtri]
///
/// Merely used for grouping together all operations on debug triangles. This is a very cheap
/// object to create/destroy (it really does nothing).
pub struct Debtri<'a> {
    vx: &'a mut VxDraw,
}

impl<'a> Debtri<'a> {
    /// Spawn the accessor object from [VxDraw].
    ///
    /// This is a very cheap operation.
    pub fn new(vx: &'a mut VxDraw) -> Self {
        Self { vx }
    }

    /// Enable drawing of the debug triangles
    pub fn show(&mut self) {
        self.vx.debtris.hidden = false;
    }

    /// Disable drawing of the debug triangles
    pub fn hide(&mut self) {
        self.vx.debtris.hidden = true;
    }

    /// Add a new debug triangle to the renderer
    ///
    /// The new triangle will be drawn upon the next draw.
    pub fn push(&mut self, triangle: DebugTriangle) -> Handle {
        let s = &mut *self.vx;
        let debtris = &mut s.debtris;

        debtris.posbuffer.push(triangle.origin[0].0);
        debtris.posbuffer.push(triangle.origin[0].1);

        debtris.posbuffer.push(triangle.origin[1].0);
        debtris.posbuffer.push(triangle.origin[1].1);

        debtris.posbuffer.push(triangle.origin[2].0);
        debtris.posbuffer.push(triangle.origin[2].1);

        for col in &triangle.colors_rgba {
            debtris.colbuffer.push(col.0);
            debtris.colbuffer.push(col.1);
            debtris.colbuffer.push(col.2);
            debtris.colbuffer.push(col.3);
        }

        for _ in 0..3 {
            debtris.tranbuffer.push(triangle.translation.0);
            debtris.tranbuffer.push(triangle.translation.1);
        }

        for _ in 0..3 {
            debtris.rotbuffer.push(triangle.rotation);
        }

        for _ in 0..3 {
            debtris.scalebuffer.push(triangle.scale);
        }

        debtris.posbuf_touch = s.swapconfig.image_count;
        debtris.colbuf_touch = s.swapconfig.image_count;
        debtris.tranbuf_touch = s.swapconfig.image_count;
        debtris.rotbuf_touch = s.swapconfig.image_count;
        debtris.scalebuf_touch = s.swapconfig.image_count;

        debtris.triangles_count += 1;
        Handle(debtris.triangles_count - 1)
    }

    /// Remove the last added debug triangle from rendering
    ///
    /// Has no effect if there are no debug triangles.
    pub fn pop(&mut self) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        debtris.triangles_count = debtris.triangles_count.checked_sub(1).unwrap_or(0);
    }

    /// Remove the last N added debug triangle from rendering
    ///
    /// If the amount to pop is bigger than the amount of debug triangles, then all debug triangles
    /// wil be removed.
    pub fn pop_many(&mut self, n: usize) {
        let end = self.vx.debtris.triangles_count;
        let begin = end.checked_sub(n).unwrap_or(0);

        let debtris = &mut self.vx.debtris;
        debtris.posbuffer.drain(begin * 6..end * 6);
        debtris.colbuffer.drain(begin * 12..end * 12);
        debtris.tranbuffer.drain(begin * 6..end * 6);
        debtris.rotbuffer.drain(begin * 3..end * 3);
        debtris.scalebuffer.drain(begin * 3..end * 3);

        debtris.triangles_count = begin;
    }

    // ---

    /// Change the vertices of the model-space
    pub fn set_vertices(&mut self, inst: &Handle, points: [(f32, f32); 3]) {
        self.vx.debtris.posbuf_touch = self.vx.swapconfig.image_count;
        unimplemented![]
    }

    /// Set a solid color of a debug triangle
    pub fn set_color(&mut self, inst: &Handle, rgba: [u8; 4]) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        debtris.colbuf_touch = vx.swapconfig.image_count;

        let idx = inst.0 * 12;
        for vtx in 0..3 {
            for (coli, cmpnt) in rgba.iter().enumerate() {
                debtris.colbuffer[idx + vtx * 4 + coli] = *cmpnt;
            }
        }
    }

    /// Set the position of a debug triangle
    pub fn set_position(&mut self, inst: &Handle, pos: (f32, f32)) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        debtris.tranbuf_touch = vx.swapconfig.image_count;

        let idx = inst.0 * 3 * 2;
        for vtx in 0..3 {
            debtris.tranbuffer[idx + vtx * 2] = pos.0;
            debtris.tranbuffer[idx + vtx * 2 + 1] = pos.1;
        }
    }

    /// Set the rotation of a debug triangle
    pub fn set_rotation<T: Copy + Into<Rad<f32>>>(&mut self, inst: &Handle, deg: T) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        let angle = deg.into().0;
        debtris.rotbuf_touch = vx.swapconfig.image_count;
        debtris.rotbuffer[inst.0 * 3..(inst.0 + 1) * 3].copy_from_slice(&[angle, angle, angle]);
    }

    /// Set the scale of a debug triangle
    pub fn set_scale(&mut self, inst: &Handle, scale: f32) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        debtris.scalebuf_touch = vx.swapconfig.image_count;

        for sc in &mut debtris.scalebuffer[inst.0 * 3..(inst.0 + 1) * 3] {
            *sc = scale;
        }
    }

    // ---

    /// Translate a debug triangle by a vector
    ///
    /// Translation does not mutate the model-space of a triangle.
    pub fn translate(&mut self, handle: &Handle, delta: (f32, f32)) {
        self.vx.debtris.tranbuf_touch = self.vx.swapconfig.image_count;

        for stride in 0..3 {
            self.vx.debtris.tranbuffer[handle.0 * 6 + stride * 2] += delta.0;
            self.vx.debtris.tranbuffer[handle.0 * 6 + stride * 2 + 1] += delta.1;
        }
    }

    /// Rotate all debug triangles
    ///
    /// Rotation does not mutate the model-space of a triangle.
    pub fn rotate<T: Copy + Into<Rad<f32>>>(&mut self, handle: &Handle, deg: T) {
        let vx = &mut *self.vx;
        let debtris = &mut vx.debtris;
        debtris.rotbuf_touch = vx.swapconfig.image_count;

        for rot in &mut debtris.rotbuffer[handle.0 * 3..(handle.0 + 1) * 3] {
            *rot += deg.into().0;
        }
    }

    // ---

    /// Translate all debug triangles by a vector
    ///
    /// Adds the translation in the argument to the existing translation of each triangle.
    /// See [translate] for more information.
    pub fn translate_all(&mut self, delta: (f32, f32)) {
        self.vx.debtris.tranbuf_touch = self.vx.swapconfig.image_count;
        for trn in self.vx.debtris.tranbuffer.chunks_exact_mut(2) {
            trn[0] += delta.0;
            trn[1] += delta.1;
        }
    }

    /// Scale all debug triangles (multiplicative)
    ///
    /// Multiplies the scale in the argument with the existing scale of each triangle.
    /// See [rotate] for more information.
    pub fn scale_all(&mut self, scale: f32) {
        self.vx.debtris.scalebuf_touch = self.vx.swapconfig.image_count;
        for sc in &mut self.vx.debtris.scalebuffer {
            *sc *= scale;
        }
    }

    /// Rotate all debug triangles
    ///
    /// Adds the rotation in the argument to the existing rotation of each triangle.
    /// See [rotate] for more information.
    pub fn rotate_all<T: Copy + Into<Rad<f32>>>(&mut self, rotation: T) {
        self.vx.debtris.rotbuf_touch = self.vx.swapconfig.image_count;
        for rot in &mut self.vx.debtris.rotbuffer {
            *rot += rotation.into().0;
        }
    }
}

/// Handle to a debug triangle
///
/// Used to update/remove a debug triangle.
pub struct Handle(usize);

/// Information used when creating/updating a debug triangle
#[derive(Clone, Copy)]
pub struct DebugTriangle {
    pub origin: [(f32, f32); 3],
    pub colors_rgba: [(u8, u8, u8, u8); 3],
    pub translation: (f32, f32),
    pub rotation: f32,
    pub scale: f32,
}

impl From<[f32; 6]> for DebugTriangle {
    fn from(array: [f32; 6]) -> Self {
        let mut tri = Self::default();
        tri.origin[0].0 = array[0];
        tri.origin[0].1 = array[1];
        tri.origin[1].0 = array[2];
        tri.origin[1].1 = array[3];
        tri.origin[2].0 = array[4];
        tri.origin[2].1 = array[5];
        tri
    }
}

impl Default for DebugTriangle {
    /// Creates a default equilateral RGB triangle without opacity or rotation
    fn default() -> Self {
        let origin = make_centered_equilateral_triangle();
        DebugTriangle {
            origin: [
                (origin[0], origin[1]),
                (origin[2], origin[3]),
                (origin[4], origin[5]),
            ],
            colors_rgba: [(255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 255)],
            rotation: 0f32,
            translation: (0f32, 0f32),
            scale: 1f32,
        }
    }
}

impl DebugTriangle {
    /// Compute the circle that contains the entire triangle regardless of rotation
    ///
    /// Useful when making sure triangles do not touch by adding both their radii together and
    /// using that to space triangles.
    pub fn radius(&self) -> f32 {
        (self.origin[0].0.powi(2) + self.origin[0].1.powi(2))
            .sqrt()
            .max(
                (self.origin[1].0.powi(2) + self.origin[1].1.powi(2))
                    .sqrt()
                    .max((self.origin[2].0.powi(2) + self.origin[2].1.powi(2)).sqrt()),
            )
            * self.scale
    }
}

// ---

pub fn create_debug_triangle(
    device: &back::Device,
    adapter: &Adapter<back::Backend>,
    format: format::Format,
    image_count: usize,
) -> DebugTriangleData {
    pub const VERTEX_SOURCE: &[u8] = include_bytes!["../_build/spirv/debtri.vert.spirv"];
    pub const FRAGMENT_SOURCE: &[u8] = include_bytes!["../_build/spirv/debtri.frag.spirv"];

    let vs_module = { unsafe { device.create_shader_module(&VERTEX_SOURCE) }.unwrap() };
    let fs_module = { unsafe { device.create_shader_module(&FRAGMENT_SOURCE) }.unwrap() };

    const ENTRY_NAME: &str = "main";
    let vs_module: <back::Backend as Backend>::ShaderModule = vs_module;
    let (vs_entry, fs_entry) = (
        pso::EntryPoint {
            entry: ENTRY_NAME,
            module: &vs_module,
            specialization: pso::Specialization::default(),
        },
        pso::EntryPoint {
            entry: ENTRY_NAME,
            module: &fs_module,
            specialization: pso::Specialization::default(),
        },
    );

    let shader_entries = pso::GraphicsShaderSet {
        vertex: vs_entry,
        hull: None,
        domain: None,
        geometry: None,
        fragment: Some(fs_entry),
    };

    let input_assembler = pso::InputAssemblerDesc::new(Primitive::TriangleList);

    let vertex_buffers = vec![
        pso::VertexBufferDesc {
            binding: 0,
            stride: 8,
            rate: pso::VertexInputRate::Vertex,
        },
        pso::VertexBufferDesc {
            binding: 1,
            stride: 4,
            rate: pso::VertexInputRate::Vertex,
        },
        pso::VertexBufferDesc {
            binding: 2,
            stride: 8,
            rate: pso::VertexInputRate::Vertex,
        },
        pso::VertexBufferDesc {
            binding: 3,
            stride: 4,
            rate: pso::VertexInputRate::Vertex,
        },
        pso::VertexBufferDesc {
            binding: 4,
            stride: 4,
            rate: pso::VertexInputRate::Vertex,
        },
    ];

    let attributes: Vec<pso::AttributeDesc> = vec![
        pso::AttributeDesc {
            location: 0,
            binding: 0,
            element: pso::Element {
                format: format::Format::Rg32Sfloat,
                offset: 0,
            },
        },
        pso::AttributeDesc {
            location: 1,
            binding: 1,
            element: pso::Element {
                format: format::Format::Rgba8Unorm,
                offset: 0,
            },
        },
        pso::AttributeDesc {
            location: 2,
            binding: 2,
            element: pso::Element {
                format: format::Format::Rg32Sfloat,
                offset: 0,
            },
        },
        pso::AttributeDesc {
            location: 3,
            binding: 3,
            element: pso::Element {
                format: format::Format::R32Sfloat,
                offset: 0,
            },
        },
        pso::AttributeDesc {
            location: 4,
            binding: 4,
            element: pso::Element {
                format: format::Format::R32Sfloat,
                offset: 0,
            },
        },
    ];

    let rasterizer = pso::Rasterizer {
        depth_clamping: false,
        polygon_mode: pso::PolygonMode::Fill,
        cull_face: pso::Face::NONE,
        front_face: pso::FrontFace::CounterClockwise,
        depth_bias: None,
        conservative: false,
    };

    let depth_stencil = pso::DepthStencilDesc {
        depth: pso::DepthTest::Off,
        depth_bounds: false,
        stencil: pso::StencilTest::Off,
    };

    let blender = {
        let blend_state = pso::BlendState::On {
            color: pso::BlendOp::Add {
                src: pso::Factor::One,
                dst: pso::Factor::Zero,
            },
            alpha: pso::BlendOp::Add {
                src: pso::Factor::One,
                dst: pso::Factor::Zero,
            },
        };
        pso::BlendDesc {
            logic_op: Some(pso::LogicOp::Copy),
            targets: vec![pso::ColorBlendDesc(pso::ColorMask::ALL, blend_state)],
        }
    };

    let triangle_render_pass = {
        let attachment = pass::Attachment {
            format: Some(format),
            samples: 1,
            ops: pass::AttachmentOps::new(
                pass::AttachmentLoadOp::Clear,
                pass::AttachmentStoreOp::Store,
            ),
            stencil_ops: pass::AttachmentOps::DONT_CARE,
            layouts: image::Layout::Undefined..image::Layout::Present,
        };
        let depth = pass::Attachment {
            format: Some(format::Format::D32Sfloat),
            samples: 1,
            ops: pass::AttachmentOps::new(
                pass::AttachmentLoadOp::Clear,
                pass::AttachmentStoreOp::Store,
            ),
            stencil_ops: pass::AttachmentOps::DONT_CARE,
            layouts: image::Layout::Undefined..image::Layout::DepthStencilAttachmentOptimal,
        };

        let subpass = pass::SubpassDesc {
            colors: &[(0, image::Layout::ColorAttachmentOptimal)],
            depth_stencil: Some(&(1, image::Layout::DepthStencilAttachmentOptimal)),
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        unsafe { device.create_render_pass(&[attachment, depth], &[subpass], &[]) }
            .expect("Can't create render pass")
    };

    let baked_states = pso::BakedStates {
        viewport: None,
        scissor: None,
        blend_color: None,
        depth_bounds: None,
    };

    let bindings = Vec::<pso::DescriptorSetLayoutBinding>::new();
    let immutable_samplers = Vec::<<back::Backend as Backend>::Sampler>::new();
    let triangle_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let mut push_constants = Vec::<(pso::ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((pso::ShaderStageFlags::VERTEX, 0..1));

    let triangle_pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&triangle_descriptor_set_layouts, push_constants)
            .expect("Couldn't create a pipeline layout")
    };

    // Describe the pipeline (rasterization, triangle interpretation)
    let pipeline_desc = pso::GraphicsPipelineDesc {
        shaders: shader_entries,
        rasterizer,
        vertex_buffers,
        attributes,
        input_assembler,
        blender,
        depth_stencil,
        multisampling: None,
        baked_states,
        layout: &triangle_pipeline_layout,
        subpass: pass::Subpass {
            index: 0,
            main_pass: &triangle_render_pass,
        },
        flags: pso::PipelineCreationFlags::empty(),
        parent: pso::BasePipeline::None,
    };

    let triangle_pipeline = unsafe {
        device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Couldn't create a graphics pipeline!")
    };

    unsafe {
        device.destroy_shader_module(vs_module);
        device.destroy_shader_module(fs_module);
    }

    let posbuf = (0..image_count)
        .map(|_| super::utils::ResizBuf::new(&device, &adapter))
        .collect::<Vec<_>>();
    let colbuf = (0..image_count)
        .map(|_| super::utils::ResizBuf::new(&device, &adapter))
        .collect::<Vec<_>>();
    let tranbuf = (0..image_count)
        .map(|_| super::utils::ResizBuf::new(&device, &adapter))
        .collect::<Vec<_>>();
    let rotbuf = (0..image_count)
        .map(|_| super::utils::ResizBuf::new(&device, &adapter))
        .collect::<Vec<_>>();
    let scalebuf = (0..image_count)
        .map(|_| super::utils::ResizBuf::new(&device, &adapter))
        .collect::<Vec<_>>();

    DebugTriangleData {
        hidden: false,
        triangles_count: 0,

        posbuf_touch: 0,
        colbuf_touch: 0,
        tranbuf_touch: 0,
        rotbuf_touch: 0,
        scalebuf_touch: 0,

        posbuffer: vec![],   // 6 per triangle
        colbuffer: vec![],   // 12 per triangle
        tranbuffer: vec![],  // 6 per triangle
        rotbuffer: vec![],   // 3 per triangle
        scalebuffer: vec![], // 3 per triangle

        posbuf,
        colbuf,
        tranbuf,
        rotbuf,
        scalebuf,

        descriptor_set: triangle_descriptor_set_layouts,
        pipeline: ManuallyDrop::new(triangle_pipeline),
        pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        render_pass: ManuallyDrop::new(triangle_render_pass),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use cgmath::Deg;
    use logger::{Generic, GenericLogger, Logger};
    use test::{black_box, Bencher};

    // ---

    #[test]
    fn simple_triangle() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);
        let tri = DebugTriangle::default();

        vx.debtri().push(tri);
        utils::add_4_screencorners(&mut vx);

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "simple_triangle", img);
    }

    #[test]
    fn test_single_triangle_api() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);
        let tri = DebugTriangle::default();

        let mut debtri = vx.debtri();
        let handle = debtri.push(tri);
        debtri.set_scale(&handle, 0.1);
        debtri.set_rotation(&handle, Deg(25.0));
        debtri.set_position(&handle, (0.05, 0.4));
        debtri.translate(&handle, (0.2, 0.1));
        debtri.rotate(&handle, Deg(5.0));

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "test_single_triangle_api", img);
    }

    // ---

    #[test]
    fn simple_triangle_change_color() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);
        let tri = DebugTriangle::default();

        let mut debtri = vx.debtri();
        let idx = debtri.push(tri);
        debtri.set_color(&idx, [255, 0, 255, 255]);

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "simple_triangle_change_color", img);
    }

    #[test]
    fn debug_triangle_corners_widescreen() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&vx);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = vx.debtri().push(tri);
            }
        }

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "debug_triangle_corners_widescreen", img);
    }

    #[test]
    fn debug_triangle_corners_tallscreen() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1x2k);
        let prspect = gen_perspective(&vx);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = vx.debtri().push(tri);
            }
        }

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "debug_triangle_corners_tallscreen", img);
    }

    #[test]
    fn circle_of_triangles() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&vx);

        for i in 0..360 {
            let mut tri = DebugTriangle::default();
            tri.translation = ((i as f32).cos(), (i as f32).sin());
            tri.scale = 0.1f32;
            let _idx = vx.debtri().push(tri);
        }

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "circle_of_triangles", img);
    }

    #[test]
    fn triangle_in_corner() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut tri = DebugTriangle::default();
        tri.scale = 0.1f32;
        let radi = tri.radius();

        let trans = -1f32 + radi;
        for j in 0..31 {
            for i in 0..31 {
                tri.translation = (trans + i as f32 * 2.0 * radi, trans + j as f32 * 2.0 * radi);
                vx.debtri().push(tri);
            }
        }

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "triangle_in_corner", img);
    }

    #[test]
    fn windmills() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);
        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "windmills", img);
    }

    #[test]
    fn windmills_mass_edits() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);
        let mut debtri = vx.debtri();

        debtri.translate_all((1.0, 0.5));
        debtri.rotate_all(Deg(90.0));
        debtri.scale_all(2.0);

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "windmills_mass_edits", img);
    }

    #[test]
    fn windmills_hidden() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);

        vx.debtri().hide();

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "windmills_hidden", img);

        vx.debtri().show();

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "windmills_hidden_now_shown", img);
    }

    #[test]
    fn windmills_ignore_perspective() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);
        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "windmills_ignore_perspective", img);
    }

    #[test]
    fn windmills_change_color() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let handles = utils::add_windmills(&mut vx, false);
        let mut debtri = vx.debtri();
        debtri.set_color(&handles[0], [255, 0, 0, 255]);
        debtri.set_color(&handles[249], [0, 255, 0, 255]);
        debtri.set_color(&handles[499], [0, 0, 255, 255]);
        debtri.set_color(&handles[999], [0, 0, 0, 255]);

        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "windmills_change_color", img);
    }

    #[test]
    fn rotating_windmills_drawing_invariant() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);
        for _ in 0..30 {
            vx.debtri().rotate_all(Deg(-1.0f32));
        }
        let img = vx.draw_frame_copy_framebuffer(&prspect);

        utils::assert_swapchain_eq(&mut vx, "rotating_windmills_drawing_invariant", img);
        utils::remove_windmills(&mut vx);

        utils::add_windmills(&mut vx, false);
        for _ in 0..30 {
            vx.debtri().rotate_all(Deg(-1.0f32));
            vx.draw_frame(&prspect);
        }
        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "rotating_windmills_drawing_invariant", img);
    }

    #[test]
    fn windmills_given_initial_rotation() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, true);
        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "windmills_given_initial_rotation", img);
    }

    // ---

    #[bench]
    fn bench_simple_triangle(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        vx.debtri().push(DebugTriangle::default());
        utils::add_4_screencorners(&mut vx);

        b.iter(|| {
            vx.draw_frame(&prspect);
        });
    }

    #[bench]
    fn bench_still_windmills(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);

        b.iter(|| {
            vx.draw_frame(&prspect);
        });
    }

    #[bench]
    fn bench_windmills_set_color(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);

        let handles = utils::add_windmills(&mut vx, false);

        b.iter(|| {
            vx.debtri()
                .set_color(&handles[0], black_box([0, 0, 0, 255]));
        });
    }

    #[bench]
    fn bench_rotating_windmills(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        utils::add_windmills(&mut vx, false);

        b.iter(|| {
            vx.debtri().rotate_all(Deg(1.0f32));
            vx.draw_frame(&prspect);
        });
    }

    #[bench]
    fn bench_rotating_windmills_set_color(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let last = utils::add_windmills(&mut vx, false).pop().unwrap();

        b.iter(|| {
            vx.debtri().rotate_all(Deg(1.0f32));
            vx.debtri().set_color(&last, [255, 0, 255, 255]);
            vx.draw_frame(&prspect);
        });
    }

    #[bench]
    fn bench_rotating_windmills_no_render(b: &mut Bencher) {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);

        utils::add_windmills(&mut vx, false);

        b.iter(|| {
            vx.debtri().rotate_all(Deg(1.0f32));
        });
    }
}
