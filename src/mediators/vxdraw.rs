use crate::glocals::{ColoredTriangleList, Log, SingleTexture, Windowing};
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
use cgmath::{Matrix4, Rad, Vector2, Vector3, Vector4};
use gfx_hal::{
    adapter::{MemoryTypeId, PhysicalDevice},
    command::{self, BufferCopy, ClearColor, ClearValue},
    device::Device,
    format::{self, ChannelType, Swizzle},
    image, memory,
    memory::Properties,
    pass, pool,
    pso::{
        self, AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState,
        ColorBlendDesc, ColorMask, DepthStencilDesc, DepthTest, DescriptorSetLayoutBinding,
        Element, Face, Factor, FrontFace, GraphicsPipelineDesc, InputAssemblerDesc, LogicOp,
        PipelineCreationFlags, PipelineStage, PolygonMode, Rasterizer, Rect, ShaderStageFlags,
        StencilTest, VertexBufferDesc, Viewport,
    },
    queue::Submission,
    window::{Extent2D, PresentMode::*, Surface, Swapchain},
    Adapter, Backbuffer, Backend, FrameSync, Instance, Primitive, SwapchainConfig,
};
use logger::{debug, info, log, trace, warn, InDebug, InDebugPretty, Logger};
use std::io::Read;
use std::iter::once;
use std::mem::{size_of, transmute, ManuallyDrop};
use winit::{dpi::LogicalSize, Event, EventsLoop, WindowBuilder};

// pub mod manytris;

// ---

#[derive(PartialEq)]
pub enum ShowWindow {
    /// Runs vulkan in headless mode (hidden window) with a swapchain of 1000x1000
    Headless1k,
    Enable,
}

