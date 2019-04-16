use crate::glocals::{Log, SingleTexture, Windowing};
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
// use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use ::image as load_image;
use arrayvec::ArrayVec;
use cgmath::prelude::*;
use cgmath::Matrix4;
use gfx_hal::{
    adapter::PhysicalDevice,
    command::{self, ClearColor, ClearValue},
    device::Device,
    format::{self, ChannelType, Swizzle},
    image, memory,
    memory::Properties,
    pass, pool,
    pso::{
        self, AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState,
        ColorBlendDesc, ColorMask, DepthStencilDesc, DepthTest, DescriptorPool,
        DescriptorSetLayoutBinding, Element, Face, Factor, FrontFace, GraphicsPipelineDesc,
        InputAssemblerDesc, LogicOp, PipelineCreationFlags, PipelineStage, PolygonMode, Rasterizer,
        Rect, ShaderStageFlags, StencilTest, VertexBufferDesc, Viewport,
    },
    queue::Submission,
    window::{Extent2D, PresentMode::*, Surface, Swapchain},
    Backbuffer, Backend, FrameSync, Instance, Primitive, SwapchainConfig,
};
use logger::{debug, info, log, trace, warn, InDebug, InDebugPretty, InHex, Logger};
use std::io::Read;
use std::iter::once;
use std::mem::{size_of, ManuallyDrop};
use winit::{dpi::LogicalSize, Event, EventsLoop, WindowBuilder};

pub mod debtri;
pub mod utils;

use debtri::*;
use utils::*;

// ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShowWindow {
    /// Runs vulkan in headless mode (hidden window) with a swapchain of 1000x1000
    Headless1k,
    Headless2x1k,
    Headless1x2k,
    Enable,
}

