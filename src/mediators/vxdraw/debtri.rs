use super::utils::*;
use crate::glocals::{
    vxdraw::{ColoredTriangleList, Windowing},
    Log,
};
use cgmath::Rad;
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
use gfx_hal::{
    device::Device,
    format, image, pass,
    pso::{
        self, AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState,
        ColorBlendDesc, ColorMask, DepthStencilDesc, DepthTest, DescriptorSetLayoutBinding,
        Element, Face, Factor, FrontFace, GraphicsPipelineDesc, InputAssemblerDesc, LogicOp,
        PipelineCreationFlags, PolygonMode, Rasterizer, ShaderStageFlags, StencilTest,
        VertexBufferDesc, Viewport,
    },
    Backend, Primitive,
};
use logger::Logger;
use std::io::Read;
use std::mem::{size_of, transmute, ManuallyDrop};

// ---

pub struct DebugTriangleHandle(usize);

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

const PTS_PER_TRI: usize = 3;
const XY_COMPNTS: usize = 2;
const COLOR_CMPNTS: usize = 4;
const DELTA_CMPNTS: usize = 2;
const ROT_CMPNTS: usize = 1;
const SCALE_CMPNTS: usize = 1;
const TRI_BYTE_SIZE: usize = PTS_PER_TRI
    * (size_of::<f32>() * XY_COMPNTS
        + size_of::<u8>() * COLOR_CMPNTS
        + size_of::<f32>() * DELTA_CMPNTS
        + size_of::<f32>() * ROT_CMPNTS
        + size_of::<f32>() * SCALE_CMPNTS);

// ---

pub fn debug_triangle_push(s: &mut Windowing, triangle: DebugTriangle) -> DebugTriangleHandle {
    let overrun = if let Some(ref mut debug_triangles) = s.debug_triangles {
        Some(
            (debug_triangles.triangles_count + 1) * TRI_BYTE_SIZE
                > debug_triangles.capacity as usize,
        )
    } else {
        None
    };
    if let Some(overrun) = overrun {
        // Do reallocation here
        assert_eq![false, overrun];
    }
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        let device = &s.device;
        unsafe {
            let mut data_target = device
                .acquire_mapping_writer(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");
            let idx = debug_triangles.triangles_count * TRI_BYTE_SIZE / size_of::<f32>();

            for (i, idx) in [idx, idx + 7, idx + 14].iter().enumerate() {
                data_target[*idx..*idx + 2]
                    .copy_from_slice(&[triangle.origin[i].0, triangle.origin[i].1]);
                data_target[*idx + 2..*idx + 3].copy_from_slice(&transmute::<[u8; 4], [f32; 1]>([
                    triangle.colors_rgba[i].0,
                    triangle.colors_rgba[i].1,
                    triangle.colors_rgba[i].2,
                    triangle.colors_rgba[i].3,
                ]));
                data_target[*idx + 3..*idx + 5]
                    .copy_from_slice(&[triangle.translation.0, triangle.translation.1]);
                data_target[*idx + 5..*idx + 6].copy_from_slice(&[triangle.rotation]);
                data_target[*idx + 6..*idx + 7].copy_from_slice(&[triangle.scale]);
            }
            debug_triangles.triangles_count += 1;
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
        DebugTriangleHandle(debug_triangles.triangles_count - 1)
    } else {
        unreachable![]
    }
}

pub fn debug_triangle_pop(s: &mut Windowing) {
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        unsafe {
            s.device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");
        }
        debug_triangles.triangles_count -= 1;
    }
}

pub fn pop_n_triangles(s: &mut Windowing, n: usize) {
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        unsafe {
            s.device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");
        }
        debug_triangles.triangles_count -= n;
    }
}

