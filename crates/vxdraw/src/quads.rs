//! Methods and types to control quads
//!
//! A quad is a renderable consisting of 4 points. Each point has a color and position associated
//! with it. By using different colors in the different points, the colors will "blend" into each
//! other. Opacity is also supported on quads.
//!
//! See [quads::Quads] for all operations supported on quads.
//!
//! # Example - Simple quad and some operations #
//! A showcase of basic operations on a quad.
//! ```
//! use cgmath::{prelude::*, Deg, Matrix4};
//! use logger::{Generic, GenericLogger, Logger};
//! use vxdraw::{ShowWindow, VxDraw};
//! fn main() {
//!     let mut vx = VxDraw::new(Logger::<Generic>::spawn_test().to_logpass(),
//!         ShowWindow::Headless1k); // Change this to ShowWindow::Enable to show the window
//!
//!     // Create a new layer of quads
//!     let quad = vx.quads().add_layer(vxdraw::quads::LayerOptions::default());
//!
//!     // Create a new quad
//!     let handle = vx.quads().add(&quad, vxdraw::quads::Quad::default());
//!
//!     // Turn the quad white
//!     vx.quads().set_solid_color(&handle, [255, 255, 255, 255]);
//!
//!     // Rotate the quad 45 degrees (counter clockwise)
//!     vx.quads().set_rotation(&handle, Deg(45.0));
//!
//!     // Scale the quad to half its current size
//!     vx.quads().scale(&handle, 0.5);
//!
//!     // Draw the frame with the identity matrix transformation (meaning no transformations)
//!     vx.draw_frame(&Matrix4::identity());
//!
//!     // Sleep here so the window does not instantly disappear
//!     std::thread::sleep(std::time::Duration::new(3, 0));
//! }
//! ```
//!
//! # Example - Curtain-like fade based on 4 quads #
//! This example moves 4 quads from the sides to "close" the scene as curtains would do.
//! ```
//! use cgmath::{prelude::*, Deg, Matrix4};
//! use logger::{Generic, GenericLogger, Logger};
//! use vxdraw::{quads::*, ShowWindow, VxDraw};
//!
//! fn main() {
//!     let mut vx = VxDraw::new(Logger::<Generic>::spawn_test().to_logpass(),
//!         ShowWindow::Headless1k); // Change this to ShowWindow::Enable to show the window
//!
//!     // Create a new layer of quads
//!     let layer = vx.quads().add_layer(LayerOptions::default());
//!
//!     // The width of the faded quad, try changing this to 2.0, or 1.0 and observe
//!     let fade_width = 0.5;
//!
//!     // The left quad data, has the right vertices completely transparent
//!     let quad_data = Quad::new()
//!         .width(fade_width)
//!         .colors([
//!             (0, 0, 0, 255),
//!             (0, 0, 0, 255),
//!             (0, 0, 0, 0),
//!             (0, 0, 0, 0),
//!         ])
//!         .translation((- 1.0 - fade_width / 2.0, 0.0));
//!
//!     // Create a new quad
//!     let left_quad_fade = vx.quads().add(&layer, quad_data);
//!
//!     // The right quad data, has the left vertices completely transparent
//!     let quad_data = Quad::new()
//!         .width(fade_width)
//!         .colors([
//!             (0, 0, 0, 0),
//!             (0, 0, 0, 0),
//!             (0, 0, 0, 255),
//!             (0, 0, 0, 255),
//!         ])
//!         .translation((1.0 + fade_width / 2.0, 0.0));
//!
//!     // Create a new quad
//!     let right_quad_fade = vx.quads().add(&layer, quad_data);
//!
//!     // Now create the completely black quads
//!     let quad_data = Quad::default();
//!     let left_quad = vx.quads().add(&layer, quad_data);
//!     let right_quad = vx.quads().add(&layer, quad_data);
//!
//!     // Some math to ensure the faded quad and the solid quads move at the same rate, and that
//!     // both solid quads cover half the screen on the last frame.
//!     let fade_width_offscreen = 1.0 + fade_width / 2.0;
//!     let fade_pos_solid = 2.0 + fade_width;
//!     let nlscale = (1.0 + fade_width) / (1.0 + fade_width / 2.0);
//!
//!     // How many frames the entire animation takes, try making it shorter or longer
//!     let frames = 50;
//!
//!     for idx in 0..frames {
//!
//!         let perc = idx as f32 * nlscale;
//!         // Move the quads
//!         vx.quads().set_translation(&left_quad_fade, (-fade_width_offscreen + (fade_width_offscreen / frames as f32) * perc, 0.0));
//!         vx.quads().set_translation(&right_quad_fade, (fade_width_offscreen - (fade_width_offscreen / frames as f32) * perc, 0.0));
//!         vx.quads().set_translation(&left_quad, (-fade_pos_solid + (fade_width_offscreen / frames as f32) * perc, 0.0));
//!         vx.quads().set_translation(&right_quad, (fade_pos_solid - (fade_width_offscreen / frames as f32) * perc, 0.0));
//!
//!         // Draw the frame with the identity matrix transformation (meaning no transformations)
//!         // Normally we use a perspective that makes the window from appearing stretched, but
//!         // for this example using the identity matrix makes the calculations easier, as the
//!         // sides of the screen are now -1 to 1.
//!         vx.draw_frame(&Matrix4::identity());
//!
//!         // Sleep so we can see some animation
//!         std::thread::sleep(std::time::Duration::new(0, 16_000_000));
//!     }
//! }
//! ```
//!
//! Note how the above has two overlapping, faded quads. This can be an undesired animation
//! artifact. The intent of the example is to show how to work with the library.
use super::utils::*;
use crate::data::{DrawType, QuadsData, VxDraw};
use cgmath::Rad;
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
use gfx_hal::{device::Device, format, image, pass, pso, Backend, Primitive};
use std::mem::ManuallyDrop;

