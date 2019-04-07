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
    command::{self, ClearColor, ClearValue},
    device::Device,
    format::{self, ChannelType, Swizzle},
    image, pass, pool,
    pso::{PipelineStage, Rect},
    queue::Submission,
    window::{Extent2D, PresentMode::*, Surface, Swapchain},
    Backbuffer, Backend, FrameSync, Instance, SwapchainConfig,
};
use logger::{info, InDebug, InDebugPretty, Logger};
use std::mem::ManuallyDrop;
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

    let mut frame_fences = vec![];
    let mut frame_render_fences = vec![];
    let mut acquire_image_semaphores = vec![];
    let mut present_wait_semaphores = vec![];
    for _ in 0..image_count {
        frame_fences.push(device.create_fence(true).expect("Can't create fence"));
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

    Windowing {
        adapter,
        command_buffers,
        command_pool: ManuallyDrop::new(command_pool),
        current_frame: 0,
        device: ManuallyDrop::new(device),
        events_loop,
        frame_fences,
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
        render_pass: ManuallyDrop::new(render_pass),
        surf,
        swapchain: ManuallyDrop::new(swapchain),
        vk_inst: ManuallyDrop::new(vk_inst),
        window,
    }
}

pub fn collect_input(windowing: &mut Windowing) -> Vec<Event> {
    let mut inputs = vec![];
    windowing.events_loop.poll_events(|evt| {
        inputs.push(evt);
    });
    inputs
}

pub fn draw_frame(s: &mut Windowing, log: &mut Logger<Log>) {
    let frame_fence = &s.frame_fences[s.current_frame];
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
            .wait_for_fence(frame_fence, u64::max_value())
            .unwrap();
        info![log, "vxdraw", "Resetting fence"];
        s.device.reset_fence(frame_fence).unwrap();

        {
            let buffer = &mut s.command_buffers[s.current_frame];
            let clear_values = [ClearValue::Color(ClearColor::Float([
                1.0f32, 0.0, 0.0, 1.0,
            ]))];
            buffer.begin(false);
            buffer.begin_render_pass_inline(
                &s.render_pass,
                &s.framebuffers[s.current_frame],
                s.render_area,
                clear_values.iter(),
            );
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
        the_command_queue.submit(submission, Some(frame_fence));
        s.swapchain
            .present(the_command_queue, image_index, present_wait_semaphores)
            .unwrap();
    }
    s.current_frame = (s.current_frame + 1) % s.image_count;
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[cfg(feature = "gfx_tests")]
    #[test]
    fn init_window_and_get_input() {
        let mut logger = Logger::spawn();
        logger.set_colorize(true);
        let mut windowing = init_window_with_vulkan(&mut logger);
        collect_input(&mut windowing);
        for _ in 0..300 {
            draw_frame(&mut windowing, &mut logger);
            std::thread::sleep(std::time::Duration::new(0, 8_000_000));
        }
    }

    #[cfg(feature = "gfx_tests")]
    #[bench]
    fn clears_per_second(b: &mut Bencher) {
        let mut logger = Logger::spawn_void();
        let mut windowing = init_window_with_vulkan(&mut logger);
        b.iter(|| {
            draw_frame(&mut windowing, &mut logger);
        });
    }
}