pub fn create_debug_triangle(s: &mut Windowing, log: &mut Logger<Log>) {
    pub const VERTEX_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout (location = 0) in vec2 position;
    layout (location = 1) in vec4 color;
    layout (location = 2) in vec2 dxdy;
    layout (location = 3) in float rotation;
    layout (location = 4) in float scale;

    layout(push_constant) uniform PushConstant {
        float w_over_h;
    } push_constant;

    layout (location = 0) out vec4 outcolor;
    out gl_PerVertex {
        vec4 gl_Position;
    };
    void main() {
        mat2 rotmatrix = mat2(cos(rotation), -sin(rotation), sin(rotation), cos(rotation));
        vec2 pos = rotmatrix * scale * position;
        if (push_constant.w_over_h >= 1.0) {
            pos.x /= push_constant.w_over_h;
        } else {
            pos.y *= push_constant.w_over_h;
        }
        gl_Position = vec4(pos + dxdy, 0.0, 1.0);
        outcolor = color;
    }";

    pub const FRAGMENT_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout(location = 0) in vec4 incolor;
    layout(location = 0) out vec4 color;
    void main() {
        color = incolor;
    }";

    let vs_module = {
        let glsl = VERTEX_SOURCE;
        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Vertex)
            .unwrap()
            .bytes()
            .map(Result::unwrap)
            .collect();
        unsafe { s.device.create_shader_module(&spirv) }.unwrap()
    };
    let fs_module = {
        let glsl = FRAGMENT_SOURCE;
        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Fragment)
            .unwrap()
            .bytes()
            .map(Result::unwrap)
            .collect();
        unsafe { s.device.create_shader_module(&spirv) }.unwrap()
    };

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
    let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);

    let vertex_buffers: Vec<VertexBufferDesc> = vec![VertexBufferDesc {
        binding: 0,
        stride: (TRI_BYTE_SIZE / PTS_PER_TRI) as u32,
        rate: 0,
    }];
    let attributes: Vec<AttributeDesc> = vec![
        AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: format::Format::Rg32Float,
                offset: 0,
            },
        },
        AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: format::Format::Rgba8Unorm,
                offset: 8,
            },
        },
        AttributeDesc {
            location: 2,
            binding: 0,
            element: Element {
                format: format::Format::Rg32Float,
                offset: 12,
            },
        },
        AttributeDesc {
            location: 3,
            binding: 0,
            element: Element {
                format: format::Format::R32Float,
                offset: 20,
            },
        },
        AttributeDesc {
            location: 4,
            binding: 0,
            element: Element {
                format: format::Format::R32Float,
                offset: 24,
            },
        },
    ];

    let rasterizer = Rasterizer {
        depth_clamping: false,
        polygon_mode: PolygonMode::Fill,
        cull_face: Face::NONE,
        front_face: FrontFace::Clockwise,
        depth_bias: None,
        conservative: false,
    };

    let depth_stencil = DepthStencilDesc {
        depth: DepthTest::Off,
        depth_bounds: false,
        stencil: StencilTest::Off,
    };
    let blender = {
        let blend_state = BlendState::On {
            color: BlendOp::Add {
                src: Factor::One,
                dst: Factor::Zero,
            },
            alpha: BlendOp::Add {
                src: Factor::One,
                dst: Factor::Zero,
            },
        };
        BlendDesc {
            logic_op: Some(LogicOp::Copy),
            targets: vec![ColorBlendDesc(ColorMask::ALL, blend_state)],
        }
    };
    let extent = image::Extent {
        width: s.swapconfig.extent.width,
        height: s.swapconfig.extent.height,
        depth: 1,
    }
    .rect();
    let triangle_render_pass = {
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

        let subpass = pass::SubpassDesc {
            colors: &[(0, image::Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        unsafe { s.device.create_render_pass(&[attachment], &[subpass], &[]) }
            .expect("Can't create render pass")
    };
    let baked_states = BakedStates {
        viewport: Some(Viewport {
            rect: extent,
            depth: (0.0..1.0),
        }),
        scissor: Some(extent),
        blend_color: None,
        depth_bounds: None,
    };
    let bindings = Vec::<DescriptorSetLayoutBinding>::new();
    let immutable_samplers = Vec::<<back::Backend as Backend>::Sampler>::new();
    let triangle_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            s.device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let mut push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((ShaderStageFlags::VERTEX, 0..1));

    let triangle_pipeline_layout = unsafe {
        s.device
            .create_pipeline_layout(&triangle_descriptor_set_layouts, push_constants)
            .expect("Couldn't create a pipeline layout")
    };

    // Describe the pipeline (rasterization, triangle interpretation)
    let pipeline_desc = GraphicsPipelineDesc {
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
        flags: PipelineCreationFlags::empty(),
        parent: BasePipeline::None,
    };

    let triangle_pipeline = unsafe {
        s.device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Couldn't create a graphics pipeline!")
    };

    unsafe {
        s.device.destroy_shader_module(vs_module);
    }
    unsafe {
        s.device.destroy_shader_module(fs_module);
    }
    let (dtbuffer, dtmemory, dtreqs) =
        make_vertex_buffer_with_data(s, &[0.0f32; TRI_BYTE_SIZE / 4 * 1000]);
    let debug_triangles = ColoredTriangleList {
        capacity: dtreqs.size,
        triangles_count: 0,
        triangles_buffer: dtbuffer,
        triangles_memory: dtmemory,
        memory_requirements: dtreqs,

        descriptor_set: triangle_descriptor_set_layouts,
        pipeline: ManuallyDrop::new(triangle_pipeline),
        pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        render_pass: ManuallyDrop::new(triangle_render_pass),
    };
    s.debug_triangles = Some(debug_triangles);
}

pub fn debug_triangle_rotate_all<T: Copy + Into<Rad<f32>>>(s: &mut Windowing, deg: T) {
    let device = &s.device;
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        unsafe {
            device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");
        }
        unsafe {
            let data_reader = device
                .acquire_mapping_reader::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");
            let mut vertices = Vec::<f32>::with_capacity(debug_triangles.triangles_count);
            for i in 0..debug_triangles.triangles_count {
                let idx = i * TRI_BYTE_SIZE / size_of::<f32>();
                let rotation = &data_reader[idx + 5..idx + 6];
                vertices.push(rotation[0]);
            }
            device.release_mapping_reader(data_reader);

            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");

            for (i, vert) in vertices.iter().enumerate() {
                let mut idx = i * TRI_BYTE_SIZE / size_of::<f32>();
                data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
                idx += 7;
                data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
                idx += 7;
                data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
            }
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

pub fn set_triangle_color(s: &mut Windowing, inst: &DebugTriangleHandle, rgba: [u8; 4]) {
    let inst = inst.0;
    let device = &s.device;
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        unsafe {
            device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");
        }
        unsafe {
            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");

            let mut idx = inst * TRI_BYTE_SIZE / size_of::<f32>();
            let rgba = &transmute::<[u8; 4], [f32; 1]>(rgba);
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            idx += 7;
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            idx += 7;
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}