// ---

/// Handle referring to a single quad
#[derive(Debug)]
pub struct Handle(usize, usize);

/// Handle referring to a quad layer
#[derive(Debug)]
pub struct Layer(usize);

impl Layerable for Layer {
    fn get_layer(&self, vx: &VxDraw) -> usize {
        for (idx, ord) in vx.draw_order.iter().enumerate() {
            match ord {
                DrawType::Quad { id } if *id == self.0 => {
                    return idx;
                }
                _ => {}
            }
        }
        panic!["Unable to get layer"]
    }
}

/// Options for creating a layer of quads
#[derive(Debug)]
pub struct LayerOptions {
    depth_test: bool,
    hide: bool,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            depth_test: false,
            hide: false,
        }
    }
}

impl LayerOptions {
    /// Hide this layer
    pub fn hide(mut self) -> Self {
        self.hide = true;
        self
    }

    /// Show this layer (default)
    pub fn show(mut self) -> Self {
        self.hide = false;
        self
    }
}

// ---

/// Quad information used for creating and getting
#[derive(Clone, Copy, Debug)]
pub struct Quad {
    width: f32,
    height: f32,
    depth: f32,
    colors: [(u8, u8, u8, u8); 4],
    translation: (f32, f32),
    rotation: f32,
    scale: f32,
    /// Moves the origin of the quad to some point, for instance, you may want a corner of the quad
    /// to be the origin. This affects rotation and translation of the quad.
    origin: (f32, f32),
}

impl Quad {
    /// Same as default
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the width of the quad
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the height of the quad
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set the colors of the quad
    ///
    /// The colors are added on top of whatever the quad's texture data is
    pub fn colors(mut self, colors: [(u8, u8, u8, u8); 4]) -> Self {
        self.colors = colors;
        self
    }

    /// Set the translation
    pub fn translation(mut self, trn: (f32, f32)) -> Self {
        self.translation = trn;
        self
    }

    /// Set the rotation. Rotation is counter-clockwise
    pub fn rotation(mut self, rot: f32) -> Self {
        self.rotation = rot;
        self
    }

    /// Set the scaling factor of this quad
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Set the origin of this quad
    pub fn origin(mut self, origin: (f32, f32)) -> Self {
        self.origin = origin;
        self
    }
}

impl Default for Quad {
    fn default() -> Self {
        Quad {
            width: 2.0,
            height: 2.0,
            depth: 0.0,
            colors: [(0, 0, 0, 255); 4],
            translation: (0.0, 0.0),
            rotation: 0.0,
            scale: 1.0,
            origin: (0.0, 0.0),
        }
    }
}

// ---

/// Accessor object to all quads
///
/// A quad is a colored object with 4 points.
/// See [crate::quads] for examples.
pub struct Quads<'a> {
    vx: &'a mut VxDraw,
}