pub fn init_window_with_vulkan(log: &mut Logger<Log>, show: ShowWindow) -> Windowing {
    let events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_visibility(show != ShowWindow::Headless1k)
        .build(&events_loop)
        .unwrap();

    if show == ShowWindow::Headless1k {
        let dpi_factor = window.get_hidpi_factor();
        window.set_inner_size(LogicalSize {
            width: 1000f64 / dpi_factor,
            height: 1000f64 / dpi_factor,
        });
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
        log![128, log, "vxdraw", "Render pass"; "color attachment" => InDebugPretty(&color_attachment); clone color_attachment];
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
    info![log, "vxdraw", "Framebuffer information"; "framebuffers" => framebuffers_string ];

    let mut frame_render_fences = vec![];
    let mut acquire_image_semaphores = vec![];
    let mut present_wait_semaphores = vec![];
    for _ in 0..image_count {
        frame_render_fences.push(device.create_fence(true).expect("Can't create fence"));
        acquire_image_semaphores.push(device.create_semaphore().expect("Can't create semaphore"));
        present_wait_semaphores.push(device.create_semaphore().expect("Can't create semaphore"));
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
    info![log, "vxdraw", "Pipeline descriptor"; "pipeline" => InDebugPretty(&tr_pipe_fmt)];

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

pub fn create_debug_triangle(s: &mut Windowing, log: &mut Logger<Log>) {
    pub const VERTEX_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout (location = 0) in vec2 position;
    layout (location = 1) in vec4 color;
    layout (location = 0) out vec4 outcolor;
    out gl_PerVertex {
        vec4 gl_Position;
    };
    void main() {
      gl_Position = vec4(position, 0.0, 1.0);
      outcolor = color;
    }";

    pub const FRAGMENT_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout(location = 0) in vec4 incolor;
    layout(location = 0) out vec4 color;
    void main() {
        color = incolor;
    }";

    info![log, "vxdraw", "Before shading module"];
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
    info![log, "vxdraw", "After shading module"];
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
        stride: (size_of::<f32>() * (2 + 1)) as u32,
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
    let push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    let triangle_pipeline_layout = unsafe {
        s.device
            .create_pipeline_layout(&triangle_descriptor_set_layouts, push_constants)
            .expect("Couldn't create a pipeline layout")
    };
    info![log, "vxdraw", "Creating custom pipe"];
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
    info![log, "vxdraw", "Neat shit"];

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
    let (dtbuffer, dtmemory, dtreqs) = make_vertex_buffer_with_data(s, &[0.0f32; 3 * 3 * 1000]);
    info![log, "vxdraw", "Vertex buffer size"; "requirements" => InDebugPretty(&dtreqs)];
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

pub fn find_memory_type_id<B: gfx_hal::Backend>(
    adap: &Adapter<B>,
    reqs: memory::Requirements,
    prop: memory::Properties,
) -> MemoryTypeId {
    adap.physical_device
        .memory_properties()
        .memory_types
        .iter()
        .enumerate()
        .find(|&(id, memory_type)| {
            reqs.type_mask & (1 << id) != 0 && memory_type.properties.contains(prop)
        })
        .map(|(id, _)| MemoryTypeId(id))
        .expect("Unable to find memory type id")
}

pub fn make_vertex_buffer_with_data(
    s: &mut Windowing,
    data: &[f32],
) -> (
    <back::Backend as Backend>::Buffer,
    <back::Backend as Backend>::Memory,
    memory::Requirements,
) {
    let device = &s.device;
    let (buffer, memory, requirements) = unsafe {
        let buffer_size: u64 = (std::mem::size_of::<f32>() * data.len()) as u64;
        let mut buffer = device
            .create_buffer(buffer_size, gfx_hal::buffer::Usage::VERTEX)
            .expect("cant make bf");
        let requirements = device.get_buffer_requirements(&buffer);
        let memory_type_id = find_memory_type_id(&s.adapter, requirements, Properties::CPU_VISIBLE);
        let memory = device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate vertex buffer memory");
        device
            .bind_buffer_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        (buffer, memory, requirements)
    };
    unsafe {
        let mut data_target = device
            .acquire_mapping_writer(&memory, 0..requirements.size)
            .expect("Failed to acquire a memory writer!");
        data_target[..data.len()].copy_from_slice(data);
        device
            .release_mapping_writer(data_target)
            .expect("Couldn't release the mapping writer!");
    }
    (buffer, memory, requirements)
}

pub fn make_transfer_buffer_of_size(
    s: &mut Windowing,
    size: u64,
) -> (
    <back::Backend as Backend>::Buffer,
    <back::Backend as Backend>::Memory,
    memory::Requirements,
) {
    let device = &s.device;
    let (buffer, memory, requirements) = unsafe {
        let mut buffer = device
            .create_buffer(size, gfx_hal::buffer::Usage::TRANSFER_DST)
            .expect("cant make bf");
        let requirements = device.get_buffer_requirements(&buffer);
        let memory_type_id = find_memory_type_id(&s.adapter, requirements, Properties::CPU_VISIBLE);
        let memory = device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate vertex buffer memory");
        device
            .bind_buffer_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        (buffer, memory, requirements)
    };
    (buffer, memory, requirements)
}

pub fn make_transfer_img_of_size(
    s: &mut Windowing,
    w: u32,
    h: u32,
) -> (
    <back::Backend as Backend>::Image,
    <back::Backend as Backend>::Memory,
    memory::Requirements,
) {
    let device = &s.device;
    let (buffer, memory, requirements) = unsafe {
        let mut buffer = device
            .create_image(
                image::Kind::D2(w, h, 1, 1),
                1,
                format::Format::Rgb8Unorm,
                image::Tiling::Linear,
                image::Usage::TRANSFER_SRC | image::Usage::TRANSFER_DST,
                image::ViewCapabilities::empty(),
            )
            .expect("cant make bf");
        let requirements = device.get_image_requirements(&buffer);
        let memory_type_id = find_memory_type_id(&s.adapter, requirements, Properties::CPU_VISIBLE);
        let memory = device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate image buffer memory");
        device
            .bind_image_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        (buffer, memory, requirements)
    };
    (buffer, memory, requirements)
}

pub fn make_vertex_buffer_with_data_on_gpu(
    s: &mut Windowing,
    data: &[f32],
) -> (
    <back::Backend as Backend>::Buffer,
    <back::Backend as Backend>::Memory,
    memory::Requirements,
) {
    let device = &s.device;
    let (buffer, memory, requirements) = unsafe {
        let buffer_size: u64 = (std::mem::size_of::<f32>() * data.len()) as u64;
        let mut buffer = device
            .create_buffer(buffer_size, gfx_hal::buffer::Usage::TRANSFER_SRC)
            .expect("cant make bf");
        let requirements = device.get_buffer_requirements(&buffer);
        let memory_type_id = find_memory_type_id(&s.adapter, requirements, Properties::CPU_VISIBLE);
        let memory = device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate vertex buffer memory");
        device
            .bind_buffer_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        (buffer, memory, requirements)
    };
    unsafe {
        let mut data_target = device
            .acquire_mapping_writer(&memory, 0..requirements.size)
            .expect("Failed to acquire a memory writer!");
        data_target[..data.len()].copy_from_slice(data);
        device
            .release_mapping_writer(data_target)
            .expect("Couldn't release the mapping writer!");
    }

    let (buffer_gpu, memory_gpu, memory_gpu_requirements) = unsafe {
        let buffer_size: u64 = (std::mem::size_of::<f32>() * data.len()) as u64;
        let mut buffer = device
            .create_buffer(
                buffer_size,
                gfx_hal::buffer::Usage::TRANSFER_DST | gfx_hal::buffer::Usage::VERTEX,
            )
            .expect("cant make bf");
        let requirements = device.get_buffer_requirements(&buffer);
        let memory_type_id =
            find_memory_type_id(&s.adapter, requirements, Properties::DEVICE_LOCAL);
        let memory = device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate vertex buffer memory");
        device
            .bind_buffer_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        (buffer, memory, requirements)
    };
    let buffer_size: u64 = (std::mem::size_of::<f32>() * data.len()) as u64;
    let mut cmd_buffer = s
        .command_pool
        .acquire_command_buffer::<gfx_hal::command::OneShot>();
    unsafe {
        cmd_buffer.begin();
        let buffer_barrier = gfx_hal::memory::Barrier::Buffer {
            families: None,
            range: None..None,
            states: gfx_hal::buffer::Access::empty()..gfx_hal::buffer::Access::TRANSFER_WRITE,
            target: &buffer_gpu,
        };
        cmd_buffer.pipeline_barrier(
            PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
            gfx_hal::memory::Dependencies::empty(),
            &[buffer_barrier],
        );
        let copy = once(BufferCopy {
            src: 0,
            dst: 0,
            size: buffer_size,
        });
        cmd_buffer.copy_buffer(&buffer, &buffer_gpu, copy);
        let buffer_barrier = gfx_hal::memory::Barrier::Buffer {
            families: None,
            range: None..None,
            states: gfx_hal::buffer::Access::TRANSFER_WRITE..gfx_hal::buffer::Access::SHADER_READ,
            target: &buffer_gpu,
        };
        cmd_buffer.pipeline_barrier(
            PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
            gfx_hal::memory::Dependencies::empty(),
            &[buffer_barrier],
        );
        cmd_buffer.finish();
        let upload_fence = device
            .create_fence(false)
            .expect("Couldn't create an upload fence!");
        s.queue_group.queues[0].submit_nosemaphores(Some(&cmd_buffer), Some(&upload_fence));
        device
            .wait_for_fence(&upload_fence, core::u64::MAX)
            .expect("Couldn't wait for the fence!");
        device.destroy_fence(upload_fence);
        device.destroy_buffer(buffer);
        device.free_memory(memory);
    }
    (buffer_gpu, memory_gpu, memory_gpu_requirements)
}

pub fn add_texture(s: &mut Windowing, _lgr: &mut Logger<Log>) {
    let (texture_vertex_buffer, texture_vertex_memory, _) = make_vertex_buffer_with_data_on_gpu(
        s,
        &[
            -1.0f32, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ],
    );
    let (texture_uv_buffer, texture_uv_memory, _) = make_vertex_buffer_with_data_on_gpu(
        s,
        &[
            -1.0f32, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ],
    );

    let device = &s.device;

    // const VERTEX_SOURCE_TEXTURE: &str = "#version 450
    // #extension GL_ARB_separate_shader_objects : enable

    // layout(constant_id = 0) const float scale = 1.2f;

    // layout(location = 0) in vec2 a_pos;
    // layout(location = 1) in vec2 a_uv;
    // layout(location = 0) out vec2 v_uv;

    // out gl_PerVertex {
    //     vec4 gl_Position;
    // };

    // void main() {
    //     v_uv = a_uv;
    //     gl_Position = vec4(scale * a_pos, 0.0, 1.0);
    // }";

    // const FRAGMENT_SOURCE_TEXTURE: &str = "#version 450
    // #extension GL_ARB_separate_shader_objects : enable

    // layout(location = 0) in vec2 v_uv;
    // layout(location = 0) out vec4 target0;

    // layout(set = 0, binding = 0) uniform texture2D u_texture;
    // layout(set = 0, binding = 1) uniform sampler u_sampler;

    // void main() {
    //     target0 = texture(sampler2D(u_texture, u_sampler), v_uv);
    // }";

    let img_data = include_bytes!["../../assets/images/logo.png"];
    let img = load_image::load_from_memory_with_format(&img_data[..], load_image::PNG)
        .unwrap()
        .to_rgba();
    let (img_width, img_height) = img.dimensions();
    let _kind = image::Kind::D2(img_width as image::Size, img_height as image::Size, 1, 1);
    let limits = s.adapter.physical_device.limits();
    let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
    let image_stride = 4usize;
    let row_pitch = (img_width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
    let upload_size = u64::from(img_height * row_pitch);
    // debug_assert!(row_pitch as usize >= row_size);

    let mut image_upload_buffer =
        unsafe { device.create_buffer(upload_size, gfx_hal::buffer::Usage::TRANSFER_SRC) }.unwrap();
    let image_mem_reqs = unsafe { device.get_buffer_requirements(&image_upload_buffer) };
    let memory_type_id = find_memory_type_id(&s.adapter, image_mem_reqs, Properties::CPU_VISIBLE);
    let image_upload_memory =
        unsafe { device.allocate_memory(memory_type_id, image_mem_reqs.size) }.unwrap();
    unsafe { device.bind_buffer_memory(&image_upload_memory, 0, &mut image_upload_buffer) }
        .unwrap();

    unsafe {
        device.destroy_buffer(image_upload_buffer);
        device.free_memory(image_upload_memory);
    }

    s.simple_textures.push(SingleTexture {
        texture_vertex_buffer: ManuallyDrop::new(texture_vertex_buffer),
        texture_vertex_memory: ManuallyDrop::new(texture_vertex_memory),
        texture_uv_buffer: ManuallyDrop::new(texture_uv_buffer),
        texture_uv_memory: ManuallyDrop::new(texture_uv_memory),
    });
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

pub fn set_triangle_color(s: &mut Windowing, inst: usize, rgba: [u8; 4]) {
    let device = &s.device;
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        const PTS: usize = 3;
        const COLORS: usize = 4;
        const COMPNTS: usize = 2;
        device.wait_idle().expect("Unable to wait for device idle");
        unsafe {
            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");

            let mut idx = inst
                * (size_of::<f32>() * COMPNTS * PTS + size_of::<u8>() * COLORS * PTS)
                / size_of::<f32>();
            let rgba = &transmute::<[u8; 4], [f32; 1]>(rgba);
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            idx += 3;
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            idx += 3;
            data_target[idx + 2..idx + 3].copy_from_slice(rgba);
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

pub fn rotate_to_triangles<T: Copy + Into<Rad<f32>>>(s: &mut Windowing, deg: T) {
    // NOTE: This algorithm sucks, we have to de-transform the triangle back to the origin
    // triangle, perform a rotation, and then re-translate the triangle back. This is highly
    // dubious and may result in drift due to float inaccuracies. A better method would be to use a
    // compute shader to compute the vertices.. but these vertices are completely arbitrary, so
    // that might not even work. Another way is to store rotation (2D) alongside the vertex, but
    // this is quite heavy... again, we'd need 3 floats (rotation) + 2 floats for the "average" of
    // this object in order to re-translate it.
    let device = &s.device;
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        const PTS: usize = 3;
        const COLORS: usize = 4;
        const COMPNTS: usize = 2;
        device.wait_idle().expect("Unable to wait for device idle");
        unsafe {
            let data_reader = device
                .acquire_mapping_reader::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");
            let mut vertices =
                Vec::<[f32; 6]>::with_capacity(debug_triangles.triangles_count * PTS);
            for i in 0..debug_triangles.triangles_count {
                let mut idx = i
                    * (size_of::<f32>() * COMPNTS * PTS + size_of::<u8>() * COLORS * PTS)
                    / size_of::<f32>();
                let card1 = &data_reader[idx..idx + 2];
                idx += 3;
                let card2 = &data_reader[idx..idx + 2];
                idx += 3;
                let card3 = &data_reader[idx..idx + 2];
                vertices.push([card1[0], card1[1], card2[0], card2[1], card3[0], card3[1]]);
            }
            device.release_mapping_reader(data_reader);

            for vert in vertices.iter_mut() {
                let off = average_xy(vert);
                let a = Matrix4::from_axis_angle(Vector3::new(0.0f32, 0.0, 1.0), deg)
                    * Vector4::new(vert[0], vert[1], 0.0, 1.0);
                let b = Matrix4::from_axis_angle(Vector3::new(0.0f32, 0.0, 1.0), deg)
                    * Vector4::new(vert[2], vert[3], 0.0, 1.0);
                let c = Matrix4::from_axis_angle(Vector3::new(0.0f32, 0.0, 1.0), deg)
                    * Vector4::new(vert[4], vert[5], 0.0, 1.0);
                vert[0] = a.x + off.0;
                vert[1] = a.y + off.1;
                vert[2] = b.x + off.0;
                vert[3] = b.y + off.1;
                vert[4] = c.x + off.0;
                vert[5] = c.y + off.1;
            }

            let mut data_target = device
                .acquire_mapping_writer::<f32>(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");

            for (i, vert) in vertices.iter().enumerate() {
                let mut idx = i
                    * (size_of::<f32>() * COMPNTS * PTS + size_of::<u8>() * COLORS * PTS)
                    / size_of::<f32>();
                data_target[idx..idx + 2].copy_from_slice(&vert[0..2]);
                idx += 3;
                data_target[idx..idx + 2].copy_from_slice(&vert[2..4]);
                idx += 3;
                data_target[idx..idx + 2].copy_from_slice(&vert[4..6]);
            }
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

pub fn add_to_triangles(s: &mut Windowing, triangle: &[f32; 6]) {
    let device = &s.device;
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        const PTS: usize = 3;
        const COLORS: usize = 4;
        const COMPNTS: usize = 2;
        const TRI_SIZE: usize = size_of::<f32>() * COMPNTS * PTS + size_of::<u8>() * COLORS * PTS;
        assert![
            (debug_triangles.triangles_count + 1) * TRI_SIZE <= debug_triangles.capacity as usize
        ];
        unsafe {
            let mut data_target = device
                .acquire_mapping_writer(
                    &debug_triangles.triangles_memory,
                    0..debug_triangles.capacity,
                )
                .expect("Failed to acquire a memory writer!");
            let mut idx = debug_triangles.triangles_count
                * (size_of::<f32>() * COMPNTS * PTS + size_of::<u8>() * COLORS * PTS)
                / size_of::<f32>();

            data_target[idx..idx + 2].copy_from_slice(&triangle[0..2]);
            data_target[idx + 2..idx + 3]
                .copy_from_slice(transmute::<&[u8; 4], &[f32; 1]>(&[255u8, 0, 0, 255]));
            idx += 3;
            data_target[idx..idx + 2].copy_from_slice(&triangle[2..4]);
            data_target[idx + 2..idx + 3]
                .copy_from_slice(transmute::<&[u8; 4], &[f32; 1]>(&[0u8, 255, 0, 255]));
            idx += 3;
            data_target[idx..idx + 2].copy_from_slice(&triangle[4..6]);
            data_target[idx + 2..idx + 3]
                .copy_from_slice(transmute::<&[u8; 4], &[f32; 1]>(&[0u8, 0, 255, 255]));
            debug_triangles.triangles_count += 1;
            device
                .release_mapping_writer(data_target)
                .expect("Couldn't release the mapping writer!");
        }
    }
}

pub fn pop_to_triangles(s: &mut Windowing) {
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        s.device.wait_idle().expect("device idle");
        debug_triangles.triangles_count -= 1;
    }
}

pub fn pop_n_triangles(s: &mut Windowing, n: usize) {
    if let Some(ref mut debug_triangles) = s.debug_triangles {
        s.device.wait_idle().expect("device idle");
        debug_triangles.triangles_count -= n;
    }
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
                    transmute::<*const f32, &[u32; 16]>(ptr),
                );
                for buffer_ref in &s.triangle_buffers {
                    let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.draw(0..3, 0..1);
                }
                if let Some(ref debug_triangles) = s.debug_triangles {
                    enc.bind_graphics_pipeline(&debug_triangles.pipeline);
                    let count = debug_triangles.triangles_count;
                    let buffers: ArrayVec<[_; 1]> = [(&debug_triangles.triangles_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    debug![log, "vxdraw", "mesh count"; "count" => count];
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
                command_buffers: std::iter::once(command_buffers),
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

pub fn average_xy(triangle: &mut [f32; 6]) -> (f32, f32) {
    let ax = (triangle[0] + triangle[2] + triangle[4]) / 3.0f32;
    let ay = (triangle[1] + triangle[3] + triangle[5]) / 3.0f32;
    triangle[0] -= ax;
    triangle[1] -= ay;
    triangle[2] -= ax;
    triangle[3] -= ay;
    triangle[4] -= ax;
    triangle[5] -= ay;
    (ax, ay)
}

pub fn gen_perspective(s: &mut Windowing) -> Matrix4<f32> {
    let size = s.swapconfig.extent;
    let pval = Vector2::new(size.height as f32, size.width as f32).normalize();
    Matrix4::from_nonuniform_scale(pval.x as f32, pval.y as f32, 1.0)
}

fn copy_image_to_rgb(s: &mut Windowing) -> Vec<u8> {
    let width = s.swapconfig.extent.width;
    let height = s.swapconfig.extent.height;

    let (buffer, memory, requirements) =
        make_transfer_buffer_of_size(s, u64::from(width * height * 3));
    let (imgbuf, imgmem, _imgreq) = make_transfer_img_of_size(s, width, height);
    let images = match s.backbuffer {
        Backbuffer::Images(ref images) => images,
        Backbuffer::Framebuffer(_) => unimplemented![],
    };
    unsafe {
        s.device
            .wait_for_fence(&s.frame_render_fences[s.current_frame], u64::max_value())
            .expect("Unable to wait for fence");
    }
    unsafe {
        let mut cmd_buffer = s
            .command_pool
            .acquire_command_buffer::<gfx_hal::command::OneShot>();
        cmd_buffer.begin();
        let image_barrier = gfx_hal::memory::Barrier::Image {
            states: (gfx_hal::image::Access::empty(), image::Layout::Present)
                ..(
                    gfx_hal::image::Access::TRANSFER_READ,
                    image::Layout::TransferSrcOptimal,
                ),
            target: &images[s.swapchain_prev_idx as usize],
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        let dstbarrier = gfx_hal::memory::Barrier::Image {
            states: (gfx_hal::image::Access::empty(), image::Layout::Undefined)
                ..(
                    gfx_hal::image::Access::TRANSFER_WRITE,
                    image::Layout::General,
                ),
            target: &imgbuf,
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        cmd_buffer.pipeline_barrier(
            PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
            gfx_hal::memory::Dependencies::empty(),
            &[image_barrier, dstbarrier],
        );
        cmd_buffer.blit_image(
            &images[s.swapchain_prev_idx as usize],
            image::Layout::TransferSrcOptimal,
            &imgbuf,
            image::Layout::General,
            image::Filter::Nearest,
            once(command::ImageBlit {
                src_subresource: image::SubresourceLayers {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layers: 0..1,
                },
                src_bounds: image::Offset { x: 0, y: 0, z: 0 }..image::Offset {
                    x: width as i32,
                    y: height as i32,
                    z: 1,
                },
                dst_subresource: image::SubresourceLayers {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layers: 0..1,
                },
                dst_bounds: image::Offset { x: 0, y: 0, z: 0 }..image::Offset {
                    x: width as i32,
                    y: height as i32,
                    z: 1,
                },
            }),
        );
        let image_barrier = gfx_hal::memory::Barrier::Image {
            states: (
                gfx_hal::image::Access::TRANSFER_READ,
                image::Layout::TransferSrcOptimal,
            )..(gfx_hal::image::Access::empty(), image::Layout::Present),
            target: &images[s.swapchain_prev_idx as usize],
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        cmd_buffer.pipeline_barrier(
            PipelineStage::TRANSFER..PipelineStage::BOTTOM_OF_PIPE,
            gfx_hal::memory::Dependencies::empty(),
            &[image_barrier],
        );
        cmd_buffer.finish();
        let the_command_queue = &mut s.queue_group.queues[0];
        let fence = s
            .device
            .create_fence(false)
            .expect("Unable to create fence");
        the_command_queue.submit_nosemaphores(once(&cmd_buffer), Some(&fence));
        s.device
            .wait_for_fence(&fence, u64::max_value())
            .expect("unable to wait for fence");
        s.device.destroy_fence(fence);
    }
    unsafe {
        let mut cmd_buffer = s
            .command_pool
            .acquire_command_buffer::<gfx_hal::command::OneShot>();
        cmd_buffer.begin();
        let image_barrier = gfx_hal::memory::Barrier::Image {
            states: (gfx_hal::image::Access::empty(), image::Layout::Undefined)
                ..(
                    gfx_hal::image::Access::TRANSFER_READ,
                    image::Layout::TransferSrcOptimal,
                ),
            target: &imgbuf,
            families: None,
            range: image::SubresourceRange {
                aspects: format::Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            },
        };
        cmd_buffer.pipeline_barrier(
            PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
            gfx_hal::memory::Dependencies::empty(),
            &[image_barrier],
        );
        cmd_buffer.copy_image_to_buffer(
            &imgbuf,
            image::Layout::TransferSrcOptimal,
            &buffer,
            once(command::BufferImageCopy {
                buffer_offset: 0,
                buffer_width: width,
                buffer_height: height,
                image_layers: image::SubresourceLayers {
                    aspects: format::Aspects::COLOR,
                    level: 0,
                    layers: 0..1,
                },
                image_offset: image::Offset { x: 0, y: 0, z: 0 },
                image_extent: image::Extent {
                    width,
                    height,
                    depth: 1,
                },
            }),
        );
        cmd_buffer.finish();
        let the_command_queue = &mut s.queue_group.queues[0];
        let fence = s
            .device
            .create_fence(false)
            .expect("Unable to create fence");
        the_command_queue.submit_nosemaphores(once(&cmd_buffer), Some(&fence));
        s.device
            .wait_for_fence(&fence, u64::max_value())
            .expect("unable to wait for fence");
        s.device.destroy_fence(fence);
    }
    unsafe {
        let reader = s
            .device
            .acquire_mapping_reader::<u8>(&memory, 0..requirements.size as u64)
            .expect("Unable to open reader");
        let result = reader
            .iter()
            .take((3 * width * height) as usize)
            .cloned()
            .collect::<Vec<_>>();
        s.device.release_mapping_reader(reader);
        s.device.destroy_buffer(buffer);
        s.device.free_memory(memory);
        s.device.destroy_image(imgbuf);
        s.device.free_memory(imgmem);
        result
    }
}

pub struct Alignment(u64);
fn align_top(alignment: Alignment, value: u64) -> u64 {
    if value % alignment.0 != 0 {
        let alig = value + (alignment.0 - value % alignment.0);
        assert![alig % alignment.0 == 0];
        alig
    } else {
        value
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

    fn make_centered_equilateral_triangle() -> [f32; 6] {
        let mut tri = [0.0f32; 6];
        static PI: f32 = std::f32::consts::PI;
        tri[2] = 1.0f32 * (60.0f32 / 180.0f32 * PI).cos();
        tri[3] = -1.0f32 * (60.0f32 / 180.0f32 * PI).sin();
        tri[4] = 1.0f32;
        let avg_x = (tri[0] + tri[2] + tri[4]) / 3.0f32;
        let avg_y = (tri[1] + tri[3] + tri[5]) / 3.0f32;
        tri[0] -= avg_x;
        tri[1] -= avg_y;
        tri[2] -= avg_x;
        tri[3] -= avg_y;
        tri[4] -= avg_x;
        tri[5] -= avg_y;
        tri
    }

    fn add_windmills(windowing: &mut Windowing) {
        use rand::Rng;
        let mut rng = random::new(0);
        for _ in 0..1000 {
            let mut tri = make_centered_equilateral_triangle();
            let (dx, dy) = (
                rng.gen_range(-1.0f32, 1.0f32),
                rng.gen_range(-1.0f32, 1.0f32),
            );
            let scale = rng.gen_range(0.03f32, 0.1f32);
            for idx in 0..tri.len() {
                tri[idx] *= scale;
            }
            tri[0] += dx;
            tri[1] += dy;
            tri[2] += dx;
            tri[3] += dy;
            tri[4] += dx;
            tri[5] += dy;
            add_to_triangles(windowing, &tri);
        }
    }

    fn remove_windmills(windowing: &mut Windowing) {
        pop_n_triangles(windowing, 1000);
    }

    fn add_4_screencorners(windowing: &mut Windowing) {
        add_to_triangles(windowing, &[-1.0f32, -1.0, 0.0, -1.0, -1.0, 0.0]);
        add_to_triangles(windowing, &[-1.0f32, 1.0, 0.0, 1.0, -1.0, 0.0]);
        add_to_triangles(windowing, &[1.0f32, -1.0, 0.0, -1.0, 1.0, 0.0]);
        add_to_triangles(windowing, &[1.0f32, 1.0, 0.0, 1.0, 1.0, 0.0]);
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
        let tri = make_centered_equilateral_triangle();

        add_to_triangles(&mut windowing, &tri);
        add_4_screencorners(&mut windowing);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle", img);
    }

    #[test]
    fn simple_triangle_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);
        let tri = make_centered_equilateral_triangle();

        add_to_triangles(&mut windowing, &tri);
        set_triangle_color(&mut windowing, 0, [255, 0, 255, 255]);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle_change_color", img);
    }

    #[test]
    fn windmills() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills", img);
    }

    #[test]
    fn windmills_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing);
        set_triangle_color(&mut windowing, 0, [255, 0, 0, 255]);
        set_triangle_color(&mut windowing, 249, [0, 255, 0, 255]);
        set_triangle_color(&mut windowing, 499, [0, 0, 255, 255]);
        set_triangle_color(&mut windowing, 999, [0, 0, 0, 255]);

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
    fn rotating_windmills_drawing_invariant() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing);
        for _ in 0..30 {
            rotate_to_triangles(&mut windowing, Deg(1.0f32));
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
        remove_windmills(&mut windowing);

        add_windmills(&mut windowing);
        for _ in 0..30 {
            rotate_to_triangles(&mut windowing, Deg(1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
    }

    // ---

    #[bench]
    fn bench_simple_triangle(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        let tri = make_centered_equilateral_triangle();
        add_to_triangles(&mut windowing, &tri);
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

        add_windmills(&mut windowing);

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_windmills_set_color(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        add_windmills(&mut windowing);

        b.iter(|| {
            set_triangle_color(&mut windowing, 0, black_box([0, 0, 0, 255]));
        });
    }

    #[bench]
    fn bench_rotating_windmills(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&mut windowing);

        add_windmills(&mut windowing);

        b.iter(|| {
            rotate_to_triangles(&mut windowing, Deg(1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_rotating_windmills_no_render(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        add_windmills(&mut windowing);

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
}
