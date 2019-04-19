use super::utils::*;
use crate::glocals::vxdraw::{SingleTexture, Windowing};
use ::image as load_image;
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
    adapter::PhysicalDevice,
    command,
    device::Device,
    format, image, memory,
    memory::Properties,
    pass,
    pso::{self, DescriptorPool},
    Backend, Primitive,
};
use std::io::Read;
use std::mem::{size_of, ManuallyDrop};

// ---

/// A view into a texture
///
/// A sprite is a rectangular view into a texture.
#[derive(Clone, Copy)]
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

/// A view into a texture
pub struct SpriteHandle(usize);

/// Handle to a texture
pub struct TextureHandle(usize);

#[derive(Clone, Copy)]
pub struct TextureOptions {
    /// Perform depth testing (and fragment culling) when drawing sprites from this texture
    pub depth_test: bool,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self { depth_test: true }
    }
}

// ---

/// Add a texture to the system
pub fn add_texture(s: &mut Windowing, img_data: &[u8], options: TextureOptions) -> TextureHandle {
    let (texture_vertex_buffer, texture_vertex_memory, texture_vertex_requirements) =
        make_vertex_buffer_with_data(s, &[0f32; 10 * 4 * 1000]);

    let device = &s.device;

    let img = load_image::load_from_memory_with_format(&img_data[..], load_image::PNG)
        .unwrap()
        .to_rgba();

    let pixel_size = 4; //size_of::<image::Rgba<u8>>();
    let row_size = pixel_size * (img.width() as usize);
    let limits = s.adapter.physical_device.limits();
    let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
    let row_pitch = ((row_size as u32 + row_alignment_mask) & !row_alignment_mask) as usize;
    debug_assert!(row_pitch as usize >= row_size);
    let required_bytes = row_pitch * img.height() as usize;

    let mut image_upload_buffer = unsafe {
        device.create_buffer(required_bytes as u64, gfx_hal::buffer::Usage::TRANSFER_SRC)
    }
    .unwrap();
    let image_mem_reqs = unsafe { device.get_buffer_requirements(&image_upload_buffer) };
    let memory_type_id = find_memory_type_id(&s.adapter, image_mem_reqs, Properties::CPU_VISIBLE);
    let image_upload_memory =
        unsafe { device.allocate_memory(memory_type_id, image_mem_reqs.size) }.unwrap();
    unsafe { device.bind_buffer_memory(&image_upload_memory, 0, &mut image_upload_buffer) }
        .unwrap();

    unsafe {
        let mut writer = s
            .device
            .acquire_mapping_writer::<u8>(&image_upload_memory, 0..image_mem_reqs.size)
            .expect("Unable to get mapping writer");
        for y in 0..img.height() as usize {
            let row = &(*img)[y * row_size..(y + 1) * row_size];
            let dest_base = y * row_pitch;
            writer[dest_base..dest_base + row.len()].copy_from_slice(row);
        }
        device
            .release_mapping_writer(writer)
            .expect("Couldn't release the mapping writer to the staging buffer!");
    }

    let mut the_image = unsafe {
        device
            .create_image(
                image::Kind::D2(img.width(), img.height(), 1, 1),
                1,
                format::Format::Rgba8Srgb,
                image::Tiling::Optimal,
                image::Usage::TRANSFER_DST | image::Usage::SAMPLED,
                image::ViewCapabilities::empty(),
            )
            .expect("Couldn't create the image!")
    };

    let image_memory = unsafe {
        let requirements = device.get_image_requirements(&the_image);
        let memory_type_id =
            find_memory_type_id(&s.adapter, requirements, memory::Properties::DEVICE_LOCAL);
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

    unsafe {
        let mut cmd_buffer = s.command_pool.acquire_command_buffer::<command::OneShot>();
        cmd_buffer.begin();
        let image_barrier = memory::Barrier::Image {
            states: (image::Access::empty(), image::Layout::Undefined)
                ..(
                    image::Access::TRANSFER_WRITE,
                    image::Layout::TransferDstOptimal,
                ),
            target: &the_image,
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        cmd_buffer.pipeline_barrier(
            pso::PipelineStage::TOP_OF_PIPE..pso::PipelineStage::TRANSFER,
            memory::Dependencies::empty(),
            &[image_barrier],
        );
        cmd_buffer.copy_buffer_to_image(
            &image_upload_buffer,
            &the_image,
            image::Layout::TransferDstOptimal,
            &[command::BufferImageCopy {
                buffer_offset: 0,
                buffer_width: (row_pitch / pixel_size) as u32,
                buffer_height: img.height(),
                image_layers: gfx_hal::image::SubresourceLayers {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layers: 0..1,
                },
                image_offset: image::Offset { x: 0, y: 0, z: 0 },
                image_extent: image::Extent {
                    width: img.width(),
                    height: img.height(),
                    depth: 1,
                },
            }],
        );
        let image_barrier = memory::Barrier::Image {
            states: (
                image::Access::TRANSFER_WRITE,
                image::Layout::TransferDstOptimal,
            )
                ..(
                    image::Access::SHADER_READ,
                    image::Layout::ShaderReadOnlyOptimal,
                ),
            target: &the_image,
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        cmd_buffer.pipeline_barrier(
            pso::PipelineStage::TRANSFER..pso::PipelineStage::FRAGMENT_SHADER,
            memory::Dependencies::empty(),
            &[image_barrier],
        );
        cmd_buffer.finish();
        let upload_fence = s
            .device
            .create_fence(false)
            .expect("Couldn't create an upload fence!");
        s.queue_group.queues[0].submit_nosemaphores(Some(&cmd_buffer), Some(&upload_fence));
        s.device
            .wait_for_fence(&upload_fence, u64::max_value())
            .expect("Couldn't wait for the fence!");
        s.device.destroy_fence(upload_fence);
    }

    unsafe {
        device.destroy_buffer(image_upload_buffer);
        device.free_memory(image_upload_memory);
    }

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
                format: format::Format::Rgb32Float,
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
        depth: if options.depth_test {
            pso::DepthTest::On {
                fun: pso::Comparison::Less,
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
                descriptors: Some(pso::Descriptor::Image(
                    &image_view,
                    image::Layout::ShaderReadOnlyOptimal,
                )),
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

    let (
        texture_vertex_buffer_indices,
        texture_vertex_memory_indices,
        texture_vertex_requirements_indices,
    ) = make_index_buffer_with_data(s, &[0f32; 4 * 1000]);

    s.dyntexs.push(SingleTexture {
        count: 0,

        texture_vertex_buffer: ManuallyDrop::new(texture_vertex_buffer),
        texture_vertex_memory: ManuallyDrop::new(texture_vertex_memory),
        texture_vertex_requirements,

        texture_vertex_buffer_indices: ManuallyDrop::new(texture_vertex_buffer_indices),
        texture_vertex_memory_indices: ManuallyDrop::new(texture_vertex_memory_indices),
        texture_vertex_requirements_indices,

        texture_image_buffer: ManuallyDrop::new(the_image),
        texture_image_memory: ManuallyDrop::new(image_memory),

        descriptor_pool: ManuallyDrop::new(descriptor_pool),
        image_view: ManuallyDrop::new(image_view),
        sampler: ManuallyDrop::new(sampler),

        descriptor_set: ManuallyDrop::new(descriptor_set),
        descriptor_set_layouts: triangle_descriptor_set_layouts,
        pipeline: ManuallyDrop::new(triangle_pipeline),
        pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        render_pass: ManuallyDrop::new(triangle_render_pass),
    });
    TextureHandle(s.dyntexs.len() - 1)
}

/// Add a sprite (a rectangular view of a texture) to the system
pub fn add_sprite(s: &mut Windowing, sprite: Sprite, texture: &TextureHandle) -> SpriteHandle {
    let tex = &mut s.dyntexs[texture.0];
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
                &tex.texture_vertex_memory_indices,
                0..tex.texture_vertex_requirements_indices.size,
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
            .acquire_mapping_writer(
                &tex.texture_vertex_memory,
                0..tex.texture_vertex_requirements.size,
            )
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
    SpriteHandle((tex.count - 1) as usize)
}

// ---

/// Translate all sprites that depend on a given texture
pub fn sprite_translate_all(s: &mut Windowing, tex: &TextureHandle, dxdy: (f32, f32)) {
    let device = &s.device;
    if let Some(ref mut stex) = s.dyntexs.get(tex.0) {
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
                    &stex.texture_vertex_memory,
                    0..stex.texture_vertex_requirements.size,
                )
                .expect("Failed to acquire a memory writer!");
            let mut vertices = Vec::with_capacity(stex.count as usize);
            for i in 0..stex.count {
                let idx = (i * 10 * 4) as usize;
                let translation = &data_reader[idx + 5..idx + 7];
                vertices.push((translation[0], translation[1]));
            }
            device.release_mapping_reader(data_reader);

            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &stex.texture_vertex_memory,
                    0..stex.texture_vertex_requirements.size,
                )
                .expect("Failed to acquire a memory writer!");

            for (i, prev_dxdy) in vertices.iter().enumerate() {
                let mut idx = (i * 10 * 4) as usize;
                let new_dxdy = (prev_dxdy.0 + dxdy.0, prev_dxdy.1 + dxdy.1);
                data_target[idx + 5..idx + 7].copy_from_slice(&[new_dxdy.0, new_dxdy.1]);
                idx += 10;
                data_target[idx + 5..idx + 7].copy_from_slice(&[new_dxdy.0, new_dxdy.1]);
                idx += 10;
                data_target[idx + 5..idx + 7].copy_from_slice(&[new_dxdy.0, new_dxdy.1]);
                idx += 10;
                data_target[idx + 5..idx + 7].copy_from_slice(&[new_dxdy.0, new_dxdy.1]);
            }
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

/// Rotate all sprites that depend on a given texture
pub fn sprite_rotate_all<T: Copy + Into<Rad<f32>>>(s: &mut Windowing, tex: &TextureHandle, deg: T) {
    let device = &s.device;
    if let Some(ref mut stex) = s.dyntexs.get(tex.0) {
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
                    &stex.texture_vertex_memory,
                    0..stex.texture_vertex_requirements.size,
                )
                .expect("Failed to acquire a memory writer!");
            let mut vertices = Vec::<f32>::with_capacity(stex.count as usize);
            for i in 0..stex.count {
                let idx = (i * 10 * 4) as usize;
                let rotation = &data_reader[idx + 7..idx + 8];
                vertices.push(rotation[0]);
            }
            device.release_mapping_reader(data_reader);

            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &stex.texture_vertex_memory,
                    0..stex.texture_vertex_requirements.size,
                )
                .expect("Failed to acquire a memory writer!");

            for (i, vert) in vertices.iter().enumerate() {
                let mut idx = (i * 10 * 4) as usize;
                data_target[idx + 7..idx + 8].copy_from_slice(&[*vert + deg.into().0]);
                idx += 10;
                data_target[idx + 7..idx + 8].copy_from_slice(&[*vert + deg.into().0]);
                idx += 10;
                data_target[idx + 7..idx + 8].copy_from_slice(&[*vert + deg.into().0]);
                idx += 10;
                data_target[idx + 7..idx + 8].copy_from_slice(&[*vert + deg.into().0]);
            }
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

#[cfg(feature = "gfx_tests")]
#[cfg(test)]
mod tests {
    use crate::mediators::vxdraw::*;
    use cgmath::{Deg, Vector3};
    use rand::Rng;
    use rand_pcg::Pcg64Mcg as random;
    use std::f32::consts::PI;

    static LOGO: &[u8] = include_bytes!["../../../assets/images/logo.png"];
    static FOREST: &[u8] = include_bytes!["../../../assets/images/forest-light.png"];
    static TREE: &[u8] = include_bytes!["../../../assets/images/treetest.png"];

    #[test]
    fn overlapping_dyntex_respect_z_order() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let tree = add_texture(&mut windowing, TREE, TextureOptions::default());
        let logo = add_texture(&mut windowing, LOGO, TextureOptions::default());

        let sprite = Sprite {
            scale: 0.5,
            ..Sprite::default()
        };

        let sprite_tree = add_sprite(
            &mut windowing,
            Sprite {
                depth: 0.5,
                ..sprite
            },
            &tree,
        );
        let sprite_logo = add_sprite(
            &mut windowing,
            Sprite {
                depth: 0.6,
                translation: (0.25, 0.25),
                ..sprite
            },
            &logo,
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "overlapping_dyntex_respect_z_order", img);
    }

    #[test]
    fn simple_texture() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        add_sprite(&mut windowing, Sprite::default(), &tex);

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "simple_texture", img);
    }

    #[test]
    fn simple_texture_adheres_to_view() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        add_sprite(&mut windowing, Sprite::default(), &tex);

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "simple_texture_adheres_to_view", img);
    }

    #[test]
    fn colored_simple_texture() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        add_sprite(
            &mut windowing,
            Sprite {
                colors: [
                    (255, 1, 2, 255),
                    (0, 255, 0, 255),
                    (0, 0, 255, 100),
                    (255, 2, 1, 0),
                ],
                ..Sprite::default()
            },
            &tex,
        );

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "colored_simple_texture", img);
    }

    #[test]
    fn translated_texture() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(
            &mut windowing,
            LOGO,
            TextureOptions {
                depth_test: false,
                ..TextureOptions::default()
            },
        );

        let base = Sprite {
            width: 1.0,
            height: 1.0,
            ..Sprite::default()
        };

        add_sprite(
            &mut windowing,
            Sprite {
                translation: (-0.5, -0.5),
                rotation: 0.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (0.5, -0.5),
                rotation: PI / 4.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (-0.5, 0.5),
                rotation: PI / 2.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (0.5, 0.5),
                rotation: PI,
                ..base
            },
            &tex,
        );
        sprite_translate_all(&mut windowing, &tex, (0.25, 0.35));

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "translated_texture", img);
    }

    #[test]
    fn rotated_texture() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(
            &mut windowing,
            LOGO,
            TextureOptions {
                depth_test: false,
                ..TextureOptions::default()
            },
        );

        let base = Sprite {
            width: 1.0,
            height: 1.0,
            ..Sprite::default()
        };

        add_sprite(
            &mut windowing,
            Sprite {
                translation: (-0.5, -0.5),
                rotation: 0.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (0.5, -0.5),
                rotation: PI / 4.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (-0.5, 0.5),
                rotation: PI / 2.0,
                ..base
            },
            &tex,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (0.5, 0.5),
                rotation: PI,
                ..base
            },
            &tex,
        );
        sprite_rotate_all(&mut windowing, &tex, Deg(90.0));

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "rotated_texture", img);
    }

    #[test]
    fn many_sprites() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(
            &mut windowing,
            LOGO,
            TextureOptions {
                depth_test: false,
                ..TextureOptions::default()
            },
        );
        for i in 0..360 {
            add_sprite(
                &mut windowing,
                Sprite {
                    rotation: ((i * 10) as f32 / 180f32 * PI),
                    scale: 0.5,
                    ..Sprite::default()
                },
                &tex,
            );
        }

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "many_sprites", img);
    }

    #[test]
    fn three_layer_scene() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let options = TextureOptions {
            depth_test: false,
            ..TextureOptions::default()
        };
        let forest = add_texture(&mut windowing, FOREST, options);
        let player = add_texture(&mut windowing, LOGO, options);
        let tree = add_texture(&mut windowing, TREE, options);

        add_sprite(&mut windowing, Sprite::default(), &forest);
        add_sprite(
            &mut windowing,
            Sprite {
                scale: 0.4,
                ..Sprite::default()
            },
            &player,
        );
        add_sprite(
            &mut windowing,
            Sprite {
                translation: (-0.3, 0.0),
                scale: 0.4,
                ..Sprite::default()
            },
            &tree,
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        tests::assert_swapchain_eq(&mut windowing, "three_layer_scene", img);
    }

}