impl<'a> Quads<'a> {
    /// Spawn the accessor object from [VxDraw].
    ///
    /// This is a very cheap operation.
    pub fn new(vx: &'a mut VxDraw) -> Self {
        Self { vx }
    }

    /// Disable drawing of the quads at this layer
    pub fn hide(&mut self, layer: &Layer) {
        self.vx.quads[layer.0].hidden = true;
    }

    /// Enable drawing of the quads at this layer
    pub fn show(&mut self, layer: &Layer) {
        self.vx.quads[layer.0].hidden = false;
    }

    /// Compare quad draw order
    ///
    /// All quads are drawn in a specific order. This method figures out which order is used
    /// between two quads. The order can be manipulated by [Quads::swap_draw_order].
    pub fn compare_draw_order(&self, left: &Handle, right: &Handle) -> std::cmp::Ordering {
        let layer_ordering = left.0.cmp(&right.0);
        if layer_ordering == std::cmp::Ordering::Equal {
            left.1.cmp(&right.1)
        } else {
            layer_ordering
        }
    }

    /// Swap two quads with each other
    ///
    /// Swaps the internal data of each quad (all vertices and their data, translation,
    /// and so on). The effect of this is that the draw order is swapped too, meaning that the
    /// quads reverse order (one drawn on top of the other).
    ///
    /// This function can swap quads from two different layers, but also quads in the same
    /// layer.
    pub fn swap_draw_order(&mut self, left: &mut Handle, right: &mut Handle) {
        let q1d = self.vx.quads[left.0].posbuffer[left.1];
        let q2d = self.vx.quads[right.0].posbuffer[right.1];
        self.vx.quads[left.0].posbuffer[left.1] = q2d;
        self.vx.quads[right.0].posbuffer[right.1] = q1d;

        let q1d = self.vx.quads[left.0].colbuffer[left.1];
        let q2d = self.vx.quads[right.0].colbuffer[right.1];
        self.vx.quads[left.0].colbuffer[left.1] = q2d;
        self.vx.quads[right.0].colbuffer[right.1] = q1d;

        let q1d = self.vx.quads[left.0].tranbuffer[left.1];
        let q2d = self.vx.quads[right.0].tranbuffer[right.1];
        self.vx.quads[left.0].tranbuffer[left.1] = q2d;
        self.vx.quads[right.0].tranbuffer[right.1] = q1d;

        let q1d = self.vx.quads[left.0].rotbuffer[left.1];
        let q2d = self.vx.quads[right.0].rotbuffer[right.1];
        self.vx.quads[left.0].rotbuffer[left.1] = q2d;
        self.vx.quads[right.0].rotbuffer[right.1] = q1d;

        let q1d = self.vx.quads[left.0].scalebuffer[left.1];
        let q2d = self.vx.quads[right.0].scalebuffer[right.1];
        self.vx.quads[left.0].scalebuffer[left.1] = q2d;
        self.vx.quads[right.0].scalebuffer[right.1] = q1d;

        self.vx.quads[left.0].posbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[left.0].colbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[left.0].tranbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[left.0].rotbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[left.0].scalebuf_touch = self.vx.swapconfig.image_count;

        self.vx.quads[right.0].posbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[right.0].colbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[right.0].tranbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[right.0].rotbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[right.0].scalebuf_touch = self.vx.swapconfig.image_count;

        std::mem::swap(&mut left.0, &mut right.0);
        std::mem::swap(&mut left.1, &mut right.1);
    }

