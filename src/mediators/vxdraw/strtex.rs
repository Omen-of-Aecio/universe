use super::utils::*;
use crate::glocals::{
    vxdraw::{StreamingTexture, Windowing},
    Log,
};
use ::image as load_image;
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
    format, image, memory, pass,
    pso::{self, DescriptorPool},
    Backend, Primitive,
};
use logger::{debug, Logger};
use std::io::Read;
use std::mem::{size_of, ManuallyDrop};

// ---

pub struct Sprite {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub colors: [(u8, u8, u8, u8); 4],
    pub uv_begin: (f32, f32),
    pub uv_end: (f32, f32),
    pub translation: (f32, f32),
    pub rotation: f32,
    pub scale: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite {
            width: 2.0,
            height: 2.0,
            depth: 0.0,
            colors: [(0, 0, 0, 255); 4],
            uv_begin: (0.0, 0.0),
            uv_end: (1.0, 1.0),
            translation: (0.0, 0.0),
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

// ---

pub fn add_streaming_texture(
    s: &mut Windowing,
    w: usize,
    h: usize,
    log: &mut Logger<Log>,
) -> usize {
    let (texture_vertex_buffer, texture_vertex_memory, vertex_requirements) =
        make_vertex_buffer_with_data(s, &[0f32; 9 * 4 * 1000]);

    let device = &s.device;

    let mut the_image = unsafe {
        device
            .create_image(
                image::Kind::D2(w as u32, h as u32, 1, 1),
                1,
                format::Format::Rgba8Srgb,
                image::Tiling::Linear,
                image::Usage::SAMPLED,
                image::ViewCapabilities::empty(),
            )
            .expect("Couldn't create the image!")
    };

    let requirements = unsafe { device.get_image_requirements(&the_image) };
    let image_memory = unsafe {
        let memory_type_id = find_memory_type_id(
            &s.adapter,
            requirements,
            memory::Properties::CPU_VISIBLE | memory::Properties::COHERENT,
        );
        device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Unable to allocate")
    };

    let image_view = unsafe {
        device
            .bind_image_memory(&image_memory, 0, &mut the_image)
            .expect("Unable to bind memory");

        device
            .create_image_view(
                &the_image,
                image::ViewKind::D2,
                format::Format::Rgba8Srgb,
                format::Swizzle::NO,
                image::SubresourceRange {
                    aspects: format::Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            )
            .expect("Couldn't create the image view!")
    };

    let sampler = unsafe {
        s.device
            .create_sampler(image::SamplerInfo::new(
                image::Filter::Nearest,
                image::WrapMode::Tile,
            ))
            .expect("Couldn't create the sampler!")
    };

    const VERTEX_SOURCE_TEXTURE: &str = "#version 450
    #extension GL_ARB_separate_shader_objects : enable

    layout(location = 0) in vec3 v_pos;
    layout(location = 1) in vec2 v_uv;
    layout(location = 2) in vec2 v_dxdy;
    layout(location = 3) in float rotation;
    layout(location = 4) in float scale;
    layout(location = 5) in vec4 color;

    layout(location = 0) out vec2 f_uv;
    layout(location = 1) out vec4 f_color;

    layout(push_constant) uniform PushConstant {
        mat4 view;
    } push_constant;

    out gl_PerVertex {
        vec4 gl_Position;
    };

    void main() {
        mat2 rotmatrix = mat2(cos(rotation), -sin(rotation), sin(rotation), cos(rotation));
        vec2 pos = rotmatrix * scale * v_pos.xy;
        f_uv = v_uv;
        f_color = color;
        gl_Position = push_constant.view * vec4(pos + v_dxdy, v_pos.z, 1.0);
    }";

    const FRAGMENT_SOURCE_TEXTURE: &str = "#version 450
    #extension GL_ARB_separate_shader_objects : enable

    layout(location = 0) in vec2 f_uv;
    layout(location = 1) in vec4 f_color;

    layout(location = 0) out vec4 color;

    layout(set = 0, binding = 0) uniform texture2D f_texture;
    layout(set = 0, binding = 1) uniform sampler f_sampler;

    void main() {
        color = texture(sampler2D(f_texture, f_sampler), f_uv);
        color.a *= f_color.a;
        color.rgb += f_color.rgb;
    }";

    let vs_module = {
        let glsl = VERTEX_SOURCE_TEXTURE;
        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Vertex)
            .unwrap()
            .bytes()
            .map(Result::unwrap)
            .collect();
        unsafe { s.device.create_shader_module(&spirv) }.unwrap()
    };
    let fs_module = {
        let glsl = FRAGMENT_SOURCE_TEXTURE;
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
    debug![log, "vxdraw", "After making"];
    let shader_entries = pso::GraphicsShaderSet {
        vertex: vs_entry,
        hull: None,
        domain: None,
        geometry: None,
        fragment: Some(fs_entry),
    };
    let input_assembler = pso::InputAssemblerDesc::new(Primitive::TriangleList);

    let vertex_buffers: Vec<pso::VertexBufferDesc> = vec![pso::VertexBufferDesc {
        binding: 0,
        stride: (size_of::<f32>() * (3 + 2 + 2 + 2 + 1)) as u32,
        rate: 0,
    }];
    let attributes: Vec<pso::AttributeDesc> = vec![
        pso::AttributeDesc {
            location: 0,
            binding: 0,
            element: pso::Element {
                format: format::Format::Rg32Float,
                offset: 0,
            },
        },
        pso::AttributeDesc {
            location: 1,
            binding: 0,
            element: pso::Element {
                format: format::Format::Rg32Float,
                offset: 12,
            },
        },
        pso::AttributeDesc {
            location: 2,
            binding: 0,
            element: pso::Element {
                format: format::Format::Rg32Float,
                offset: 20,
            },
        },
        pso::AttributeDesc {
            location: 3,
            binding: 0,
            element: pso::Element {
                format: format::Format::R32Float,
                offset: 28,
            },
        },
        pso::AttributeDesc {
            location: 4,
            binding: 0,
            element: pso::Element {
                format: format::Format::R32Float,
                offset: 32,
            },
        },
        pso::AttributeDesc {
            location: 5,
            binding: 0,
            element: pso::Element {
                format: format::Format::Rgba8Unorm,
                offset: 36,
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
        depth: pso::DepthTest::On {
            fun: pso::Comparison::Less,
            write: true,
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
    let baked_states = pso::BakedStates {
        viewport: Some(pso::Viewport {
            rect: extent,
            depth: (0.0..1.0),
        }),
        scissor: Some(extent),
        blend_color: None,
        depth_bounds: None,
    };
    let mut bindings = Vec::<pso::DescriptorSetLayoutBinding>::new();
    bindings.push(pso::DescriptorSetLayoutBinding {
        binding: 0,
        ty: pso::DescriptorType::SampledImage,
        count: 1,
        stage_flags: pso::ShaderStageFlags::FRAGMENT,
        immutable_samplers: false,
    });
    bindings.push(pso::DescriptorSetLayoutBinding {
        binding: 1,
        ty: pso::DescriptorType::Sampler,
        count: 1,
        stage_flags: pso::ShaderStageFlags::FRAGMENT,
        immutable_samplers: false,
    });
    let immutable_samplers = Vec::<<back::Backend as Backend>::Sampler>::new();
    let triangle_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            s.device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];

    let mut descriptor_pool = unsafe {
        s.device
            .create_descriptor_pool(
                1, // sets
                &[
                    pso::DescriptorRangeDesc {
                        ty: pso::DescriptorType::SampledImage,
                        count: 1,
                    },
                    pso::DescriptorRangeDesc {
                        ty: pso::DescriptorType::Sampler,
                        count: 1,
                    },
                ],
            )
            .expect("Couldn't create a descriptor pool!")
    };

    let descriptor_set = unsafe {
        descriptor_pool
            .allocate_set(&triangle_descriptor_set_layouts[0])
            .expect("Couldn't make a Descriptor Set!")
    };

    unsafe {
        s.device.write_descriptor_sets(vec![
            pso::DescriptorSetWrite {
                set: &descriptor_set,
                binding: 0,
                array_offset: 0,
                descriptors: Some(pso::Descriptor::Image(&image_view, image::Layout::General)),
            },
            pso::DescriptorSetWrite {
                set: &descriptor_set,
                binding: 1,
                array_offset: 0,
                descriptors: Some(pso::Descriptor::Sampler(&sampler)),
            },
        ]);
    }

    let mut push_constants = Vec::<(pso::ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((pso::ShaderStageFlags::VERTEX, 0..16));
    let triangle_pipeline_layout = unsafe {
        s.device
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

    let (vertex_buffer_indices, vertex_memory_indices, vertex_requirements_indices) =
        make_index_buffer_with_data(s, &[0f32; 4 * 1000]);

    unsafe {
        let barrier_fence = s.device.create_fence(false).expect("unable to make fence");
        // TODO Use a proper command buffer here
        s.device.wait_idle().unwrap();
        let buffer = &mut s.command_buffers[s.current_frame];
        buffer.begin(false);
        {
            let image_barrier = memory::Barrier::Image {
                states: (image::Access::empty(), image::Layout::Undefined)
                    ..(
                        // image::Access::HOST_READ | image::Access::HOST_WRITE,
                        image::Access::empty(),
                        image::Layout::General,
                    ),
                target: &the_image,
                families: None,
                range: image::SubresourceRange {
                    aspects: format::Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            };
            buffer.pipeline_barrier(
                pso::PipelineStage::TOP_OF_PIPE..pso::PipelineStage::HOST,
                memory::Dependencies::empty(),
                &[image_barrier],
            );
        }
        buffer.finish();
        s.queue_group.queues[0].submit_nosemaphores(Some(&*buffer), Some(&barrier_fence));
        s.device
            .wait_for_fence(&barrier_fence, u64::max_value())
            .unwrap();
        s.device.destroy_fence(barrier_fence);
    }

    s.strtexs.push(StreamingTexture {
        count: 0,

        width: w as u32,
        height: h as u32,

        vertex_buffer: ManuallyDrop::new(texture_vertex_buffer),
        vertex_memory: ManuallyDrop::new(texture_vertex_memory),
        vertex_requirements,

        vertex_buffer_indices: ManuallyDrop::new(vertex_buffer_indices),
        vertex_memory_indices: ManuallyDrop::new(vertex_memory_indices),
        vertex_requirements_indices,

        image_buffer: ManuallyDrop::new(the_image),
        image_memory: ManuallyDrop::new(image_memory),
        image_requirements: requirements,

        descriptor_pool: ManuallyDrop::new(descriptor_pool),
        image_view: ManuallyDrop::new(image_view),
        sampler: ManuallyDrop::new(sampler),

        descriptor_set: ManuallyDrop::new(descriptor_set),
        descriptor_set_layouts: triangle_descriptor_set_layouts,
        pipeline: ManuallyDrop::new(triangle_pipeline),
        pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        render_pass: ManuallyDrop::new(triangle_render_pass),
    });
    s.strtexs.len() - 1
}

/// Add a sprite (a rectangular view of a texture) to the system
pub fn streaming_texture_add_sprite(s: &mut Windowing, sprite: Sprite, texture: usize) -> usize {
    let tex = &mut s.strtexs[texture];
    let device = &s.device;

    // Derive xy from the sprite's initial UV
    let uv_a = sprite.uv_begin;
    let uv_b = sprite.uv_end;

    let width = sprite.width;
    let height = sprite.height;

    let topleft = (-width / 2f32, -height / 2f32);
    let topleft_uv = uv_a;

    let topright = (width / 2f32, -height / 2f32);
    let topright_uv = (uv_b.0, uv_a.1);

    let bottomleft = (-width / 2f32, height / 2f32);
    let bottomleft_uv = (uv_a.0, uv_b.1);

    let bottomright = (width / 2f32, height / 2f32);
    let bottomright_uv = (uv_b.0, uv_b.1);

    unsafe {
        let mut data_target = device
            .acquire_mapping_writer(
                &tex.vertex_memory_indices,
                0..tex.vertex_requirements_indices.size,
            )
            .expect("Failed to acquire a memory writer!");
        let ver = (tex.count * 6) as u16;
        let ind = (tex.count * 4) as u16;
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
    }
    unsafe {
        let mut data_target = device
            .acquire_mapping_writer(&tex.vertex_memory, 0..tex.vertex_requirements.size)
            .expect("Failed to acquire a memory writer!");
        let idx = (tex.count * 4 * 10) as usize;

        for (i, (point, uv)) in [
            (topleft, topleft_uv),
            (bottomleft, bottomleft_uv),
            (bottomright, bottomright_uv),
            (topright, topright_uv),
        ]
        .iter()
        .enumerate()
        {
            let idx = idx + i * 10;
            data_target[idx..idx + 3].copy_from_slice(&[point.0, point.1, sprite.depth]);
            data_target[idx + 3..idx + 5].copy_from_slice(&[uv.0, uv.1]);
            data_target[idx + 5..idx + 7]
                .copy_from_slice(&[sprite.translation.0, sprite.translation.1]);
            data_target[idx + 7..idx + 8].copy_from_slice(&[sprite.rotation]);
            data_target[idx + 8..idx + 9].copy_from_slice(&[sprite.scale]);
            data_target[idx + 9..idx + 10]
                .copy_from_slice(&[std::mem::transmute::<_, f32>(sprite.colors[i])]);
        }
        tex.count += 1;
        device
            .release_mapping_writer(data_target)
            .expect("Couldn't release the mapping writer!");
    }
    (tex.count - 1) as usize
}

// ---

pub fn streaming_texture_set_pixels(
    s: &mut Windowing,
    id: usize,
    modifier: impl Iterator<Item = (u32, u32, (u8, u8, u8, u8))>,
) {
    if let Some(ref strtex) = s.strtexs.get(id) {
        unsafe {
            let foot = s.device.get_image_subresource_footprint(
                &strtex.image_buffer,
                image::Subresource {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layer: 0,
                },
            );
            for item in modifier {
                let w = item.0;
                let h = item.1;
                let color = item.2;

                if w > strtex.width || h > strtex.height {
                    continue;
                }

                let access = foot.row_pitch * h as u64 + (w * 4) as u64;
                let align = align_top(Alignment(strtex.image_requirements.alignment), access + 4);

                s.device
                    .wait_for_fences(
                        &s.frames_in_flight_fences,
                        gfx_hal::device::WaitFor::All,
                        u64::max_value(),
                    )
                    .expect("Unable to wait for fences");

                let mut target = s
                    .device
                    .acquire_mapping_writer(&*strtex.image_memory, access..align)
                    .expect("unable to acquire mapping writer");
                target[0..4].copy_from_slice(&[color.0, color.1, color.2, color.3]);
                s.device
                    .release_mapping_writer(target)
                    .expect("Unable to release mapping writer");
            }
        }
    }
}

pub fn streaming_texture_set_pixels_block(
    s: &mut Windowing,
    id: usize,
    start: (u32, u32),
    wh: (u32, u32),
    color: (u8, u8, u8, u8),
) {
    if let Some(ref strtex) = s.strtexs.get(id) {
        if start.0 + wh.0 > strtex.width || start.1 + wh.1 > strtex.height {
            return;
        }
        unsafe {
            let foot = s.device.get_image_subresource_footprint(
                &strtex.image_buffer,
                image::Subresource {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layer: 0,
                },
            );

            // Vulkan 01390, Size must be a multiple of DeviceLimits:nonCoherentAtomSize, or offset
            // plus size = size of memory, if it's not VK_WHOLE_SIZE
            let access_begin = foot.row_pitch * start.1 as u64 + (start.0 * 4) as u64;
            let access_end = foot.row_pitch
                * (start.1 + if wh.1 == 0 { 0 } else { wh.1 - 1 }) as u64
                + ((start.0 + wh.0) * 4) as u64;

            debug_assert![access_end <= strtex.image_requirements.size];

            let aligned = perfect_mapping_alignment(Align {
                access_offset: access_begin,
                how_many_bytes_you_need: access_end - access_begin,
                non_coherent_atom_size: s.device_limits.non_coherent_atom_size as u64,
                memory_size: strtex.image_requirements.size,
            });

            s.device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");

            let mut target = s
                .device
                .acquire_mapping_writer::<u8>(&*strtex.image_memory, aligned.begin..aligned.end)
                .expect("unable to acquire mapping writer");

            let mut colbuff = vec![];
            for _ in start.0..start.0 + wh.0 {
                colbuff.extend(&[color.0, color.1, color.2, color.3]);
            }

            for idx in start.1..start.1 + wh.1 {
                let idx = (idx - start.1) as usize;
                let pitch = foot.row_pitch as usize;
                target[aligned.index_offset as usize + idx * pitch
                    ..aligned.index_offset as usize + idx * pitch + (wh.0) as usize * 4]
                    .copy_from_slice(&colbuff);
            }
            s.device
                .release_mapping_writer(target)
                .expect("Unable to release mapping writer");
        }
    }
}

pub fn streaming_texture_set_pixel(
    s: &mut Windowing,
    id: usize,
    w: u32,
    h: u32,
    color: (u8, u8, u8, u8),
) {
    if let Some(ref strtex) = s.strtexs.get(id) {
        if !(w < strtex.width && h < strtex.height) {
            return;
        }
        unsafe {
            let foot = s.device.get_image_subresource_footprint(
                &strtex.image_buffer,
                image::Subresource {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layer: 0,
                },
            );
            let access = foot.row_pitch * h as u64 + (w * 4) as u64;

            let aligned = perfect_mapping_alignment(Align {
                access_offset: access,
                how_many_bytes_you_need: 4,
                non_coherent_atom_size: s.device_limits.non_coherent_atom_size as u64,
                memory_size: strtex.image_requirements.size,
            });

            s.device
                .wait_for_fences(
                    &s.frames_in_flight_fences,
                    gfx_hal::device::WaitFor::All,
                    u64::max_value(),
                )
                .expect("Unable to wait for fences");

            let mut target = s
                .device
                .acquire_mapping_writer(&*strtex.image_memory, aligned.begin..aligned.end)
                .expect("unable to acquire mapping writer");

            target[aligned.index_offset as usize..(aligned.index_offset + 4) as usize]
                .copy_from_slice(&[color.0, color.1, color.2, color.3]);

            s.device
                .release_mapping_writer(target)
                .expect("Unable to release mapping writer");
        }
    }
}