pub fn init_window_with_vulkan(log: &mut Logger<Log>, show: ShowWindow) -> Windowing {
    info![log, "vxdraw", "Initializing rendering"; "headless" => InDebug(&show)];
    let events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_visibility(show == ShowWindow::Enable)
        .build(&events_loop)
        .unwrap();

    match show {
        ShowWindow::Headless1k => {
            let dpi_factor = window.get_hidpi_factor();
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
        }
        ShowWindow::Headless2x1k => {
            let dpi_factor = window.get_hidpi_factor();
            window.set_inner_size(LogicalSize {
                width: 2000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
        }
        ShowWindow::Headless1x2k => {
            let dpi_factor = window.get_hidpi_factor();
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 2000f64 / dpi_factor,
            });
        }
        ShowWindow::Enable => {}
    }

    let version = 1;
    let vk_inst = back::Instance::create("renderer", version);
    let mut surf: <back::Backend as Backend>::Surface = vk_inst.create_surface(&window);
    let mut adapters = vk_inst.enumerate_adapters();
    let len = adapters.len();
    info![log, "vxdraw", "Adapters found"; "count" => len];
    for (idx, adap) in adapters.iter().enumerate() {
        let info = adap.info.clone();
        let limits = adap.physical_device.limits();
        info![log, "vxdraw", "Adapter found"; "idx" => idx, "info" => InDebugPretty(&info), "device limits" => InDebugPretty(&limits)];
    }
    // TODO Find appropriate adapter, I've never seen a case where we have 2+ adapters, that time
    // will come one day
    let adapter = adapters.remove(0);
    let (device, queue_group) = adapter
        .open_with::<_, gfx_hal::Graphics>(1, |family| surf.supports_queue_family(family))
        .expect("Unable to find device supporting graphics");

    let (caps, formats, present_modes, _composite_alpha) =
        surf.compatibility(&adapter.physical_device);

    if !caps.usage.contains(image::Usage::TRANSFER_SRC) {
        warn![
            log,
            "vxdraw", "Surface does not support TRANSFER_SRC, may fail during testing"
        ];
    }

    info![log, "vxdraw", "Surface capabilities"; "capabilities" => InDebugPretty(&caps); clone caps];
    info![log, "vxdraw", "Formats available"; "formats" => InDebugPretty(&formats); clone formats];
    let format = formats.map_or(format::Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .cloned()
            .unwrap_or(formats[0])
    });

    info![log, "vxdraw", "Format chosen"; "format" => InDebugPretty(&format); clone format];
    info![log, "vxdraw", "Available present modes"; "modes" => InDebugPretty(&present_modes); clone present_modes];

    // https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkPresentModeKHR.html
    // VK_PRESENT_MODE_FIFO_KHR ... This is the only value of presentMode that is required to be supported
    let present_mode = {
        [Mailbox, Fifo, Relaxed, Immediate]
            .iter()
            .cloned()
            .find(|pm| present_modes.contains(pm))
            .ok_or("No PresentMode values specified!")
            .unwrap()
    };
    info![log, "vxdraw", "Using best possible present mode"; "mode" => InDebug(&present_mode)];

    let image_count = if present_mode == Mailbox {
        (caps.image_count.end - 1).min(3)
    } else {
        (caps.image_count.end - 1).min(2)
    };
    info![log, "vxdraw", "Using swapchain images"; "count" => image_count];

    let dims = {
        let dpi_factor = window.get_hidpi_factor();
        info![log, "vxdraw", "Window DPI factor"; "factor" => dpi_factor];
        let (w, h): (u32, u32) = window
            .get_inner_size()
            .unwrap()
            .to_physical(dpi_factor)
            .into();
        Extent2D {
            width: w,
            height: h,
        }
    };
    info![log, "vxdraw", "Swapchain size"; "extent" => InDebug(&dims)];

    let mut swap_config = SwapchainConfig::from_caps(&caps, format, dims);
    swap_config.present_mode = present_mode;
    swap_config.image_count = image_count;
    swap_config.extent = dims;
    swap_config.image_usage |= gfx_hal::image::Usage::TRANSFER_SRC;
    info![log, "vxdraw", "Swapchain final configuration"; "swapchain" => InDebugPretty(&swap_config); clone swap_config];

    let (swapchain, backbuffer) =
        unsafe { device.create_swapchain(&mut surf, swap_config.clone(), None) }
            .expect("Unable to create swapchain");

    let backbuffer_string = format!["{:#?}", backbuffer];
    info![log, "vxdraw", "Backbuffer information"; "backbuffers" => backbuffer_string];

    let image_views: Vec<_> = match backbuffer {
        Backbuffer::Images(ref images) => images
            .iter()
            .map(|image| unsafe {
                device
                    .create_image_view(
                        &image,
                        image::ViewKind::D2,
                        format, // MUST be identical to the image's format
                        Swizzle::NO,
                        image::SubresourceRange {
                            aspects: format::Aspects::COLOR,
                            levels: 0..1,
                            layers: 0..1,
                        },
                    )
                    .map_err(|_| "Couldn't create the image_view for the image!")
            })
            .collect::<Result<Vec<_>, &str>>()
            .unwrap(),
        Backbuffer::Framebuffer(_) => unimplemented!("Can't handle framebuffer backbuffer!"),
    };

    {
        let image_views = format!["{:?}", image_views];
        info![log, "vxdraw", "Created image views"; "image views" => image_views];
    }

    // NOTE: for curious people, the render_pass, used in both framebuffer creation AND command
    // buffer when drawing, only need to be _compatible_, which means the SAMPLE count and the
    // FORMAT is _the exact same_.
    // Other elements such as attachment load/store methods are irrelevant.
    // https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#renderpass-compatibility
    let render_pass = {
        let color_attachment = pass::Attachment {
            format: Some(format),
            samples: 1,
            ops: pass::AttachmentOps {
                load: pass::AttachmentLoadOp::Clear,
                store: pass::AttachmentStoreOp::Store,
            },
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
        info![log, "vxdraw", "Render pass info"; "color attachment" => InDebugPretty(&color_attachment); clone color_attachment];
        unsafe {
            device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .map_err(|_| "Couldn't create a render pass!")
                .unwrap()
        }
    };
    {
        let rpfmt = format!["{:#?}", render_pass];
        info![log, "vxdraw", "Created render pass for framebuffers"; "renderpass" => rpfmt];
    }

    let framebuffers: Vec<<back::Backend as Backend>::Framebuffer> = {
        image_views
            .iter()
            .map(|image_view| unsafe {
                device
                    .create_framebuffer(
                        &render_pass,
                        vec![image_view],
                        image::Extent {
                            width: dims.width as u32,
                            height: dims.height as u32,
                            depth: 1,
                        },
                    )
                    .map_err(|_| "Failed to create a framebuffer!")
            })
            .collect::<Result<Vec<_>, &str>>()
            .unwrap()
    };

    let framebuffers_string = format!["{:#?}", framebuffers];
    info![log, "vxdraw", "Framebuffer information"; "framebuffers" => framebuffers_string];

    let mut frame_render_fences = vec![];
    let mut acquire_image_semaphores = vec![];
    let mut present_wait_semaphores = vec![];
    for _ in 0..image_count {
        frame_render_fences.push(device.create_fence(true).expect("Can't create fence"));
        acquire_image_semaphores.push(device.create_semaphore().expect("Can't create semaphore"));
        present_wait_semaphores.push(device.create_semaphore().expect("Can't create semaphore"));
    }

    {
        let count = frame_render_fences.len();
        info![log, "vxdraw", "Allocated fences and semaphores"; "count" => count];
    }

    let mut command_pool = unsafe {
        device
            .create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::RESET_INDIVIDUAL)
            .unwrap()
    };
    let command_buffers: Vec<_> = framebuffers
        .iter()
        .map(|_| command_pool.acquire_command_buffer::<command::MultiShot>())
        .collect();

    // triangle

    pub const VERTEX_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout (location = 0) in vec2 position;
    out gl_PerVertex {
        vec4 gl_Position;
    };
    layout(push_constant) uniform ColorBlock {
        mat4 view;
    } PushConstant;
    void main()
    {
      gl_Position = PushConstant.view * vec4(position, 0.0, 1.0);
    }";

    pub const FRAGMENT_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout(location = 0) out vec4 color;
    void main()
    {
        color = vec4(1.0);
    }";

    let vs_module = {
        let glsl = VERTEX_SOURCE;
        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Vertex)
            .unwrap()
            .bytes()
            .map(Result::unwrap)
            .collect();
        unsafe { device.create_shader_module(&spirv) }.unwrap()
    };
    let fs_module = {
        let glsl = FRAGMENT_SOURCE;
        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Fragment)
            .unwrap()
            .bytes()
            .map(Result::unwrap)
            .collect();
        unsafe { device.create_shader_module(&spirv) }.unwrap()
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

        let subpass = pass::SubpassDesc {
            colors: &[(0, image::Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        unsafe { device.create_render_pass(&[attachment], &[subpass], &[]) }
            .expect("Can't create render pass")
    };

    // ---

    let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);

    let vertex_buffers: Vec<VertexBufferDesc> = vec![VertexBufferDesc {
        binding: 0,
        stride: (size_of::<f32>() * 2) as u32,
        rate: 0,
    }];
    let attributes: Vec<AttributeDesc> = vec![AttributeDesc {
        location: 0,
        binding: 0,
        element: Element {
            format: format::Format::Rg32Float,
            offset: 0,
        },
    }];

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
        width: dims.width as u32,
        height: dims.height as u32,
        depth: 1,
    }
    .rect();
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
            device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let mut push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((ShaderStageFlags::VERTEX, 0..16));
    let triangle_pipeline_layout = unsafe {
        device
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

    let tr_pipe_fmt = format!["{:#?}", pipeline_desc];
    info![log, "vxdraw", "Pipeline descriptor"; "pipeline" => tr_pipe_fmt];

    let triangle_pipeline = unsafe {
        device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Couldn't create a graphics pipeline!")
    };

    unsafe {
        device.destroy_shader_module(vs_module);
    }
    unsafe {
        device.destroy_shader_module(fs_module);
    }

    let mut windowing = Windowing {
        acquire_image_semaphores,
        adapter,
        backbuffer,
        command_buffers,
        command_pool: ManuallyDrop::new(command_pool),
        current_frame: 0,
        debug_triangles: None,
        device: ManuallyDrop::new(device),
        events_loop,
        frame_render_fences,
        framebuffers,
        format,
        image_count: image_count as usize,
        image_views,
        present_wait_semaphores,
        queue_group: ManuallyDrop::new(queue_group),
        render_area: Rect {
            x: 0,
            y: 0,
            w: dims.width as i16,
            h: dims.height as i16,
        },
        render_pass: ManuallyDrop::new(render_pass),
        surf,
        swapchain: ManuallyDrop::new(swapchain),
        swapchain_prev_idx: 0,
        swapconfig: swap_config,
        simple_textures: vec![],
        triangle_buffers: vec![],
        triangle_descriptor_set_layouts,
        triangle_memory: vec![],
        triangle_pipeline: ManuallyDrop::new(triangle_pipeline),
        triangle_pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        triangle_render_pass: ManuallyDrop::new(triangle_render_pass),
        vk_inst: ManuallyDrop::new(vk_inst),
        window,
    };
    create_debug_triangle(&mut windowing, log);
    windowing
}