    /// Create a new layer for quads
    ///
    /// This new layer will be ordered on top of all previous layers, meaning that its quads will
    /// be drawn on top of all other drawn items. If another layer is created, that layer will be
    /// drawn on top of this layer, and so on.
    pub fn add_layer(&mut self, options: LayerOptions) -> Layer {
        let s = &mut *self.vx;
        pub const VERTEX_SOURCE: &[u8] = include_bytes!["../_build/spirv/quads.vert.spirv"];

        pub const FRAGMENT_SOURCE: &[u8] = include_bytes!["../_build/spirv/quads.frag.spirv"];

        let vs_module = { unsafe { s.device.create_shader_module(&VERTEX_SOURCE) }.unwrap() };
        let fs_module = { unsafe { s.device.create_shader_module(&FRAGMENT_SOURCE) }.unwrap() };

        // Describe the shaders
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

        let vertex_buffers: Vec<pso::VertexBufferDesc> = vec![
            pso::VertexBufferDesc {
                binding: 0,
                stride: 2 * 4,
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
            front_face: pso::FrontFace::Clockwise,
            depth_bias: None,
            conservative: false,
        };

        let depth_stencil = pso::DepthStencilDesc {
            depth: if options.depth_test {
                pso::DepthTest::On {
                    fun: pso::Comparison::LessEqual,
                    write: true,
                }
            } else {
                pso::DepthTest::Off
            },
            depth_bounds: false,
            stencil: pso::StencilTest::Off,
        };
        let blender = {
            let blend_state = pso::BlendState::On {
                color: pso::BlendOp::Add {
                    src: pso::Factor::SrcAlpha,
                    dst: pso::Factor::OneMinusSrcAlpha,
                },
                alpha: pso::BlendOp::Add {
                    src: pso::Factor::One,
                    dst: pso::Factor::OneMinusSrcAlpha,
                },
            };
            pso::BlendDesc {
                logic_op: Some(pso::LogicOp::Copy),
                targets: vec![pso::ColorBlendDesc(pso::ColorMask::ALL, blend_state)],
            }
        };
        let quad_render_pass = {
            let attachment = pass::Attachment {
                format: Some(s.format),
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

            unsafe {
                s.device
                    .create_render_pass(&[attachment, depth], &[subpass], &[])
            }
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
        let quad_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
            vec![unsafe {
                s.device
                    .create_descriptor_set_layout(bindings, immutable_samplers)
                    .expect("Couldn't make a DescriptorSetLayout")
            }];
        let mut push_constants = Vec::<(pso::ShaderStageFlags, std::ops::Range<u32>)>::new();
        push_constants.push((pso::ShaderStageFlags::VERTEX, 0..16));

        let quad_pipeline_layout = unsafe {
            s.device
                .create_pipeline_layout(&quad_descriptor_set_layouts, push_constants)
                .expect("Couldn't create a pipeline layout")
        };

        // Describe the pipeline (rasterization, quad interpretation)
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
            layout: &quad_pipeline_layout,
            subpass: pass::Subpass {
                index: 0,
                main_pass: &quad_render_pass,
            },
            flags: pso::PipelineCreationFlags::empty(),
            parent: pso::BasePipeline::None,
        };

        let quad_pipeline = unsafe {
            s.device
                .create_graphics_pipeline(&pipeline_desc, None)
                .expect("Couldn't create a graphics pipeline!")
        };

        unsafe {
            s.device.destroy_shader_module(vs_module);
            s.device.destroy_shader_module(fs_module);
        }

        let image_count = s.swapconfig.image_count;
        let posbuf = (0..image_count)
            .map(|_| super::utils::ResizBuf::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();
        let colbuf = (0..image_count)
            .map(|_| super::utils::ResizBuf::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();
        let tranbuf = (0..image_count)
            .map(|_| super::utils::ResizBuf::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();
        let rotbuf = (0..image_count)
            .map(|_| super::utils::ResizBuf::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();
        let scalebuf = (0..image_count)
            .map(|_| super::utils::ResizBuf::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();

        let indices = (0..image_count)
            .map(|_| super::utils::ResizBufIdx4::new(&s.device, &s.adapter))
            .collect::<Vec<_>>();

        let quads = QuadsData {
            hidden: options.hide,
            count: 0,

            holes: vec![],

            posbuf_touch: 0,
            colbuf_touch: 0,
            tranbuf_touch: 0,
            rotbuf_touch: 0,
            scalebuf_touch: 0,

            posbuffer: vec![],
            colbuffer: vec![],
            tranbuffer: vec![],
            rotbuffer: vec![],
            scalebuffer: vec![],

            posbuf,
            colbuf,
            tranbuf,
            rotbuf,
            scalebuf,

            indices,

            descriptor_set: quad_descriptor_set_layouts,
            pipeline: ManuallyDrop::new(quad_pipeline),
            pipeline_layout: ManuallyDrop::new(quad_pipeline_layout),
            render_pass: ManuallyDrop::new(quad_render_pass),
        };
        s.quads.push(quads);
        s.draw_order.push(DrawType::Quad {
            id: s.quads.len() - 1,
        });
        Layer(s.quads.len() - 1)
    }

    /// Add a new quad to the given layer
    ///
    /// The new quad will be based on the data in [Quad], and inserted into the given [Layer].
    pub fn add(&mut self, layer: &Layer, quad: Quad) -> Handle {
        let width = quad.width;
        let height = quad.height;

        let topleft = (
            -width / 2f32 - quad.origin.0,
            -height / 2f32 - quad.origin.1,
        );
        let topright = (width / 2f32 - quad.origin.0, -height / 2f32 - quad.origin.1);
        let bottomleft = (-width / 2f32 - quad.origin.0, height / 2f32 - quad.origin.1);
        let bottomright = (width / 2f32 - quad.origin.0, height / 2f32 - quad.origin.1);
        let replace = self.vx.quads.get(layer.0).map(|x| !x.holes.is_empty());
        if replace.is_none() {
            panic!["Layer does not exist"];
        }
        let handle = if replace.unwrap() {
            let hole = self.vx.quads.get_mut(layer.0).unwrap().holes.pop().unwrap();
            let handle = Handle(layer.0, hole);
            self.set_deform(
                &handle,
                [
                    (topleft.0, topleft.1),
                    (bottomleft.0, bottomleft.1),
                    (bottomright.0, bottomright.1),
                    (topright.0, topright.1),
                ],
            );
            self.set_color(
                &handle,
                [
                    quad.colors[0].0,
                    quad.colors[0].1,
                    quad.colors[0].2,
                    quad.colors[0].3,
                    quad.colors[1].0,
                    quad.colors[1].1,
                    quad.colors[1].2,
                    quad.colors[1].3,
                    quad.colors[2].0,
                    quad.colors[2].1,
                    quad.colors[2].2,
                    quad.colors[2].3,
                    quad.colors[3].0,
                    quad.colors[3].1,
                    quad.colors[3].2,
                    quad.colors[3].3,
                ],
            );
            self.set_translation(&handle, (quad.translation.0, quad.translation.1));
            self.set_rotation(&handle, Rad(quad.rotation));
            self.set_scale(&handle, quad.scale);
            handle
        } else {
            let quads = self.vx.quads.get_mut(layer.0).unwrap();
            quads.posbuffer.push([
                topleft.0,
                topleft.1,
                bottomleft.0,
                bottomleft.1,
                bottomright.0,
                bottomright.1,
                topright.0,
                topright.1,
            ]);
            quads.colbuffer.push([
                quad.colors[0].0,
                quad.colors[0].1,
                quad.colors[0].2,
                quad.colors[0].3,
                quad.colors[1].0,
                quad.colors[1].1,
                quad.colors[1].2,
                quad.colors[1].3,
                quad.colors[2].0,
                quad.colors[2].1,
                quad.colors[2].2,
                quad.colors[2].3,
                quad.colors[3].0,
                quad.colors[3].1,
                quad.colors[3].2,
                quad.colors[3].3,
            ]);
            quads.tranbuffer.push([
                quad.translation.0,
                quad.translation.1,
                quad.translation.0,
                quad.translation.1,
                quad.translation.0,
                quad.translation.1,
                quad.translation.0,
                quad.translation.1,
            ]);
            quads
                .rotbuffer
                .push([quad.rotation, quad.rotation, quad.rotation, quad.rotation]);
            quads
                .scalebuffer
                .push([quad.scale, quad.scale, quad.scale, quad.scale]);

            quads.count += 1;

            Handle(layer.0, quads.count - 1)
        };

        let quads = self.vx.quads.get_mut(layer.0).unwrap();
        quads.posbuf_touch = self.vx.swapconfig.image_count;
        quads.colbuf_touch = self.vx.swapconfig.image_count;
        quads.tranbuf_touch = self.vx.swapconfig.image_count;
        quads.rotbuf_touch = self.vx.swapconfig.image_count;
        quads.scalebuf_touch = self.vx.swapconfig.image_count;

        handle
    }

    /// Remove a quad
    ///
    /// The quad is set to a scale of 0 and its handle is stored internally in a list of
    /// `holes`. Calling [Quads::add] with available holes will fill the first available hole
    /// with the new quad.
    pub fn remove(&mut self, handle: Handle) {
        self.vx.quads[handle.0].holes.push(handle.1);
        self.set_scale(&handle, 0.0);
    }

    // ---

    /// Change the vertices of the model-space
    ///
    /// The name `set_deform` is used to keep consistent [Quads::deform].
    /// What this function does is just setting absolute vertex positions for each vertex in the
    /// triangle.
    pub fn set_deform(&mut self, handle: &Handle, points: [(f32, f32); 4]) {
        self.vx.quads[handle.0].posbuf_touch = self.vx.swapconfig.image_count;
        let vertex = &mut self.vx.quads[handle.0].posbuffer[handle.1];
        for (idx, point) in points.iter().enumerate() {
            vertex[idx * 2] = point.0;
            vertex[idx * 2 + 1] = point.1;
        }
    }

    /// Set a solid color of a quad
    pub fn set_solid_color(&mut self, handle: &Handle, rgba: [u8; 4]) {
        self.vx.quads[handle.0].colbuf_touch = self.vx.swapconfig.image_count;
        for idx in 0..4 {
            self.vx.quads[handle.0].colbuffer[handle.1][idx * 4..(idx + 1) * 4]
                .copy_from_slice(&rgba);
        }
    }

    /// Set a solid color each vertex of a quad
    pub fn set_color(&mut self, handle: &Handle, rgba: [u8; 16]) {
        self.vx.quads[handle.0].colbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[handle.0].colbuffer[handle.1].copy_from_slice(&rgba);
    }

    /// Set the position (translation) of a quad triangle
    ///
    /// The name `set_translation` is chosen to keep the counterparts [Quads::translate] and
    /// `translate_all` consistent. This function can purely be thought of as setting the position
    /// of the triangle with respect to the model-space's origin.
    pub fn set_translation(&mut self, handle: &Handle, position: (f32, f32)) {
        self.vx.quads[handle.0].tranbuf_touch = self.vx.swapconfig.image_count;
        for idx in 0..4 {
            self.vx.quads[handle.0].tranbuffer[handle.1][idx * 2] = position.0;
            self.vx.quads[handle.0].tranbuffer[handle.1][idx * 2 + 1] = position.1;
        }
    }

    /// Set the rotation of a quad
    ///
    /// The rotation is about the model space origin.
    pub fn set_rotation<T: Copy + Into<Rad<f32>>>(&mut self, handle: &Handle, deg: T) {
        let angle = deg.into().0;
        self.vx.quads[handle.0].rotbuf_touch = self.vx.swapconfig.image_count;
        self.vx.quads[handle.0].rotbuffer[handle.1].copy_from_slice(&[angle, angle, angle, angle]);
    }

    /// Set the scale of a quad
    pub fn set_scale(&mut self, handle: &Handle, scale: f32) {
        self.vx.quads[handle.0].scalebuf_touch = self.vx.swapconfig.image_count;
        for sc in &mut self.vx.quads[handle.0].scalebuffer[handle.1] {
            *sc = scale;
        }
    }

    // ---

    /// Deform a quad by adding delta vertices
    ///
    /// Adds the delta vertices to the quad. Beware: This changes model space form.
    pub fn deform(&mut self, handle: &Handle, delta: [(f32, f32); 4]) {
        self.vx.quads[handle.0].posbuf_touch = self.vx.swapconfig.image_count;
        let points = &mut self.vx.quads[handle.0].posbuffer[handle.1];
        points[0] += delta[0].0;
        points[1] += delta[0].1;
        points[2] += delta[1].0;
        points[3] += delta[1].1;
        points[4] += delta[2].0;
        points[5] += delta[2].1;
        points[6] += delta[3].0;
        points[7] += delta[3].1;
    }

    /// Translate a quad by a vector
    ///
    /// Translation does not mutate the model-space of a quad.
    pub fn translate(&mut self, handle: &Handle, movement: (f32, f32)) {
        self.vx.quads[handle.0].tranbuf_touch = self.vx.swapconfig.image_count;
        for idx in 0..4 {
            self.vx.quads[handle.0].tranbuffer[handle.1][idx * 2] += movement.0;
            self.vx.quads[handle.0].tranbuffer[handle.1][idx * 2 + 1] += movement.1;
        }
    }

    /// Rotate a quad
    ///
    /// Rotation does not mutate the model-space of a quad.
    pub fn rotate<T: Copy + Into<Rad<f32>>>(&mut self, handle: &Handle, deg: T) {
        self.vx.quads[handle.0].rotbuf_touch = self.vx.swapconfig.image_count;
        for rot in &mut self.vx.quads[handle.0].rotbuffer[handle.1] {
            *rot += deg.into().0;
        }
    }

    /// Scale a quad
    ///
    /// Scale does not mutate the model-space of a quad.
    pub fn scale(&mut self, handle: &Handle, scale: f32) {
        self.vx.quads[handle.0].scalebuf_touch = self.vx.swapconfig.image_count;
        for sc in &mut self.vx.quads[handle.0].scalebuffer[handle.1] {
            *sc *= scale;
        }
    }

    // ---

    /// Rotate all quads
    ///
    /// Adds the rotation in the argument to the existing rotation of each quad.
    /// See [Quads::rotate] for more information.
    pub fn rotate_all<T: Copy + Into<Rad<f32>>>(&mut self, layer: &Layer, deg: T) {
        self.vx.quads[layer.0].rotbuf_touch = self.vx.swapconfig.image_count;
        for rot in self.vx.quads[layer.0].rotbuffer.iter_mut() {
            rot[0] += deg.into().0;
            rot[1] += deg.into().0;
            rot[2] += deg.into().0;
            rot[3] += deg.into().0;
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use cgmath::Deg;
    use logger::{Generic, GenericLogger, Logger};

    #[test]
    fn simple_quad() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let layer = vx.quads().add_layer(LayerOptions::default());
        vx.quads().add(&layer, quad);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad", img);
    }

    #[test]
    fn simple_quad_hide() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let quad = quads::Quad::default();

        let layer = vx.quads().add_layer(LayerOptions::default());
        vx.quads().add(&layer, quad);
        vx.quads().hide(&layer);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_hide", img);
    }

    #[test]
    fn simple_quad_translated() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let handle = quads.add(&layer, quad);
        quads.translate(&handle, (0.25, 0.4));

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_translated", img);
    }

    #[test]
    fn swapping_quad_draw_order() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let quad = quads::Quad::default();
        let layer = vx.quads().add_layer(LayerOptions::default());
        let mut q1 = vx.quads().add(&layer, quad);
        let mut q2 = vx.quads().add(&layer, quad);

        let mut quads = vx.quads();
        quads.translate(&q1, (-0.5, -0.5));
        quads.set_solid_color(&q1, [255, 0, 255, 255]);
        quads.translate(&q2, (0.5, 0.5));
        quads.set_solid_color(&q2, [0, 255, 255, 128]);

        assert_eq![std::cmp::Ordering::Less, quads.compare_draw_order(&q1, &q2)];
        quads.swap_draw_order(&mut q1, &mut q2);
        assert_eq![
            std::cmp::Ordering::Greater,
            quads.compare_draw_order(&q1, &q2)
        ];

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "swapping_quad_draw_order", img);
    }

    #[test]
    fn swapping_quad_draw_order_different_layers() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let quad = quads::Quad::default();
        let layer1 = vx.quads().add_layer(LayerOptions::default());
        let layer2 = vx.quads().add_layer(LayerOptions::default());
        let mut q1 = vx.quads().add(&layer1, quad);
        let mut q2 = vx.quads().add(&layer2, quad);

        let mut quads = vx.quads();
        quads.translate(&q1, (-0.5, -0.5));
        quads.set_solid_color(&q1, [255, 0, 255, 255]);
        quads.translate(&q2, (0.5, 0.5));
        quads.set_solid_color(&q2, [0, 255, 255, 128]);

        assert_eq![std::cmp::Ordering::Less, quads.compare_draw_order(&q1, &q2)];
        quads.swap_draw_order(&mut q1, &mut q2);
        assert_eq![
            std::cmp::Ordering::Greater,
            quads.compare_draw_order(&q1, &q2)
        ];

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "swapping_quad_draw_order_different_layers", img);
    }

    #[test]
    fn three_quads_add_remove() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let _q1 = quads.add(&layer, quad);
        let q2 = quads.add(&layer, quad);
        let q3 = quads.add(&layer, quad);

        quads.translate(&q2, (0.25, 0.4));
        quads.set_solid_color(&q2, [0, 0, 255, 128]);

        quads.translate(&q3, (0.35, 0.8));
        quads.remove(q2);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "three_quads_add_remove", img);
    }

