use crate::glocals::{Log, Windowing};
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
// use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use arrayvec::ArrayVec;
use gfx_hal::{
    adapter::PhysicalDevice,
    command::{self, ClearColor, ClearValue},
    device::Device,
    format::{self, ChannelType, Swizzle},
    image, pass, pool,
    pso::{
        self, AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendOp, BlendState,
        ColorBlendDesc, ColorMask, DepthStencilDesc, DepthTest, DescriptorSetLayoutBinding,
        Element, Face, Factor, FrontFace, GraphicsPipelineDesc, InputAssemblerDesc, LogicOp,
        PipelineCreationFlags, PipelineStage, PolygonMode, Rasterizer, Rect, ShaderStageFlags,
        StencilTest, VertexBufferDesc, Viewport,
    },
    queue::Submission,
    window::{Extent2D, PresentMode::*, Surface, Swapchain},
    Backbuffer, Backend, FrameSync, Instance, Primitive, SwapchainConfig,
};
use logger::{info, warn, InDebug, InDebugPretty, Logger};
use std::io::Read;
use std::mem::{size_of, ManuallyDrop};
use winit::{Event, EventsLoop, Window};

// ---

pub fn init_window_with_vulkan(log: &mut Logger<Log>) -> Windowing {
    let events_loop = EventsLoop::new();
    let window = Window::new(&events_loop).unwrap();
    let version = 1;
    let vk_inst = back::Instance::create("renderer", version);
    let mut surf: <back::Backend as Backend>::Surface = vk_inst.create_surface(&window);
    let mut adapters = vk_inst.enumerate_adapters();
    let len = adapters.len();
    info![log, "vxdraw", "Adapters found"; "count" => len];
    for (idx, adap) in adapters.iter().enumerate() {
        let info = adap.info.clone();
        info![log, "vxdraw", "Adapter found"; "idx" => idx, "info" => InDebugPretty(&info)];
    }
    // TODO Find appropriate adapter, I've never seen a case where we have 2+ adapters, that time
    // will come one day
    let adapter = adapters.remove(0);
    let (device, queue_group) = adapter
        .open_with::<_, gfx_hal::Graphics>(1, |family| surf.supports_queue_family(family))
        .expect("Unable to find device supporting graphics");

    let (caps, formats, present_modes, _composite_alpha) =
        surf.compatibility(&adapter.physical_device);
    let format = formats.map_or(format::Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .cloned()
            .unwrap_or(formats[0])
    });

    {
        let present_modes = present_modes.clone();
        info![log, "vxdraw", "Present modes"; "modes" => InDebugPretty(&present_modes)];
    }

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

    let dpi_factor = window.get_hidpi_factor();
    info![log, "vxdraw", "Window DPI factor"; "factor" => dpi_factor];

    let (w, h): (u32, u32) = window
        .get_inner_size()
        .unwrap()
        .to_physical(dpi_factor)
        .into();
    let dims = Extent2D {
        width: w,
        height: h,
    };
    info![log, "vxdraw", "Swapchain size"; "extent" => InDebug(&dims)];

    let mut swap_config = SwapchainConfig::from_caps(&caps, format, dims);
    swap_config.present_mode = present_mode;
    swap_config.image_count = image_count;
    {
        let swap_config = swap_config.clone();
        info![log, "vxdraw", "Swapchain final configuration"; "swapchain" => InDebugPretty(&swap_config)];
    }

    let (swapchain, backbuffer) = unsafe { device.create_swapchain(&mut surf, swap_config, None) }
        .expect("Unable to create swapchain");

    let backbuffer_string = format!["{:#?}", backbuffer];
    info![log, "vxdraw", "Backbuffer information"; "backbuffers" => backbuffer_string];

    let image_views: Vec<_> = match backbuffer {
        Backbuffer::Images(images) => images
            .into_iter()
            .map(|image| unsafe {
                device
                    .create_image_view(
                        &image,
                        image::ViewKind::D2,
                        format,
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
        unsafe {
            device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .map_err(|_| "Couldn't create a render pass!")
                .unwrap()
        }
    };

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

    // Section 2, shaders
    pub const VERTEX_SOURCE: &str = "#version 450
    #extension GL_ARG_separate_shader_objects : enable
    layout (location = 0) in vec2 position;
    out gl_PerVertex {
        vec4 gl_Position;
    };
    void main()
    {
      gl_Position = vec4(position, 0.0, 1.0);
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

    // Create a render pass for this thing
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
    let descriptor_set_layouts: Vec<<back::Backend as Backend>::DescriptorSetLayout> =
        vec![unsafe {
            device
                .create_descriptor_set_layout(bindings, immutable_samplers)
                .expect("Couldn't make a DescriptorSetLayout")
        }];
    let push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
    let triangle_pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&descriptor_set_layouts, push_constants)
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

    Windowing {
        adapter,
        command_buffers,
        command_pool: ManuallyDrop::new(command_pool),
        current_frame: 0,
        device: ManuallyDrop::new(device),
        events_loop,
        frame_render_fences,
        acquire_image_semaphores,
        present_wait_semaphores,
        framebuffers,
        image_count: image_count as usize,
        image_views,
        queue_group: ManuallyDrop::new(queue_group),
        render_area: Rect {
            x: 0,
            y: 0,
            w: w as i16,
            h: h as i16,
        },
        triangle_buffers: vec![],
        triangle_memory: vec![],
        triangle_render_pass: ManuallyDrop::new(triangle_render_pass),
        triangle_pipeline: ManuallyDrop::new(triangle_pipeline),
        triangle_pipeline_layout: ManuallyDrop::new(triangle_pipeline_layout),
        render_pass: ManuallyDrop::new(render_pass),
        surf,
        swapchain: ManuallyDrop::new(swapchain),
        vk_inst: ManuallyDrop::new(vk_inst),
        window,
    }
}

pub fn add_triangle(s: &mut Windowing, triangle: &[f32; 6]) {
    let (buffer, memory, requirements) = unsafe {
        const F32_XY_TRIANGLE: u64 = (std::mem::size_of::<f32>() * 2 * 3) as u64;
        use gfx_hal::{adapter::MemoryTypeId, memory::Properties};
        let mut buffer = s
            .device
            .create_buffer(F32_XY_TRIANGLE, gfx_hal::buffer::Usage::VERTEX)
            .expect("cant make bf");
        let requirements = s.device.get_buffer_requirements(&buffer);
        let memory_type_id = s
            .adapter
            .physical_device
            .memory_properties()
            .memory_types
            .iter()
            .enumerate()
            .find(|&(id, memory_type)| {
                requirements.type_mask & (1 << id) != 0
                    && memory_type.properties.contains(Properties::CPU_VISIBLE)
            })
            .map(|(id, _)| MemoryTypeId(id))
            .unwrap();
        let memory = s
            .device
            .allocate_memory(memory_type_id, requirements.size)
            .expect("Couldn't allocate vertex buffer memory");
        s.device
            .bind_buffer_memory(&memory, 0, &mut buffer)
            .expect("Couldn't bind the buffer memory!");
        // (buffer, memory, requirements)
        (buffer, memory, requirements)
    };
    // Upload vertex data
    unsafe {
        let mut data_target = s
            .device
            .acquire_mapping_writer(&memory, 0..requirements.size)
            .expect("Failed to acquire a memory writer!");
        data_target[..6].copy_from_slice(triangle);
        s.device
            .release_mapping_writer(data_target)
            .expect("Couldn't release the mapping writer!");
    }
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

pub fn draw_frame(s: &mut Windowing, log: &mut Logger<Log>) {
    let frame_render_fence = &s.frame_render_fences[s.current_frame];
    let acquire_image_semaphore = &s.acquire_image_semaphores[s.current_frame];
    let present_wait_semaphore = &s.present_wait_semaphores[s.current_frame];
    let frame = s.current_frame;
    info![log, "vxdraw", "Current frame"; "frame" => frame];

    let image_index;
    unsafe {
        image_index = s
            .swapchain
            .acquire_image(
                u64::max_value(),
                FrameSync::Semaphore(acquire_image_semaphore),
            )
            .unwrap();
        info![log, "vxdraw", "Acquired image index"; "index" => image_index];
        assert_eq![image_index as usize, s.current_frame];

        info![log, "vxdraw", "Waiting for fence"];
        s.device
            .wait_for_fence(frame_render_fence, u64::max_value())
            .unwrap();
        info![log, "vxdraw", "Resetting fence"];
        s.device.reset_fence(frame_render_fence).unwrap();

        {
            let buffer = &mut s.command_buffers[s.current_frame];
            let clear_values = [ClearValue::Color(ClearColor::Float([
                1.0f32, 0.0, 0.0, 1.0,
            ]))];
            buffer.begin(false);
            {
                let mut enc = buffer.begin_render_pass_inline(
                    &s.render_pass,
                    &s.framebuffers[s.current_frame],
                    s.render_area,
                    clear_values.iter(),
                );
                enc.bind_graphics_pipeline(&s.triangle_pipeline);
                for buffer_ref in &s.triangle_buffers {
                    let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.draw(0..3, 0..1);
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
        let signal_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
        // yes, you have to write it twice like this. yes, it's silly.
        let present_wait_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
        let submission = Submission {
            command_buffers: std::iter::once(command_buffers),
            wait_semaphores,
            signal_semaphores,
        };
        let the_command_queue = &mut s.queue_group.queues[0];
        the_command_queue.submit(submission, Some(frame_render_fence));
        s.swapchain
            .present(the_command_queue, image_index, present_wait_semaphores)
            .unwrap();
    }
    s.current_frame = (s.current_frame + 1) % s.image_count;
}

// ---

#[cfg(feature = "gfx_tests")]
#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn init_window_and_get_input() {
        let mut logger = Logger::spawn();
        logger.set_colorize(true);
        let mut windowing = init_window_with_vulkan(&mut logger);
        collect_input(&mut windowing);
        add_triangle(&mut windowing, &[0.0f32, 0.0, 1.0, 1.0, 1.0, 0.0]);
        add_triangle(&mut windowing, &[-0.5f32, 0.5, 0.2, 0.2, 0.3, 0.3]);
        for _ in 0..300 {
            draw_frame(&mut windowing, &mut logger);
            std::thread::sleep(std::time::Duration::new(0, 8_000_000));
        }
    }

    #[bench]
    fn clears_per_second(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        logger.set_colorize(true);
        let mut windowing = init_window_with_vulkan(&mut logger);
        b.iter(|| {
            draw_frame(&mut windowing, &mut logger);
        });
    }
}