pub struct Sprite {
    width: f32,
    height: f32,
    uv_begin: (f32, f32),
    uv_end: (f32, f32),
    translation: (f32, f32),
    rotation: f32,
    scale: f32,
}

pub fn add_sprite(s: &mut Windowing, sprite: Sprite, texture: usize, log: &mut Logger<Log>) -> u32 {
    let tex = &mut s.simple_textures[texture];
    let device = &s.device;

    // Derive xy from the sprite's initial UV
    let a = sprite.uv_begin;
    let b = sprite.uv_end;

    let width = sprite.width;
    let height = sprite.height;

    let topleft = (-width / 2f32, -height / 2f32);
    let topleft_uv = a;

    let topright = (width / 2f32, -height / 2f32);
    let topright_uv = (b.0, a.1);

    let bottomleft = (-width / 2f32, height / 2f32);
    let bottomleft_uv = (a.0, b.1);

    let bottomright = (width / 2f32, height / 2f32);
    let bottomright_uv = (b.0, b.1);

    unsafe {
        let mut data_target = device
            .acquire_mapping_writer(
                &tex.texture_vertex_memory,
                0..tex.texture_vertex_requirements.size,
            )
            .expect("Failed to acquire a memory writer!");
        let idx = (tex.count * 6 * 8) as usize;

        for (i, (idx, point, uv)) in [
            (idx, topleft, topleft_uv),
            (idx + 8, bottomleft, bottomleft_uv),
            (idx + 16, bottomright, bottomright_uv),
            (idx + 24, bottomright, bottomright_uv),
            (idx + 32, topright, topright_uv),
            (idx + 40, topleft, topleft_uv),
        ]
        .iter()
        .enumerate()
        {
            data_target[*idx..*idx + 2].copy_from_slice(&[point.0, point.1]);
            data_target[*idx + 2..*idx + 4].copy_from_slice(&[uv.0, uv.1]);
            data_target[*idx + 4..*idx + 6]
                .copy_from_slice(&[sprite.translation.0, sprite.translation.1]);
            data_target[*idx + 6..*idx + 7].copy_from_slice(&[sprite.rotation]);
            data_target[*idx + 7..*idx + 8].copy_from_slice(&[sprite.scale]);
        }
        tex.count += 1;
        device
            .release_mapping_writer(data_target)
            .expect("Couldn't release the mapping writer!");
    }
    tex.count - 1
}