    #[test]
    fn simple_quad_set_position() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let handle = quads.add(&layer, quad);
        quads.set_translation(&handle, (0.25, 0.4));

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_set_position", img);
    }

    #[test]
    fn simple_quad_scale() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[2].2 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let handle = quads.add(&layer, quad);
        quads.set_scale(&handle, 0.5);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_scale", img);
    }

    #[test]
    fn simple_quad_deform() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[2].2 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let handle = quads.add(&layer, quad);
        quads.scale(&handle, 0.5);
        quads.deform(&handle, [(-0.5, 0.0), (0.0, 0.0), (0.0, 0.0), (0.5, 0.1)]);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_deform", img);
    }

    #[test]
    fn simple_quad_set_position_after_initial() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let mut quads = vx.quads();
        let layer = quads.add_layer(LayerOptions::default());
        let handle = quads.add(&layer, quad);

        for _ in 0..3 {
            vx.draw_frame(&prspect);
        }

        vx.quads().set_translation(&handle, (0.25, 0.4));

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_set_position_after_initial", img);
    }

    #[test]
    fn simple_quad_rotated_with_exotic_origin() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);

        let mut quad = quads::Quad::default();
        quad.scale = 0.2;
        quad.colors[0].0 = 255;
        quad.colors[3].0 = 255;

        let layer = vx.quads().add_layer(LayerOptions::default());
        vx.quads().add(&layer, quad);

        let mut quad = quads::Quad::default();
        quad.scale = 0.2;
        quad.origin = (-1.0, -1.0);
        quad.colors[0].1 = 255;
        quad.colors[3].1 = 255;

        let mut quads = vx.quads();
        quads.add(&layer, quad);

        // when
        quads.rotate_all(&layer, Deg(30.0));

        // then
        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "simple_quad_rotated_with_exotic_origin", img);
    }

    // DISABLED because we might disable depth buffering altogether
    // #[test]
    // fn overlapping_quads_respect_z_order() {
    //     let logger = Logger::<Generic>::spawn_void().to_logpass();
    //     let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
    //     let prspect = gen_perspective(&vx);
    //     let mut quad = quads::Quad {
    //         scale: 0.5,
    //         ..quads::Quad::default()
    //     };

    //     for i in 0..4 {
    //         quad.colors[i] = (0, 255, 0, 255);
    //     }
    //     quad.depth = 0.0;
    //     quad.translation = (0.25, 0.25);

    //     let layer = vx.quads().add_layer(LayerOptions {
    //         depth_test: true,
    //         ..LayerOptions::default()
    //     });
    //     vx.quads().add(&layer, quad);

    //     for i in 0..4 {
    //         quad.colors[i] = (255, 0, 0, 255);
    //     }
    //     quad.depth = 0.5;
    //     quad.translation = (0.0, 0.0);
    //     vx.quads().add(&layer, quad);

    //     let img = vx.draw_frame_copy_framebuffer(&prspect);
    //     utils::assert_swapchain_eq(&mut vx, "overlapping_quads_respect_z_order", img);

    //     // ---

    //     vx.quads().pop_n_quads(&layer, 2);

    //     // ---

    //     for i in 0..4 {
    //         quad.colors[i] = (255, 0, 0, 255);
    //     }
    //     quad.depth = 0.5;
    //     quad.translation = (0.0, 0.0);
    //     vx.quads().add(&layer, quad);

    //     for i in 0..4 {
    //         quad.colors[i] = (0, 255, 0, 255);
    //     }
    //     quad.depth = 0.0;
    //     quad.translation = (0.25, 0.25);
    //     vx.quads().add(&layer, quad);

    //     let img = vx.draw_frame_copy_framebuffer(&prspect);
    //     utils::assert_swapchain_eq(&mut vx, "overlapping_quads_respect_z_order", img);
    // }

    #[test]
    fn quad_layering() {
        let logger = Logger::<Generic>::spawn_void().to_logpass();
        let mut vx = VxDraw::new(logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&vx);
        let mut quad = quads::Quad {
            scale: 0.5,
            ..quads::Quad::default()
        };

        for i in 0..4 {
            quad.colors[i] = (0, 255, 0, 255);
        }
        quad.depth = 0.0;
        quad.translation = (0.25, 0.25);

        let layer1 = vx.quads().add_layer(LayerOptions::default());
        let layer2 = vx.quads().add_layer(LayerOptions::default());

        vx.quads().add(&layer2, quad);

        quad.scale = 0.6;
        for i in 0..4 {
            quad.colors[i] = (0, 0, 255, 255);
        }
        vx.quads().add(&layer1, quad);

        let img = vx.draw_frame_copy_framebuffer(&prspect);
        utils::assert_swapchain_eq(&mut vx, "quad_layering", img);
    }
}
