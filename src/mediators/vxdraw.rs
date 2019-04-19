use crate::glocals::{vxdraw::Windowing, Log};
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
    image, memory, pass, pool,
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
use logger::{debug, info, trace, warn, InDebug, InDebugPretty, Logger};
use std::io::Read;
use std::iter::once;
use std::mem::{size_of, ManuallyDrop};
use winit::{dpi::LogicalSize, Event, EventsLoop, WindowBuilder};

pub mod debtri;
pub mod dyntex;
pub mod quads;
pub mod strtex;
pub mod utils;

use debtri::{DebugTriangle, DebugTriangleHandle};
use dyntex::*;
use strtex::{add_streaming_texture, streaming_texture_add_sprite};
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

#[cfg(not(feature = "gl"))]
fn set_window_size(window: &mut winit::Window, show: ShowWindow) -> Extent2D {
    let dpi_factor = window.get_hidpi_factor();
    let (w, h): (u32, u32) = match show {
        ShowWindow::Headless1k => {
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
            (1000, 1000)
        }
        ShowWindow::Headless2x1k => {
            window.set_inner_size(LogicalSize {
                width: 2000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
            (2000, 1000)
        }
        ShowWindow::Headless1x2k => {
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 2000f64 / dpi_factor,
            });
            (1000, 2000)
        }
        ShowWindow::Enable => window
            .get_inner_size()
            .unwrap()
            .to_physical(dpi_factor)
            .into(),
    };
    Extent2D {
        width: w,
        height: h,
    }
}

#[cfg(feature = "gl")]
fn set_window_size(window: &mut glutin::GlWindow, show: ShowWindow) -> Extent2D {
    let dpi_factor = window.get_hidpi_factor();
    let (w, h): (u32, u32) = match show {
        ShowWindow::Headless1k => {
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
            (1000, 1000)
        }
        ShowWindow::Headless2x1k => {
            window.set_inner_size(LogicalSize {
                width: 2000f64 / dpi_factor,
                height: 1000f64 / dpi_factor,
            });
            (2000, 1000)
        }
        ShowWindow::Headless1x2k => {
            window.set_inner_size(LogicalSize {
                width: 1000f64 / dpi_factor,
                height: 2000f64 / dpi_factor,
            });
            (1000, 2000)
        }
        ShowWindow::Enable => window
            .get_inner_size()
            .unwrap()
            .to_physical(dpi_factor)
            .into(),
    };
    Extent2D {
        width: w,
        height: h,
    }
}