pub fn add_texture(s: &mut Windowing, log: &mut Logger<Log>) -> usize {
    #[rustfmt::skip]
    let (texture_vertex_buffer, texture_vertex_memory, texture_vertex_requirements) = make_vertex_buffer_with_data(
        s,
        &[0f32; 8*6*1000]);
    // &[
    // -1.0f32, -1.0, // Original position
    // 0.0, 0.0,      // UV
    // 0.0, 0.0,      // DX DY
    // 0.0,           // Rot
    // 0.0,           // Scale

    // -1.0f32, 1.0,
    // 0.0, 1.0,
    // 0.0, 0.0,
    // 0.0,

    // 1.0f32, 1.0,
    // 1.0f32, 1.0,
    // 0.0, 0.0,
    // 0.0,

    // 1.0f32, 1.0,
    // 1.0f32, 1.0,
    // 0.0, 0.0,
    // 0.0,

    // 1.0f32, -1.0,
    // 1.0f32, 0.0,
    // 0.0, 0.0,
    // 0.0,

    // -1.0f32, -1.0,
    // 0.0f32, 0.0,
    // 0.0, 0.0,
    // 0.0,
    // ],
    // );

    let device = &s.device;

    let img_data = include_bytes!["../../assets/images/logo.png"];
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
            PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
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
            PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
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

    layout(location = 0) in vec2 v_pos;
    layout(location = 1) in vec2 v_uv;
    layout(location = 2) in vec2 v_dxdy;
    layout(location = 3) in float rotation;
    layout(location = 4) in float scale;

    layout(location = 0) out vec2 f_uv;

    out gl_PerVertex {
        vec4 gl_Position;
    };

    void main() {
        mat2 rotmatrix = mat2(cos(rotation), -sin(rotation), sin(rotation), cos(rotation));
        vec2 pos = rotmatrix * scale * v_pos;
        f_uv = v_uv;
        gl_Position = vec4(pos + v_dxdy, 0.0, 1.0);
    }";

    const FRAGMENT_SOURCE_TEXTURE: &str = "#version 450
    #extension GL_ARB_separate_shader_objects : enable

    layout(location = 0) in vec2 f_uv;
    layout(location = 0) out vec4 color;

    layout(set = 0, binding = 0) uniform texture2D f_texture;
    layout(set = 0, binding = 1) uniform sampler f_sampler;

    void main() {
        color = texture(sampler2D(f_texture, f_sampler), f_uv);
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
    info![log, "vxdraw", "After making"];
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
        stride: (size_of::<f32>() * (2 + 2 + 2 + 2)) as u32,
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
                format: format::Format::Rg32Float,
                offset: 8,
            },
        },
        AttributeDesc {
            location: 2,
            binding: 0,
            element: Element {
                format: format::Format::Rg32Float,
                offset: 16,
            },
        },
        AttributeDesc {
            location: 3,
            binding: 0,
            element: Element {
                format: format::Format::R32Float,
                offset: 24,
            },
        },
        AttributeDesc {
            location: 4,
            binding: 0,
            element: Element {
                format: format::Format::R32Float,
                offset: 28,
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
                src: Factor::SrcAlpha,
                dst: Factor::OneMinusSrcAlpha,
            },
            alpha: BlendOp::Add {
                src: Factor::One,
                dst: Factor::OneMinusSrcAlpha,
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
    let mut bindings = Vec::<DescriptorSetLayoutBinding>::new();
    bindings.push(DescriptorSetLayoutBinding {
        binding: 0,
        ty: pso::DescriptorType::SampledImage,
        count: 1,
        stage_flags: ShaderStageFlags::FRAGMENT,
        immutable_samplers: false,
    });
    bindings.push(DescriptorSetLayoutBinding {
        binding: 1,
        ty: pso::DescriptorType::Sampler,
        count: 1,
        stage_flags: ShaderStageFlags::FRAGMENT,
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

    let push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
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

    let (
        texture_vertex_buffer_indices,
        texture_vertex_memory_indices,
        texture_vertex_requirements_indices,
    ) = make_index_buffer_with_data(s, &[0f32; 4 * 1000]);

    s.simple_textures.push(SingleTexture {
        count: 0,

        texture_vertex_buffer: ManuallyDrop::new(texture_vertex_buffer),
        texture_vertex_memory: ManuallyDrop::new(texture_vertex_memory),
        texture_vertex_requirements,

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
    s.simple_textures.len() - 1
}

pub fn add_triangle(s: &mut Windowing, triangle: &[f32; 6]) {
    let (buffer, memory, _) = make_vertex_buffer_with_data_on_gpu(s, &triangle[..]);
    s.triangle_buffers.push(buffer);
    s.triangle_memory.push(memory);
}

pub fn collect_input(windowing: &mut Windowing) -> Vec<Event> {
    let mut inputs = vec![];
    windowing.events_loop.poll_events(|evt| {
        inputs.push(evt);
    });
    inputs
}

pub fn draw_frame_copy_framebuffer(
    s: &mut Windowing,
    log: &mut Logger<Log>,
    view: &Matrix4<f32>,
) -> Vec<u8> {
    draw_frame_internal(s, log, view, copy_image_to_rgb)
}

pub fn draw_frame(s: &mut Windowing, log: &mut Logger<Log>, view: &Matrix4<f32>) {
    draw_frame_internal(s, log, view, |_| {});
}

fn draw_frame_internal<T>(
    s: &mut Windowing,
    log: &mut Logger<Log>,
    view: &Matrix4<f32>,
    postproc: fn(&mut Windowing) -> T,
) -> T {
    let frame_render_fence = &s.frame_render_fences[s.current_frame];
    let acquire_image_semaphore = &s.acquire_image_semaphores[s.current_frame];
    let frame = s.current_frame;
    trace![log, "vxdraw", "Current frame"; "frame" => frame];

    let image_index;
    let postproc_res = unsafe {
        image_index = s
            .swapchain
            .acquire_image(
                u64::max_value(),
                FrameSync::Semaphore(acquire_image_semaphore),
            )
            .unwrap();
        trace![log, "vxdraw", "Acquired image index"; "index" => image_index];
        assert_eq![image_index as usize, s.current_frame];
        s.swapchain_prev_idx = image_index;
        trace![log, "vxdraw", "Waiting for fence"];
        s.device
            .wait_for_fence(frame_render_fence, u64::max_value())
            .unwrap();
        trace![log, "vxdraw", "Resetting fence"];
        s.device.reset_fence(frame_render_fence).unwrap();

        {
            let buffer = &mut s.command_buffers[s.current_frame];
            let clear_values = [ClearValue::Color(ClearColor::Float([
                1.0f32, 0.25, 0.5, 0.75,
            ]))];
            buffer.begin(false);
            {
                let mut enc = buffer.begin_render_pass_inline(
                    &s.render_pass,
                    &s.framebuffers[s.current_frame],
                    s.render_area,
                    clear_values.iter(),
                );
                let ptr = view.as_ptr();

                enc.bind_graphics_pipeline(&s.triangle_pipeline);
                enc.push_graphics_constants(
                    &s.triangle_pipeline_layout,
                    ShaderStageFlags::VERTEX,
                    0,
                    &*(ptr as *const [u32; 16]),
                );
                for buffer_ref in &s.triangle_buffers {
                    let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.draw(0..3, 0..1);
                }
                for simple_tex in s.simple_textures.iter() {
                    enc.bind_graphics_pipeline(&simple_tex.pipeline);
                    enc.bind_graphics_descriptor_sets(
                        &simple_tex.pipeline_layout,
                        0,
                        Some(&*simple_tex.descriptor_set),
                        &[],
                    );
                    let buffers: ArrayVec<[_; 1]> =
                        [(&*simple_tex.texture_vertex_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.draw(0..simple_tex.count * 6, 0..1);
                }
                if let Some(ref debug_triangles) = s.debug_triangles {
                    enc.bind_graphics_pipeline(&debug_triangles.pipeline);
                    let ratio =
                        s.swapconfig.extent.width as f32 / s.swapconfig.extent.height as f32;
                    enc.push_graphics_constants(
                        &debug_triangles.pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        &(std::mem::transmute::<f32, [u32; 1]>(ratio)),
                    );
                    let count = debug_triangles.triangles_count;
                    let buffers: ArrayVec<[_; 1]> = [(&debug_triangles.triangles_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    trace![log, "vxdraw", "mesh count"; "count" => count];
                    enc.draw(0..(count * 3) as u32, 0..1);
                }
            }
            buffer.finish();
        }

        let command_buffers = &s.command_buffers[s.current_frame];
        let wait_semaphores: ArrayVec<[_; 1]> = [(
            acquire_image_semaphore,
            PipelineStage::COLOR_ATTACHMENT_OUTPUT,
        )]
        .into();
        {
            let present_wait_semaphore = &s.present_wait_semaphores[frame];
            let signal_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
            // yes, you have to write it twice like this. yes, it's silly.
            let submission = Submission {
                command_buffers: once(command_buffers),
                wait_semaphores,
                signal_semaphores,
            };
            s.queue_group.queues[0].submit(submission, Some(frame_render_fence));
        }
        let postproc_res = postproc(s);
        let present_wait_semaphore = &s.present_wait_semaphores[frame];
        let present_wait_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
        s.swapchain
            .present(
                &mut s.queue_group.queues[0],
                image_index,
                present_wait_semaphores,
            )
            .unwrap();
        postproc_res
    };
    s.current_frame = (s.current_frame + 1) % s.image_count;
    postproc_res
}

pub fn generate_map(s: &mut Windowing, w: u32, h: u32, log: &mut Logger<Log>) -> Vec<u8> {
    static VERTEX_SOURCE: &str = include_str!("../../shaders/proc1.vert");
    static FRAGMENT_SOURCE: &str = include_str!("../../shaders/proc1.frag");
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
        stride: 8u32,
        rate: 0,
    }];
    let attributes: Vec<AttributeDesc> = vec![AttributeDesc {
        location: 0,
        binding: 0,
        element: Element {
            format: format::Format::Rg32Float,
            offset: 0,
        },
    }];

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
        // width: s.swapconfig.extent.width,
        // height: s.swapconfig.extent.height,
        width: w,
        height: h,
        depth: 1,
    }
    .rect();
    let mapgen_render_pass = {
        let attachment = pass::Attachment {
            format: Some(format::Format::Rgba8Srgb),
            samples: 1,
            ops: pass::AttachmentOps::new(
                pass::AttachmentLoadOp::Clear,
                pass::AttachmentStoreOp::Store,
            ),
            stencil_ops: pass::AttachmentOps::DONT_CARE,
            layouts: image::Layout::General..image::Layout::General,
        };

        let subpass = pass::SubpassDesc {
            colors: &[(0, image::Layout::General)],
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
    let mut mapgen_descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            s.device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let mut push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    push_constants.push((ShaderStageFlags::FRAGMENT, 0..4));

    let mapgen_pipeline_layout = unsafe {
        s.device
            .create_pipeline_layout(&mapgen_descriptor_set_layouts, push_constants)
            .expect("Couldn't create a pipeline layout")
    };

    // Describe the pipeline (rasterization, mapgen interpretation)
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
        layout: &mapgen_pipeline_layout,
        subpass: pass::Subpass {
            index: 0,
            main_pass: &mapgen_render_pass,
        },
        flags: PipelineCreationFlags::empty(),
        parent: BasePipeline::None,
    };

    let mapgen_pipeline = unsafe {
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

    // ---

    unsafe {
        let mut image = s
            .device
            .create_image(
                image::Kind::D2(w, h, 1, 1),
                1,
                format::Format::Rgba8Srgb,
                image::Tiling::Linear,
                image::Usage::COLOR_ATTACHMENT | image::Usage::TRANSFER_DST | image::Usage::SAMPLED,
                image::ViewCapabilities::empty(),
            )
            .expect("Unable to create image");
        let requirements = s.device.get_image_requirements(&image);
        let memory_type_id =
            find_memory_type_id(&s.adapter, requirements, memory::Properties::CPU_VISIBLE);
        let memory = s
            .device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Unable to allocate memory");
        let image_view = {
            s.device
                .bind_image_memory(&memory, 0, &mut image)
                .expect("Unable to bind memory");

            s.device
                .create_image_view(
                    &image,
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

        let framebuffer = s
            .device
            .create_framebuffer(
                &mapgen_render_pass,
                vec![&image_view],
                image::Extent {
                    width: w,
                    height: h,
                    depth: 1,
                },
            )
            .expect("fbo");

        #[rustfmt::skip]
        let (pt_buffer, pt_memory, _) = make_vertex_buffer_with_data(
            s,
            &[
                -1.0, -1.0,
                1.0, -1.0,
                1.0, 1.0,
                1.0, 1.0,
                -1.0, 1.0,
                -1.0, -1.0,
            ],
        );

        let mut cmd_buffer = s.command_pool.acquire_command_buffer::<command::OneShot>();
        let clear_values = [ClearValue::Color(ClearColor::Float([
            1.0f32, 0.25, 0.5, 0.75,
        ]))];
        cmd_buffer.begin();
        {
            let image_barrier = memory::Barrier::Image {
                states: (image::Access::empty(), image::Layout::Undefined)
                    ..(image::Access::SHADER_WRITE, image::Layout::General),
                target: &image,
                families: None,
                range: image::SubresourceRange {
                    aspects: format::Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            };
            cmd_buffer.pipeline_barrier(
                PipelineStage::TOP_OF_PIPE..PipelineStage::FRAGMENT_SHADER,
                memory::Dependencies::empty(),
                &[image_barrier],
            );
            let mut enc = cmd_buffer.begin_render_pass_inline(
                &mapgen_render_pass,
                &framebuffer,
                extent,
                clear_values.iter(),
            );
            enc.bind_graphics_pipeline(&mapgen_pipeline);
            enc.push_graphics_constants(
                &mapgen_pipeline_layout,
                ShaderStageFlags::FRAGMENT,
                0,
                &(std::mem::transmute::<[f32; 4], [u32; 4]>([w as f32, 0.3, 93.0, 3.0])),
            );
            let buffers: ArrayVec<[_; 1]> = [(pt_buffer, 0)].into();
            enc.bind_vertex_buffers(0, buffers);
            enc.draw(0..6, 0..1);
        }
        cmd_buffer.finish();
        let upload_fence = s
            .device
            .create_fence(false)
            .expect("Couldn't create an upload fence!");
        s.queue_group.queues[0].submit_nosemaphores(Some(&cmd_buffer), Some(&upload_fence));
        s.device
            .wait_for_fence(&upload_fence, u64::max_value())
            .expect("Unable to wait for fence");
        s.device.destroy_fence(upload_fence);
        s.command_pool.free(once(cmd_buffer));

        let footprint = s.device.get_image_subresource_footprint(
            &image,
            image::Subresource {
                aspects: format::Aspects::COLOR,
                level: 0,
                layer: 0,
            },
        );

        let map = s
            .device
            .acquire_mapping_reader(&memory, footprint.slice)
            .expect("Mapped memory");

        let pixel_size = size_of::<load_image::Rgba<u8>>() as u32;
        let row_size = pixel_size * w;

        let mut result: Vec<u8> = Vec::new();
        for y in 0..h as usize {
            let dest_base = y * footprint.row_pitch as usize;
            result.extend(map[dest_base..dest_base + row_size as usize].iter());
        }
        s.device.release_mapping_reader(map);

        s.device.destroy_buffer(pt_buffer);
        s.device.free_memory(pt_memory);
        s.device.destroy_pipeline_layout(mapgen_pipeline_layout);
        s.device.destroy_graphics_pipeline(mapgen_pipeline);
        for desc_set_layout in mapgen_descriptor_set_layouts.drain(..) {
            s.device.destroy_descriptor_set_layout(desc_set_layout);
        }
        s.device.destroy_render_pass(mapgen_render_pass);
        s.device.destroy_framebuffer(framebuffer);
        s.device.destroy_image_view(image_view);
        s.device.destroy_image(image);
        s.device.free_memory(memory);
        result
    }
}

// ---

#[cfg(feature = "gfx_tests")]
#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    use cgmath::{Deg, Vector3};
    use rand_pcg::Pcg64Mcg as random;

    // ---

    fn add_windmills(windowing: &mut Windowing, rand_rotat: bool) -> Vec<DebugTriangleHandle> {
        use rand::Rng;
        use std::f32::consts::PI;
        let mut rng = random::new(0);
        let mut debug_triangles = Vec::with_capacity(1000);
        for _ in 0..1000 {
            let mut tri = DebugTriangle::default();
            let (dx, dy) = (
                rng.gen_range(-1.0f32, 1.0f32),
                rng.gen_range(-1.0f32, 1.0f32),
            );
            let scale = rng.gen_range(0.03f32, 0.1f32);
            if rand_rotat {
                tri.rotation = rng.gen_range(-PI, PI);
            }
            tri.scale = scale;
            tri.translation = (dx, dy);
            debug_triangles.push(add_to_triangles(windowing, tri));
        }
        debug_triangles
    }

    fn remove_windmills(windowing: &mut Windowing) {
        pop_n_triangles(windowing, 1000);
    }

    fn add_4_screencorners(windowing: &mut Windowing) {
        add_to_triangles(
            windowing,
            DebugTriangle::from([-1.0f32, -1.0, 0.0, -1.0, -1.0, 0.0]),
        );
        add_to_triangles(
            windowing,
            DebugTriangle::from([-1.0f32, 1.0, 0.0, 1.0, -1.0, 0.0]),
        );
        add_to_triangles(
            windowing,
            DebugTriangle::from([1.0f32, -1.0, 0.0, -1.0, 1.0, 0.0]),
        );
        add_to_triangles(
            windowing,
            DebugTriangle::from([1.0f32, 1.0, 0.0, 1.0, 1.0, 0.0]),
        );
    }

    fn assert_swapchain_eq(windowing: &mut Windowing, name: &str, rgb: Vec<u8>) {
        use load_image::ImageDecoder;
        std::fs::create_dir_all("_build/vxdraw_results").expect("Unable to create directories");

        let genname = String::from("_build/vxdraw_results/") + name + ".png";
        let correctname = String::from("tests/vxdraw/") + name + ".png";
        let diffname = String::from("_build/vxdraw_results/") + name + "#diff.png";
        let appendname = String::from("_build/vxdraw_results/") + name + "#sum.png";

        let file = std::fs::File::create(&genname).expect("Unable to create file");
        let enc = load_image::png::PNGEncoder::new(file);
        enc.encode(
            &rgb,
            windowing.swapconfig.extent.width,
            windowing.swapconfig.extent.height,
            load_image::ColorType::RGB(8),
        )
        .expect("Unable to encode PNG file");

        let correct = std::fs::File::open(&correctname).unwrap();
        let dec = load_image::png::PNGDecoder::new(correct)
            .expect("Unable to read PNG file (does it exist?)");

        assert_eq![
            (
                windowing.swapconfig.extent.width as u64,
                windowing.swapconfig.extent.height as u64
            ),
            dec.dimensions(),
            "The swapchain image and the preset correct image MUST be of the exact same size"
        ];
        assert_eq![
            load_image::ColorType::RGB(8),
            dec.colortype(),
            "Both images MUST have the RGB(8) format"
        ];

        let correct_bytes = dec
            .into_reader()
            .expect("Unable to read file")
            .bytes()
            .map(|x| x.expect("Unable to read byte"))
            .collect::<Vec<u8>>();

        fn absdiff(lhs: u8, rhs: u8) -> u8 {
            if let Some(newbyte) = lhs.checked_sub(rhs) {
                newbyte
            } else {
                rhs - lhs
            }
        }

        if correct_bytes != rgb {
            let mut diff = Vec::with_capacity(correct_bytes.len());
            for (idx, byte) in correct_bytes.iter().enumerate() {
                diff.push(absdiff(*byte, rgb[idx]));
            }
            let file = std::fs::File::create(&diffname).expect("Unable to create file");
            let enc = load_image::png::PNGEncoder::new(file);
            enc.encode(
                &diff,
                windowing.swapconfig.extent.width,
                windowing.swapconfig.extent.height,
                load_image::ColorType::RGB(8),
            )
            .expect("Unable to encode PNG file");
            std::process::Command::new("convert")
                .args(&[
                    "-bordercolor".into(),
                    "black".into(),
                    "-border".into(),
                    "20".into(),
                    correctname,
                    genname,
                    diffname,
                    "+append".into(),
                    appendname.clone(),
                ])
                .output()
                .expect("Failed to execute process");
            std::process::Command::new("feh")
                .args(&[appendname])
                .output()
                .expect("Failed to execute process");
            assert![false, "Images were NOT the same!"];
        } else {
            assert![true];
        }
    }

    // ---

    #[test]
    fn setup_and_teardown() {
        let mut logger = Logger::spawn_void();
        let _ = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
    }

    #[test]
    fn setup_and_teardown_draw_clear() {
        let mut logger = Logger::spawn();
        logger.set_colorize(true);
        logger.set_log_level(64);

        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "setup_and_teardown_draw_with_test", img);
    }

    #[test]
    fn setup_and_teardown_with_gpu_upload() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let (buffer, memory, _) =
            make_vertex_buffer_with_data_on_gpu(&mut windowing, &vec![1.0f32; 10_000]);

        unsafe {
            windowing.device.destroy_buffer(buffer);
            windowing.device.free_memory(memory);
        }
    }

    #[test]
    fn init_window_and_get_input() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        collect_input(&mut windowing);
    }

    #[test]
    fn simple_triangle() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);
        let tri = DebugTriangle::default();

        add_to_triangles(&mut windowing, tri);
        add_4_screencorners(&mut windowing);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle", img);
    }

    #[test]
    fn simple_triangle_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);
        let tri = DebugTriangle::default();

        let idx = add_to_triangles(&mut windowing, tri);
        set_triangle_color(&mut windowing, &idx, [255, 0, 255, 255]);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle_change_color", img);
    }

    #[test]
    fn debug_triangle_corners_widescreen() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&mut windowing);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = add_to_triangles(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "debug_triangle_corners_widescreen", img);
    }

    #[test]
    fn debug_triangle_corners_tallscreen() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1x2k);
        let prspect = gen_perspective(&mut windowing);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = add_to_triangles(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "debug_triangle_corners_tallscreen", img);
    }

    #[test]
    fn circle_of_triangles() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&mut windowing);

        for i in 0..360 {
            let mut tri = DebugTriangle::default();
            tri.translation = ((i as f32).cos(), (i as f32).sin());
            tri.scale = 0.1f32;
            let _idx = add_to_triangles(&mut windowing, tri);
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "circle_of_triangles", img);
    }

    #[test]
    fn triangle_in_corner() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let mut tri = DebugTriangle::default();
        tri.scale = 0.1f32;
        let radi = tri.radius();

        let trans = -1f32 + radi;
        for j in 0..31 {
            for i in 0..31 {
                tri.translation = (trans + i as f32 * 2.0 * radi, trans + j as f32 * 2.0 * radi);
                add_to_triangles(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "triangle_in_corner", img);
    }

    #[test]
    fn a_bunch_of_quads() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let mut topright = DebugTriangle::from([-1.0, -1.0, 1.0, 1.0, 1.0, -1.0]);
        let mut bottomleft = DebugTriangle::from([-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
        topright.scale = 0.1;
        bottomleft.scale = 0.1;
        let radi = 0.1;
        let trans = -1f32 + radi;

        for j in 0..10 {
            for i in 0..10 {
                topright.translation =
                    (trans + i as f32 * 2.0 * radi, trans + j as f32 * 2.0 * radi);
                bottomleft.translation =
                    (trans + i as f32 * 2.0 * radi, trans + j as f32 * 2.0 * radi);
                add_to_triangles(&mut windowing, topright);
                add_to_triangles(&mut windowing, bottomleft);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "a_bunch_of_quads", img);
    }

    #[test]
    fn generate_map() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let mut img = super::generate_map(&mut windowing, 1000, 1000, &mut logger);
        let img = img
            .drain(..)
            .enumerate()
            .filter(|(idx, _)| idx % 4 != 0)
            .map(|(_, v)| v)
            .collect::<Vec<u8>>();
        assert_swapchain_eq(&mut windowing, "genmap", img);
    }

    #[test]
    fn windmills() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, false);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills", img);
    }

    #[test]
    fn windmills_ignore_perspective() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, false);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills_ignore_perspective", img);
    }

    #[test]
    fn windmills_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let handles = add_windmills(&mut windowing, false);
        set_triangle_color(&mut windowing, &handles[0], [255, 0, 0, 255]);
        set_triangle_color(&mut windowing, &handles[249], [0, 255, 0, 255]);
        set_triangle_color(&mut windowing, &handles[499], [0, 0, 255, 255]);
        set_triangle_color(&mut windowing, &handles[999], [0, 0, 0, 255]);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills_change_color", img);
    }

    #[test]
    fn tearing_test() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let tri = make_centered_equilateral_triangle();
        add_triangle(&mut windowing, &tri);
        for i in 0..=360 {
            if i % 2 == 0 {
                add_4_screencorners(&mut windowing);
            } else {
                pop_n_triangles(&mut windowing, 4);
            }
            let rot =
                prspect * Matrix4::from_axis_angle(Vector3::new(0.0f32, 0.0, 1.0), Deg(i as f32));
            draw_frame(&mut windowing, &mut logger, &rot);
            std::thread::sleep(std::time::Duration::new(0, 80_000_000));
        }
    }

    #[test]
    fn simple_texture() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, &mut logger);
        add_sprite(
            &mut windowing,
            Sprite {
                width: 2f32,
                height: 2f32,
                uv_begin: (0.0, 0.0),
                uv_end: (1.0, 1.0),
                translation: (0.0, 0.0),
                rotation: 0.0,
                scale: 1.0,
            },
            tex,
            &mut logger,
        );

        let prspect = gen_perspective(&mut windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "simple_texture", img);
    }

    #[test]
    fn many_sprites() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, &mut logger);
        for i in 0..360 {
            add_sprite(
                &mut windowing,
                Sprite {
                    width: 2f32,
                    height: 2f32,
                    uv_begin: (0.0, 0.0),
                    uv_end: (1.0, 1.0),
                    translation: (0.0, 0.0),
                    rotation: ((i * 10) as f32 / 180f32 * 3.14),
                    scale: 0.5,
                },
                tex,
                &mut logger,
            );
        }

        let prspect = gen_perspective(&mut windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "many_sprites", img);
    }

    #[test]
    fn rotating_windmills_drawing_invariant() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, false);
        for _ in 0..30 {
            rotate_to_triangles(&mut windowing, Deg(-1.0f32));
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
        remove_windmills(&mut windowing);

        add_windmills(&mut windowing, false);
        for _ in 0..30 {
            rotate_to_triangles(&mut windowing, Deg(-1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
    }

    #[test]
    fn windmills_given_initial_rotation() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, true);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "windmills_given_initial_rotation", img);
    }

    // ---

    #[bench]
    fn bench_many_sprites(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, &mut logger);
        for i in 0..500 {
            add_sprite(
                &mut windowing,
                Sprite {
                    width: 2f32,
                    height: 2f32,
                    uv_begin: (0.0, 0.0),
                    uv_end: (0.05, 0.05),
                    translation: (0.0, 0.0),
                    rotation: ((i * 10) as f32 / 180f32 * 3.14),
                    scale: 0.5,
                },
                tex,
                &mut logger,
            );
        }

        let prspect = gen_perspective(&mut windowing);
        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_simple_triangle(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_to_triangles(&mut windowing, DebugTriangle::default());
        add_4_screencorners(&mut windowing);

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_still_windmills(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, false);

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_windmills_set_color(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let handles = add_windmills(&mut windowing, false);

        b.iter(|| {
            set_triangle_color(&mut windowing, &handles[0], black_box([0, 0, 0, 255]));
        });
    }

    #[bench]
    fn bench_rotating_windmills(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing, false);

        b.iter(|| {
            rotate_to_triangles(&mut windowing, Deg(1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_rotating_windmills_no_render(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        add_windmills(&mut windowing, false);

        b.iter(|| {
            rotate_to_triangles(&mut windowing, Deg(1.0f32));
        });
    }

    #[bench]
    fn clears_per_second(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn noninstanced_1k_triangles(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);
        let tri = make_centered_equilateral_triangle();
        for _ in 0..1000 {
            add_triangle(&mut windowing, &tri);
        }

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_generate_map(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        b.iter(|| {
            super::generate_map(&mut windowing, 1000, 1000, &mut logger);
        });
    }
}
