use super::utils::*;
use crate::glocals::{
    vxdraw::{ColoredQuadList, Windowing},
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

pub struct QuadHandle(usize);

#[derive(Clone, Copy)]
pub struct Quad {
    pub width: f32,
    pub height: f32,
    pub colors: [(u8, u8, u8, u8); 4],
    pub translation: (f32, f32),
    pub rotation: f32,
    pub scale: f32,
}

impl Default for Quad {
    fn default() -> Self {
        Quad {
            width: 2.0,
            height: 2.0,
            colors: [(0, 0, 0, 255); 4],
            translation: (0.0, 0.0),
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

// ---

const PTS_PER_QUAD: usize = 4;
const XY_COMPNTS: usize = 2;
const COLOR_CMPNTS: usize = 4;
const DELTA_CMPNTS: usize = 2;
const ROT_CMPNTS: usize = 1;
const SCALE_CMPNTS: usize = 1;
const QUAD_BYTE_SIZE: usize = PTS_PER_QUAD
    * (size_of::<f32>() * XY_COMPNTS
        + size_of::<u8>() * COLOR_CMPNTS
        + size_of::<f32>() * DELTA_CMPNTS
        + size_of::<f32>() * ROT_CMPNTS
        + size_of::<f32>() * SCALE_CMPNTS);

// ---

pub fn quad_push(s: &mut Windowing, quad: Quad) -> QuadHandle {
    let overrun = if let Some(ref mut quads) = s.quads {
        Some((quads.count + 1) * QUAD_BYTE_SIZE > quads.capacity as usize)
    } else {
        None
    };
    if let Some(overrun) = overrun {
        // Do reallocation here
        assert_eq![false, overrun];
    }
    if let Some(ref mut quads) = s.quads {
        let device = &s.device;

        let width = quad.width;
        let height = quad.height;

        let topleft = (-width / 2f32, -height / 2f32);
        let topright = (width / 2f32, -height / 2f32);
        let bottomleft = (-width / 2f32, height / 2f32);
        let bottomright = (width / 2f32, height / 2f32);

        unsafe {
            let mut data_target = device
                .acquire_mapping_writer(
                    &quads.quads_memory_indices,
                    0..quads.quads_requirements_indices.size,
                )
                .expect("Failed to acquire a memory writer!");
            let ver = (quads.count * 6) as u16;
            let ind = (quads.count * 4) as u16;
            data_target[ver as usize..(ver + 6) as usize].copy_from_slice(&[
                ind,
                ind + 1,
                ind + 2,
                ind + 2,
                ind + 3,
                ind,
            ]);
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
            let mut data_target = device
                .acquire_mapping_writer(&quads.quads_memory, 0..quads.memory_requirements.size)
                .expect("Failed to acquire a memory writer!");
            let idx = quads.count * QUAD_BYTE_SIZE / size_of::<f32>();

            for (i, point) in [topleft, bottomleft, bottomright, topright]
                .iter()
                .enumerate()
            {
                // pub width: f32,
                // pub height: f32,
                // pub colors: [(u8, u8, u8, u8); 4],
                // pub translation: (f32, f32),
                // pub rotation: f32,
                // pub scale: f32,
                let idx = i * 7;

                data_target[idx..idx + 2].copy_from_slice(&[point.0, point.1]);
                data_target[idx + 2..idx + 3].copy_from_slice(&transmute::<[u8; 4], [f32; 1]>([
                    quad.colors[i].0,
                    quad.colors[i].1,
                    quad.colors[i].2,
                    quad.colors[i].3,
                ]));
                data_target[idx + 3..idx + 5]
                    .copy_from_slice(&[quad.translation.0, quad.translation.1]);
                data_target[idx + 5..idx + 6].copy_from_slice(&[quad.rotation]);
                data_target[idx + 6..idx + 7].copy_from_slice(&[quad.scale]);
            }
            quads.count += 1;
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
        QuadHandle(quads.count - 1)
    } else {
        unreachable![]
    }
}

pub fn quad_pop(s: &mut Windowing) {
    unimplemented![]
    // if let Some(ref mut quads) = s.quads {
    //     unsafe {
    //         s.device
    //             .wait_for_fences(
    //                 &s.frames_in_flight_fences,
    //                 gfx_hal::device::WaitFor::All,
    //                 u64::max_value(),
    //             )
    //             .expect("Unable to wait for fences");
    //     }
    //     quads.count -= 1;
    // }
}

pub fn pop_n_quads(s: &mut Windowing, n: usize) {
    unimplemented!()
    // if let Some(ref mut quads) = s.quads {
    //     unsafe {
    //         s.device
    //             .wait_for_fences(
    //                 &s.frames_in_flight_fences,
    //                 gfx_hal::device::WaitFor::All,
    //                 u64::max_value(),
    //             )
    //             .expect("Unable to wait for fences");
    //     }
    //     quads.count -= n;
    // }
}

pub fn create_quad(s: &mut Windowing, log: &mut Logger<Log>) {
    pub const VERTEX_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout (location = 0) in vec2 position;
    layout (location = 1) in vec4 color;
    layout (location = 2) in vec2 dxdy;
    layout (location = 3) in float rotation;
    layout (location = 4) in float scale;

    layout(push_constant) uniform PushConstant {
        mat4 view;
    } push_constant;

    layout (location = 0) out vec4 outcolor;

    out gl_PerVertex {
        vec4 gl_Position;
    };
    void main() {
        mat2 rotmatrix = mat2(cos(rotation), -sin(rotation), sin(rotation), cos(rotation));
        vec2 pos = rotmatrix * scale * position;
        gl_Position = push_constant.view * vec4(pos + dxdy, 0.0, 1.0);
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
        stride: (QUAD_BYTE_SIZE / PTS_PER_QUAD) as u32,
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
        depth: pso::DepthTest::On {
            fun: pso::Comparison::Less,
            write: true,
        },
        depth_bounds: false,
        stencil: StencilTest::Off,
    };
    let blender = {
        let blend_state = BlendState::On {
            color: BlendOp::Add {
                src: pso::Factor::SrcAlpha,
                dst: pso::Factor::OneMinusSrcAlpha,
            },
            alpha: pso::BlendOp::Add {
                src: pso::Factor::One,
                dst: pso::Factor::OneMinusSrcAlpha,
            },
            // alpha: pso::BlendOp::Max,
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
            format: Some(format::Format::D32Float),
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
    let quad_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            s.device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let mut push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((ShaderStageFlags::VERTEX, 0..16));

    let quad_pipeline_layout = unsafe {
        s.device
            .create_pipeline_layout(&quad_descriptor_set_layouts, push_constants)
            .expect("Couldn't create a pipeline layout")
    };

    // Describe the pipeline (rasterization, quad interpretation)
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
        layout: &quad_pipeline_layout,
        subpass: pass::Subpass {
            index: 0,
            main_pass: &quad_render_pass,
        },
        flags: PipelineCreationFlags::empty(),
        parent: BasePipeline::None,
    };

    let quad_pipeline = unsafe {
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
        make_vertex_buffer_with_data(s, &[0.0f32; QUAD_BYTE_SIZE / 4 * 1000]);

    let (quads_buffer_indices, quads_memory_indices, quads_requirements_indices) =
        make_index_buffer_with_data(s, &[0f32; 4 * 1000]);

    let quads = ColoredQuadList {
        capacity: dtreqs.size,
        count: 0,
        quads_buffer: dtbuffer,
        quads_memory: dtmemory,
        memory_requirements: dtreqs,

        quads_buffer_indices: quads_buffer_indices,
        quads_memory_indices: quads_memory_indices,
        quads_requirements_indices,

        descriptor_set: quad_descriptor_set_layouts,
        pipeline: ManuallyDrop::new(quad_pipeline),
        pipeline_layout: ManuallyDrop::new(quad_pipeline_layout),
        render_pass: ManuallyDrop::new(quad_render_pass),
    };
    s.quads = Some(quads);
}

pub fn quad_rotate_all<T: Copy + Into<Rad<f32>>>(s: &mut Windowing, deg: T) {
    unimplemented![]
    // let device = &s.device;
    // if let Some(ref mut quads) = s.quads {
    //     unsafe {
    //         device
    //             .wait_for_fences(
    //                 &s.frames_in_flight_fences,
    //                 gfx_hal::device::WaitFor::All,
    //                 u64::max_value(),
    //             )
    //             .expect("Unable to wait for fences");
    //     }
    //     unsafe {
    //         let data_reader = device
    //             .acquire_mapping_reader::<f32>(
    //                 &quads.quads_memory,
    //                 0..quads.capacity,
    //             )
    //             .expect("Failed to acquire a memory writer!");
    //         let mut vertices = Vec::<f32>::with_capacity(quads.count);
    //         for i in 0..quads.count {
    //             let idx = i * QUAD_BYTE_SIZE / size_of::<f32>();
    //             let rotation = &data_reader[idx + 5..idx + 6];
    //             vertices.push(rotation[0]);
    //         }
    //         device.release_mapping_reader(data_reader);

    //         let mut data_target = device
    //             .acquire_mapping_writer::<f32>(
    //                 &quads.quads_memory,
    //                 0..quads.capacity,
    //             )
    //             .expect("Failed to acquire a memory writer!");

    //         for (i, vert) in vertices.iter().enumerate() {
    //             let mut idx = i * QUAD_BYTE_SIZE / size_of::<f32>();
    //             data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
    //             idx += 7;
    //             data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
    //             idx += 7;
    //             data_target[idx + 5..idx + 6].copy_from_slice(&[*vert + deg.into().0]);
    //         }
    //         device
    //             .release_mapping_writer(data_target)
    //             .expect("Couldn't release the mapping writer!");
    //     }
    // }
}

pub fn set_quad_color(s: &mut Windowing, inst: &QuadHandle, rgba: [u8; 4]) {
    unimplemented![]
    // let inst = inst.0;
    // let device = &s.device;
    // if let Some(ref mut quads) = s.quads.get(inst.0) {
    //     unsafe {
    //         device
    //             .wait_for_fences(
    //                 &s.frames_in_flight_fences,
    //                 gfx_hal::device::WaitFor::All,
    //                 u64::max_value(),
    //             )
    //             .expect("Unable to wait for fences");
    //     }
    //     unsafe {
    //         let mut data_target = device
    //             .acquire_mapping_writer::<f32>(
    //                 &quads.quads_memory,
    //                 0..quads.capacity,
    //             )
    //             .expect("Failed to acquire a memory writer!");

    //         let mut idx = inst * QUAD_BYTE_SIZE / size_of::<f32>();
    //         let rgba = &transmute::<[u8; 4], [f32; 1]>(rgba);
    //         data_target[idx + 2..idx + 3].copy_from_slice(rgba);
    //         idx += 7;
    //         data_target[idx + 2..idx + 3].copy_from_slice(rgba);
    //         idx += 7;
    //         data_target[idx + 2..idx + 3].copy_from_slice(rgba);
    //         device
    //             .release_mapping_writer(data_target)
    //             .expect("Couldn't release the mapping writer!");
    //     }
    // }
}