pub fn init_window_with_vulkan(log: &mut Logger<Log>, show: ShowWindow) -> Windowing {
    #[cfg(feature = "gl")]
    static BACKEND: &str = "OpenGL";
    #[cfg(feature = "vulkan")]
    static BACKEND: &str = "Vulkan";
    #[cfg(feature = "metal")]
    static BACKEND: &str = "Metal";
    #[cfg(feature = "dx12")]
    static BACKEND: &str = "Dx12";

    info![log, "vxdraw", "Initializing rendering"; "show" => InDebug(&show), "backend" => BACKEND];

    let events_loop = EventsLoop::new();
    let window_builder = WindowBuilder::new().with_visibility(show == ShowWindow::Enable);

    #[cfg(feature = "gl")]
    let (mut adapters, mut surf, dims) = {
        let mut window = {
            let builder = back::config_context(
                back::glutin::ContextBuilder::new(),
                format::Format::Rgba8Srgb,
                None,
            )
            .with_vsync(true);
            back::glutin::GlWindow::new(window_builder, builder, &events_loop).unwrap()
        };

        set_window_size(&mut window, show);
        let dims = {
            let dpi_factor = window.get_hidpi_factor();
            debug![log, "vxdraw", "Window DPI factor"; "factor" => dpi_factor];
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

        let surface = back::Surface::from_window(window);
        let adapters = surface.enumerate_adapters();
        (adapters, surface, dims)
    };

    #[cfg(not(feature = "gl"))]
    let (window, vk_inst, mut adapters, mut surf, dims) = {
        let mut window = window_builder.build(&events_loop).unwrap();
        let version = 1;
        let vk_inst = back::Instance::create("renderer", version);
        let surf: <back::Backend as Backend>::Surface = vk_inst.create_surface(&window);
        let adapters = vk_inst.enumerate_adapters();
        let dims = set_window_size(&mut window, show);
        let dpi_factor = window.get_hidpi_factor();
        debug![log, "vxdraw", "Window DPI factor"; "factor" => dpi_factor];
        (window, vk_inst, adapters, surf, dims)
    };

    // ---

    {
        let len = adapters.len();
        debug![log, "vxdraw", "Adapters found"; "count" => len];
    }

    for (idx, adap) in adapters.iter().enumerate() {
        let info = adap.info.clone();
        let limits = adap.physical_device.limits();
        debug![log, "vxdraw", "Adapter found"; "idx" => idx, "info" => InDebugPretty(&info), "device limits" => InDebugPretty(&limits)];
    }

    // TODO Find appropriate adapter, I've never seen a case where we have 2+ adapters, that time
    // will come one day
    let adapter = adapters.remove(0);
    let (device, queue_group) = adapter
        .open_with::<_, gfx_hal::Graphics>(1, |family| surf.supports_queue_family(family))
        .expect("Unable to find device supporting graphics");

    let phys_dev_limits = adapter.physical_device.limits();

    let (caps, formats, present_modes, composite_alpha) =
        surf.compatibility(&adapter.physical_device);

    debug![log, "vxdraw", "Surface capabilities"; "capabilities" => InDebugPretty(&caps); clone caps];
    debug![log, "vxdraw", "Formats available"; "formats" => InDebugPretty(&formats); clone formats];
    debug![log, "vxdraw", "Composition"; "alpha" => InDebugPretty(&composite_alpha); clone composite_alpha];
    let format = formats.map_or(format::Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .cloned()
            .unwrap_or(formats[0])
    });

    debug![log, "vxdraw", "Format chosen"; "format" => InDebugPretty(&format); clone format];
    debug![log, "vxdraw", "Available present modes"; "modes" => InDebugPretty(&present_modes); clone present_modes];

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
    debug![log, "vxdraw", "Using best possible present mode"; "mode" => InDebug(&present_mode)];

    let image_count = if present_mode == Mailbox {
        (caps.image_count.end - 1)
            .min(3)
            .max(caps.image_count.start)
    } else {
        (caps.image_count.end - 1)
            .min(2)
            .max(caps.image_count.start)
    };
    debug![log, "vxdraw", "Using swapchain images"; "count" => image_count];

    debug![log, "vxdraw", "Swapchain size"; "extent" => InDebug(&dims)];

    let mut swap_config = SwapchainConfig::from_caps(&caps, format, dims);
    swap_config.present_mode = present_mode;
    swap_config.image_count = image_count;
    swap_config.extent = dims;
    if caps.usage.contains(image::Usage::TRANSFER_SRC) {
        swap_config.image_usage |= gfx_hal::image::Usage::TRANSFER_SRC;
    } else {
        warn![
            log,
            "vxdraw", "Surface does not support TRANSFER_SRC, may fail during testing"
        ];
    }

    debug![log, "vxdraw", "Swapchain final configuration"; "swapchain" => InDebugPretty(&swap_config); clone swap_config];

    let (swapchain, backbuffer) =
        unsafe { device.create_swapchain(&mut surf, swap_config.clone(), None) }
            .expect("Unable to create swapchain");

    let backbuffer_string = format!["{:#?}", backbuffer];
    debug![log, "vxdraw", "Backbuffer information"; "backbuffers" => backbuffer_string];

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
        debug![log, "vxdraw", "Render pass info"; "color attachment" => InDebugPretty(&color_attachment); clone color_attachment];
        unsafe {
            device
                .create_render_pass(&[color_attachment, depth], &[subpass], &[])
                .map_err(|_| "Couldn't create a render pass!")
                .unwrap()
        }
    };

    {
        let rpfmt = format!["{:#?}", render_pass];
        debug![log, "vxdraw", "Created render pass for framebuffers"; "renderpass" => rpfmt];
    }

    let mut depth_images: Vec<<back::Backend as Backend>::Image> = vec![];
    let mut depth_image_views: Vec<<back::Backend as Backend>::ImageView> = vec![];
    let mut depth_image_memories: Vec<<back::Backend as Backend>::Memory> = vec![];
    let mut depth_image_requirements: Vec<memory::Requirements> = vec![];

    let (image_views, framebuffers) = match backbuffer {
        Backbuffer::Images(ref images) => {
            let image_views = images
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
                .unwrap();

            unsafe {
                for _ in &image_views {
                    let mut depth_image = device
                        .create_image(
                            image::Kind::D2(dims.width, dims.height, 1, 1),
                            1,
                            format::Format::D32Float,
                            image::Tiling::Optimal,
                            image::Usage::DEPTH_STENCIL_ATTACHMENT,
                            image::ViewCapabilities::empty(),
                        )
                        .expect("Unable to create depth image");
                    let requirements = device.get_image_requirements(&depth_image);
                    let memory_type_id = find_memory_type_id(
                        &adapter,
                        requirements,
                        memory::Properties::DEVICE_LOCAL,
                    );
                    let memory = device
                        .allocate_memory(memory_type_id, requirements.size)
                        .expect("Couldn't allocate image memory!");
                    device
                        .bind_image_memory(&memory, 0, &mut depth_image)
                        .expect("Couldn't bind the image memory!");
                    let image_view = device
                        .create_image_view(
                            &depth_image,
                            image::ViewKind::D2,
                            format::Format::D32Float,
                            format::Swizzle::NO,
                            image::SubresourceRange {
                                aspects: format::Aspects::DEPTH,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        )
                        .expect("Couldn't create the image view!");
                    depth_images.push(depth_image);
                    depth_image_views.push(image_view);
                    depth_image_requirements.push(requirements);
                    depth_image_memories.push(memory);
                }
            }
            // pub image: ManuallyDrop<B::Image>,
            // pub requirements: Requirements,
            // pub memory: ManuallyDrop<B::Memory>,
            // pub image_view: ManuallyDrop<B::ImageView>,
            let framebuffers: Vec<<back::Backend as Backend>::Framebuffer> = {
                image_views
                    .iter()
                    .enumerate()
                    .map(|(idx, image_view)| unsafe {
                        device
                            .create_framebuffer(
                                &render_pass,
                                vec![image_view, &depth_image_views[idx]],
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
            (image_views, framebuffers)
        }
        #[cfg(not(feature = "gl"))]
        Backbuffer::Framebuffer(_) => unimplemented![],
        #[cfg(feature = "gl")]
        Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
    };

    {
        let image_views = format!["{:?}", image_views];
        debug![log, "vxdraw", "Created image views"; "image views" => image_views];
    }

    let framebuffers_string = format!["{:#?}", framebuffers];
    debug![log, "vxdraw", "Framebuffer information"; "framebuffers" => framebuffers_string];

    let max_frames_in_flight = 3;
    assert![max_frames_in_flight > 0];

    let mut frames_in_flight_fences = vec![];
    let mut present_wait_semaphores = vec![];
    for _ in 0..max_frames_in_flight {
        frames_in_flight_fences.push(device.create_fence(true).expect("Can't create fence"));
        present_wait_semaphores.push(device.create_semaphore().expect("Can't create semaphore"));
    }

    let acquire_image_semaphores = (0..image_count)
        .map(|_| device.create_semaphore().expect("Can't create semaphore"))
        .collect::<Vec<_>>();

    {
        let count = frames_in_flight_fences.len();
        debug![log, "vxdraw", "Allocated fences and semaphores"; "count" => count];
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

    let mut windowing = Windowing {
        acquire_image_semaphores,
        acquire_image_semaphore_free: ManuallyDrop::new(
            device
                .create_semaphore()
                .expect("Unable to create semaphore"),
        ),
        adapter,
        backbuffer,
        command_buffers,
        command_pool: ManuallyDrop::new(command_pool),
        current_frame: 0,
        max_frames_in_flight,
        debtris: None,
        device: ManuallyDrop::new(device),
        device_limits: phys_dev_limits,
        events_loop,
        frames_in_flight_fences,
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
        swapconfig: swap_config,
        strtexs: vec![],
        dyntexs: vec![],
        quads: None,
        depth_images,
        depth_image_views,
        depth_image_requirements,
        depth_image_memories,
        #[cfg(not(feature = "gl"))]
        vk_inst: ManuallyDrop::new(vk_inst),
        #[cfg(not(feature = "gl"))]
        window,
    };
    debtri::create_debug_triangle(&mut windowing);
    quads::create_quad(&mut windowing, log);
    windowing
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
    draw_frame_internal(s, log, view, |_, _| {});
}

fn draw_frame_internal<T>(
    s: &mut Windowing,
    log: &mut Logger<Log>,
    view: &Matrix4<f32>,
    postproc: fn(&mut Windowing, gfx_hal::window::SwapImageIndex) -> T,
) -> T {
    let postproc_res = unsafe {
        let swap_image = s
            .swapchain
            .acquire_image(
                u64::max_value(),
                FrameSync::Semaphore(&*s.acquire_image_semaphore_free),
            )
            .unwrap();

        core::mem::swap(
            &mut *s.acquire_image_semaphore_free,
            &mut s.acquire_image_semaphores[swap_image as usize],
        );

        s.device
            .wait_for_fence(
                &s.frames_in_flight_fences[s.current_frame],
                u64::max_value(),
            )
            .unwrap();

        s.device
            .reset_fence(&s.frames_in_flight_fences[s.current_frame])
            .unwrap();

        {
            let current_frame = s.current_frame;
            let texture_count = s.dyntexs.len();
            let debugtris_cnt = s.debtris.as_ref().map_or(0, |x| x.triangles_count);
            trace![log, "vxdraw", "Drawing frame"; "swapchain image" => swap_image, "flight" => current_frame, "textures" => texture_count, "debug triangles" => debugtris_cnt];
        }

        {
            let buffer = &mut s.command_buffers[s.current_frame];
            let clear_values = [
                ClearValue::Color(ClearColor::Float([1.0f32, 0.25, 0.5, 0.75])),
                ClearValue::DepthStencil(gfx_hal::command::ClearDepthStencil(1.0, 0)),
            ];
            buffer.begin(false);
            for strtex in s.strtexs.iter() {
                let image_barrier = memory::Barrier::Image {
                    states: (image::Access::empty(), image::Layout::General)
                        ..(
                            image::Access::SHADER_READ,
                            image::Layout::ShaderReadOnlyOptimal,
                        ),
                    target: &*strtex.image_buffer,
                    families: None,
                    range: image::SubresourceRange {
                        aspects: format::Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };
                buffer.pipeline_barrier(
                    PipelineStage::TOP_OF_PIPE..PipelineStage::FRAGMENT_SHADER,
                    memory::Dependencies::empty(),
                    &[image_barrier],
                );
                // Submit automatically makes host writes available for the device
                let image_barrier = memory::Barrier::Image {
                    states: (image::Access::empty(), image::Layout::ShaderReadOnlyOptimal)
                        ..(image::Access::empty(), image::Layout::General),
                    target: &*strtex.image_buffer,
                    families: None,
                    range: image::SubresourceRange {
                        aspects: format::Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };
                buffer.pipeline_barrier(
                    PipelineStage::FRAGMENT_SHADER..PipelineStage::HOST,
                    memory::Dependencies::empty(),
                    &[image_barrier],
                );
            }
            {
                let mut enc = buffer.begin_render_pass_inline(
                    &s.render_pass,
                    &s.framebuffers[swap_image as usize],
                    s.render_area,
                    clear_values.iter(),
                );

                for strtex in s.strtexs.iter() {
                    enc.bind_graphics_pipeline(&strtex.pipeline);
                    enc.push_graphics_constants(
                        &strtex.pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        &*(view.as_ptr() as *const [u32; 16]),
                    );
                    enc.bind_graphics_descriptor_sets(
                        &strtex.pipeline_layout,
                        0,
                        Some(&*strtex.descriptor_set),
                        &[],
                    );
                    let buffers: ArrayVec<[_; 1]> = [(&*strtex.vertex_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.bind_index_buffer(gfx_hal::buffer::IndexBufferView {
                        buffer: &strtex.vertex_buffer_indices,
                        offset: 0,
                        index_type: gfx_hal::IndexType::U16,
                    });
                    enc.draw_indexed(0..strtex.count * 6, 0, 0..1);
                }

                for dyntex in s.dyntexs.iter() {
                    enc.bind_graphics_pipeline(&dyntex.pipeline);
                    enc.push_graphics_constants(
                        &dyntex.pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        &*(view.as_ptr() as *const [u32; 16]),
                    );
                    enc.bind_graphics_descriptor_sets(
                        &dyntex.pipeline_layout,
                        0,
                        Some(&*dyntex.descriptor_set),
                        &[],
                    );
                    let buffers: ArrayVec<[_; 1]> = [(&*dyntex.texture_vertex_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.bind_index_buffer(gfx_hal::buffer::IndexBufferView {
                        buffer: &dyntex.texture_vertex_buffer_indices,
                        offset: 0,
                        index_type: gfx_hal::IndexType::U16,
                    });
                    enc.draw_indexed(0..dyntex.count * 6, 0, 0..1);
                }

                if let Some(ref quads) = s.quads {
                    enc.bind_graphics_pipeline(&quads.pipeline);
                    enc.push_graphics_constants(
                        &quads.pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        &*(view.as_ptr() as *const [u32; 16]),
                    );
                    let buffers: ArrayVec<[_; 1]> = [(&quads.quads_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.bind_index_buffer(gfx_hal::buffer::IndexBufferView {
                        buffer: &quads.quads_buffer_indices,
                        offset: 0,
                        index_type: gfx_hal::IndexType::U16,
                    });
                    enc.draw_indexed(0..quads.count as u32 * 6, 0, 0..1);
                }

                if let Some(ref debtris) = s.debtris {
                    enc.bind_graphics_pipeline(&debtris.pipeline);
                    let ratio =
                        s.swapconfig.extent.width as f32 / s.swapconfig.extent.height as f32;
                    enc.push_graphics_constants(
                        &debtris.pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        &(std::mem::transmute::<f32, [u32; 1]>(ratio)),
                    );
                    let count = debtris.triangles_count;
                    let buffers: ArrayVec<[_; 1]> = [(&debtris.triangles_buffer, 0)].into();
                    enc.bind_vertex_buffers(0, buffers);
                    enc.draw(0..(count * 3) as u32, 0..1);
                }
            }
            buffer.finish();
        }

        let command_buffers = &s.command_buffers[s.current_frame];
        let wait_semaphores: ArrayVec<[_; 1]> = [(
            &s.acquire_image_semaphores[swap_image as usize],
            PipelineStage::COLOR_ATTACHMENT_OUTPUT,
        )]
        .into();
        {
            let present_wait_semaphore = &s.present_wait_semaphores[s.current_frame];
            let signal_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
            let submission = Submission {
                command_buffers: once(command_buffers),
                wait_semaphores,
                signal_semaphores,
            };
            s.queue_group.queues[0].submit(
                submission,
                Some(&s.frames_in_flight_fences[s.current_frame]),
            );
        }
        let postproc_res = postproc(s, swap_image);
        let present_wait_semaphore = &s.present_wait_semaphores[s.current_frame];
        let present_wait_semaphores: ArrayVec<[_; 1]> = [present_wait_semaphore].into();
        s.swapchain
            .present(
                &mut s.queue_group.queues[0],
                swap_image,
                present_wait_semaphores,
            )
            .unwrap();
        postproc_res
    };
    s.current_frame = (s.current_frame + 1) % s.max_frames_in_flight;
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
            let buffers: ArrayVec<[_; 1]> = [(&pt_buffer, 0)].into();
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
    use rand::Rng;
    use rand_pcg::Pcg64Mcg as random;
    use std::f32::consts::PI;

    static LOGO: &[u8] = include_bytes!["../../assets/images/logo.png"];
    static FOREST: &[u8] = include_bytes!["../../assets/images/forest-light.png"];
    static TREE: &[u8] = include_bytes!["../../assets/images/treetest.png"];

    // ---

    fn add_windmills(windowing: &mut Windowing, rand_rotat: bool) -> Vec<DebugTriangleHandle> {
        let mut rng = random::new(0);
        let mut debtris = Vec::with_capacity(1000);
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
            debtris.push(debtri::push(windowing, tri));
        }
        debtris
    }

    fn remove_windmills(windowing: &mut Windowing) {
        debtri::pop_many(windowing, 1000);
    }

    fn add_4_screencorners(windowing: &mut Windowing) {
        debtri::push(
            windowing,
            DebugTriangle::from([-1.0f32, -1.0, 0.0, -1.0, -1.0, 0.0]),
        );
        debtri::push(
            windowing,
            DebugTriangle::from([-1.0f32, 1.0, 0.0, 1.0, -1.0, 0.0]),
        );
        debtri::push(
            windowing,
            DebugTriangle::from([1.0f32, -1.0, 0.0, -1.0, 1.0, 0.0]),
        );
        debtri::push(
            windowing,
            DebugTriangle::from([1.0f32, 1.0, 0.0, 1.0, 1.0, 0.0]),
        );
    }

    pub fn assert_swapchain_eq(windowing: &mut Windowing, name: &str, rgb: Vec<u8>) {
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

        let correct = match std::fs::File::open(&correctname) {
            Ok(x) => x,
            Err(err) => {
                std::process::Command::new("feh")
                    .args(&[genname])
                    .output()
                    .expect("Failed to execute process");
                panic![err]
            }
        };

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
        let mut logger = Logger::spawn_void();
        logger.set_colorize(true);
        logger.set_log_level(64);

        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

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
        let prspect = gen_perspective(&windowing);
        let tri = DebugTriangle::default();

        debtri::push(&mut windowing, tri);
        add_4_screencorners(&mut windowing);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle", img);
    }

    #[test]
    fn simple_triangle_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);
        let tri = DebugTriangle::default();

        let idx = debtri::push(&mut windowing, tri);
        debtri::set_color(&mut windowing, &idx, [255, 0, 255, 255]);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "simple_triangle_change_color", img);
    }

    #[test]
    fn debug_triangle_corners_widescreen() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&windowing);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = debtri::push(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "debug_triangle_corners_widescreen", img);
    }

    #[test]
    fn debug_triangle_corners_tallscreen() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1x2k);
        let prspect = gen_perspective(&windowing);

        for i in [-1f32, 1f32].iter() {
            for j in [-1f32, 1f32].iter() {
                let mut tri = DebugTriangle::default();
                tri.translation = (*i, *j);
                let _idx = debtri::push(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "debug_triangle_corners_tallscreen", img);
    }

    #[test]
    fn circle_of_triangles() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&windowing);

        for i in 0..360 {
            let mut tri = DebugTriangle::default();
            tri.translation = ((i as f32).cos(), (i as f32).sin());
            tri.scale = 0.1f32;
            let _idx = debtri::push(&mut windowing, tri);
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "circle_of_triangles", img);
    }

    #[test]
    fn triangle_in_corner() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let mut tri = DebugTriangle::default();
        tri.scale = 0.1f32;
        let radi = tri.radius();

        let trans = -1f32 + radi;
        for j in 0..31 {
            for i in 0..31 {
                tri.translation = (trans + i as f32 * 2.0 * radi, trans + j as f32 * 2.0 * radi);
                debtri::push(&mut windowing, tri);
            }
        }

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "triangle_in_corner", img);
    }

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
        assert_swapchain_eq(&mut windowing, "overlapping_dyntex_respect_z_order", img);
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
        let prspect = gen_perspective(&windowing);

        add_windmills(&mut windowing, false);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills", img);
    }

    #[test]
    fn windmills_ignore_perspective() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let prspect = gen_perspective(&windowing);

        add_windmills(&mut windowing, false);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills_ignore_perspective", img);
    }

    #[test]
    fn windmills_change_color() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let handles = add_windmills(&mut windowing, false);
        debtri::set_color(&mut windowing, &handles[0], [255, 0, 0, 255]);
        debtri::set_color(&mut windowing, &handles[249], [0, 255, 0, 255]);
        debtri::set_color(&mut windowing, &handles[499], [0, 0, 255, 255]);
        debtri::set_color(&mut windowing, &handles[999], [0, 0, 0, 255]);

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "windmills_change_color", img);
    }

    #[test]
    fn tearing_test() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let _tri = make_centered_equilateral_triangle();
        debtri::push(&mut windowing, DebugTriangle::default());
        for i in 0..=360 {
            if i % 2 == 0 {
                add_4_screencorners(&mut windowing);
            } else {
                debtri::pop_many(&mut windowing, 4);
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
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        add_sprite(&mut windowing, Sprite::default(), &tex);

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "simple_texture", img);
    }

    #[test]
    fn simple_texture_adheres_to_view() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        add_sprite(&mut windowing, Sprite::default(), &tex);

        let prspect = gen_perspective(&windowing);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "simple_texture_adheres_to_view", img);
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
        assert_swapchain_eq(&mut windowing, "colored_simple_texture", img);
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
        assert_swapchain_eq(&mut windowing, "translated_texture", img);
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
        assert_swapchain_eq(&mut windowing, "rotated_texture", img);
    }

    #[test]
    fn correct_perspective() {
        let mut logger = Logger::spawn_void();
        {
            let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
            assert_eq![Matrix4::identity(), gen_perspective(&windowing)];
        }
        {
            let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1x2k);
            assert_eq![
                Matrix4::from_nonuniform_scale(1.0, 0.5, 1.0),
                gen_perspective(&windowing)
            ];
        }
        {
            let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless2x1k);
            assert_eq![
                Matrix4::from_nonuniform_scale(0.5, 1.0, 1.0),
                gen_perspective(&windowing)
            ];
        }
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
        assert_swapchain_eq(&mut windowing, "many_sprites", img);
    }

    #[test]
    fn rotating_windmills_drawing_invariant() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        add_windmills(&mut windowing, false);
        for _ in 0..30 {
            debtri::rotate_all(&mut windowing, Deg(-1.0f32));
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);

        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
        remove_windmills(&mut windowing);

        add_windmills(&mut windowing, false);
        for _ in 0..30 {
            debtri::rotate_all(&mut windowing, Deg(-1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        }
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "rotating_windmills_drawing_invariant", img);
    }

    #[test]
    fn windmills_given_initial_rotation() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        add_windmills(&mut windowing, true);
        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "windmills_given_initial_rotation", img);
    }

    #[test]
    fn streaming_texture_blocks() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let id = add_streaming_texture(&mut windowing, 1000, 1000, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (0, 0),
            (500, 500),
            (255, 0, 0, 255),
        );
        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (500, 0),
            (500, 500),
            (0, 255, 0, 255),
        );
        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (0, 500),
            (500, 500),
            (0, 0, 255, 255),
        );
        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (500, 500),
            (500, 500),
            (0, 0, 0, 0),
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "streaming_texture_blocks", img);
    }

    #[test]
    fn streaming_texture_blocks_off_by_one() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let id = add_streaming_texture(&mut windowing, 10, 1, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (0, 0),
            (10, 1),
            (0, 255, 0, 255),
        );

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (3, 0),
            (1, 1),
            (0, 0, 255, 255),
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "streaming_texture_blocks_off_by_one", img);

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (3, 0),
            (0, 1),
            (255, 0, 255, 255),
        );

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (3, 0),
            (0, 0),
            (255, 0, 255, 255),
        );

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (3, 0),
            (1, 0),
            (255, 0, 255, 255),
        );

        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            id,
            (30, 0),
            (800, 0),
            (255, 0, 255, 255),
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "streaming_texture_blocks_off_by_one", img);
    }

    #[test]
    fn streaming_texture_weird_pixel_accesses() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let id = add_streaming_texture(&mut windowing, 20, 20, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        let mut rng = random::new(0);

        for _ in 0..1000 {
            let x = rng.gen_range(0, 30);
            let y = rng.gen_range(0, 30);

            strtex::streaming_texture_set_pixel(&mut windowing, id, x, y, (0, 255, 0, 255));
        }
    }

    #[test]
    fn streaming_texture_weird_block_accesses() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let id = add_streaming_texture(&mut windowing, 64, 64, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        let mut rng = random::new(0);

        for _ in 0..1000 {
            let start = (rng.gen_range(0, 100), rng.gen_range(0, 100));
            let wh = (rng.gen_range(0, 100), rng.gen_range(0, 100));

            strtex::streaming_texture_set_pixels_block(
                &mut windowing,
                id,
                start,
                wh,
                (0, 255, 0, 255),
            );
        }
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
        assert_swapchain_eq(&mut windowing, "three_layer_scene", img);
    }

    #[test]
    fn streaming_texture_respects_z_ordering() {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let strtex1 = add_streaming_texture(&mut windowing, 10, 10, &mut logger);
        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            strtex1,
            (0, 0),
            (9, 9),
            (255, 255, 0, 255),
        );
        strtex::streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), strtex1);

        let strtex2 = add_streaming_texture(&mut windowing, 10, 10, &mut logger);
        strtex::streaming_texture_set_pixels_block(
            &mut windowing,
            strtex2,
            (1, 1),
            (9, 9),
            (0, 255, 255, 255),
        );
        strtex::streaming_texture_add_sprite(
            &mut windowing,
            strtex::Sprite {
                depth: 0.1,
                ..strtex::Sprite::default()
            },
            strtex2,
        );

        let img = draw_frame_copy_framebuffer(&mut windowing, &mut logger, &prspect);
        assert_swapchain_eq(&mut windowing, "streaming_texture_z_ordering", img);
    }

    // ---

    #[bench]
    fn bench_many_sprites(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        for i in 0..1000 {
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
        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_many_particles(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let tex = add_texture(&mut windowing, LOGO, TextureOptions::default());
        let mut rng = random::new(0);
        for i in 0..1000 {
            let (dx, dy) = (
                rng.gen_range(-1.0f32, 1.0f32),
                rng.gen_range(-1.0f32, 1.0f32),
            );
            add_sprite(
                &mut windowing,
                Sprite {
                    translation: (dx, dy),
                    rotation: ((i * 10) as f32 / 180f32 * PI),
                    scale: 0.01,
                    ..Sprite::default()
                },
                &tex,
            );
        }

        let prspect = gen_perspective(&windowing);
        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_simple_triangle(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        debtri::push(&mut windowing, DebugTriangle::default());
        add_4_screencorners(&mut windowing);

        b.iter(|| {
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_still_windmills(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

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
            debtri::set_color(&mut windowing, &handles[0], black_box([0, 0, 0, 255]));
        });
    }

    #[bench]
    fn bench_rotating_windmills(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        add_windmills(&mut windowing, false);

        b.iter(|| {
            debtri::rotate_all(&mut windowing, Deg(1.0f32));
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }

    #[bench]
    fn bench_rotating_windmills_no_render(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        add_windmills(&mut windowing, false);

        b.iter(|| {
            debtri::rotate_all(&mut windowing, Deg(1.0f32));
        });
    }

    #[bench]
    fn clears_per_second(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

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

    #[bench]
    fn bench_streaming_texture_set_500x500_area(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let id = add_streaming_texture(&mut windowing, 1000, 1000, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        b.iter(|| {
            strtex::streaming_texture_set_pixels_block(
                &mut windowing,
                id,
                (0, 0),
                (500, 500),
                (255, 0, 0, 255),
            );
        });
    }

    #[bench]
    fn bench_streaming_texture_set_single_pixel(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);

        let id = add_streaming_texture(&mut windowing, 1000, 1000, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        b.iter(|| {
            strtex::streaming_texture_set_pixel(
                &mut windowing,
                id,
                black_box(1),
                black_box(2),
                (255, 0, 0, 255),
            );
        });
    }

    #[bench]
    fn bench_streaming_texture_set_single_pixel_while_drawing(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger, ShowWindow::Headless1k);
        let prspect = gen_perspective(&windowing);

        let id = add_streaming_texture(&mut windowing, 50, 50, &mut logger);
        streaming_texture_add_sprite(&mut windowing, strtex::Sprite::default(), id);

        b.iter(|| {
            strtex::streaming_texture_set_pixel(
                &mut windowing,
                id,
                black_box(1),
                black_box(2),
                (255, 0, 0, 255),
            );
            draw_frame(&mut windowing, &mut logger, &prspect);
        });
    }
}
