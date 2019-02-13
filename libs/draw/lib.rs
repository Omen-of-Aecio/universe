#![cfg_attr(
    not(any(
        feature = "vulkan",
        feature = "dx12",
        feature = "metal",
        feature = "gl"
    )),
    allow(dead_code, unused_extern_crates, unused_imports)
)]

extern crate env_logger;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
extern crate gfx_backend_gl as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;

extern crate glsl_to_spirv;
extern crate image;
extern crate winit;

use hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use hal::pass::Subpass;
use hal::pso::{PipelineStage, ShaderStageFlags};
use hal::queue::Submission;
use hal::{
    buffer, command, format as f, image as i, memory as m, pass, pool, pso, window::Extent2D,
};
use hal::{Backbuffer, Backend, DescriptorPool, FrameSync, Primitive, SwapchainConfig};
use hal::{Device, Instance, PhysicalDevice, Surface, Swapchain};

use std::fs;
use std::io::{Cursor, Read};

#[cfg_attr(rustfmt, rustfmt_skip)]
const DIMS: Extent2D = Extent2D { width: 1024, height: 768 };

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0..1,
    layers: 0..1,
};

pub struct Draw {
    command_pool: hal::CommandPool<back::Backend, hal::Graphics>,
    device: back::Device,
    format: hal::format::Format,
    frame_fence: <back::Backend as Backend>::Fence,
    frame_semaphore: <back::Backend as Backend>::Semaphore,
    framebuffers: Vec<u32>,
    queue_group: hal::QueueGroup<back::Backend, hal::Graphics>,
    swap_chain: <back::Backend as Backend>::Swapchain,
    viewport: pso::Viewport,
}

impl Draw {
    pub fn new(surface: &mut back::Surface) -> Self {
        // Step 1: Find devices on machine
        let mut adapters = surface.enumerate_adapters();
        for adapter in &adapters {
            println!("Adapter: {:?}", adapter.info);
        }
        let mut adapter = adapters.remove(0);
        // let memory_types = adapter.physical_device.memory_properties().memory_types;
        // let limits = adapter.physical_device.limits();
        // Step 2: Open device supporting Graphics
        let (device, queue_group) = adapter
            .open_with::<_, hal::Graphics>(1, |family| {
                surface.supports_queue_family(family)
            })
            .expect("Unable to find device supporting graphics");
        // Step 3: Create command pool
        let command_pool = unsafe {
            device.create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty())
        }
        .expect("Can't create command pool");
        // Step 4: Create semaphore
        let frame_semaphore = device.create_semaphore().expect("Can't create semaphore");
        let frame_fence = device.create_fence(false).expect("Can't create fence");
        // Step 5: Set up swapchain
        let (caps, formats, present_modes, _composite_alpha) =
            surface.compatibility(&mut adapter.physical_device);
        let format = formats.map_or(f::Format::Rgba8Srgb, |formats| {
            formats
                .iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .map(|format| *format)
                .unwrap_or(formats[0])
        });
        let present_mode = {
            use gfx_hal::window::PresentMode::*;
            [Mailbox, Fifo, Relaxed, Immediate]
              .iter()
              .cloned()
              .find(|pm| present_modes.contains(pm))
              .ok_or("No PresentMode values specified!")
              .unwrap()
        };
        println!["{:?}", present_modes];
        println!["{:?}", present_mode];
        println!["{:?}", caps];

        let swap_config = SwapchainConfig::from_caps(&caps, format, DIMS);
        println!("{:?}", swap_config);
        let extent = swap_config.extent.to_extent();

        let (swap_chain, backbuffer) =
            unsafe { device.create_swapchain(surface, swap_config, None) }
                .expect("Can't create swapchain");
        // Step 6: Create render pass
        let render_pass = {
            let attachment = pass::Attachment {
                format: Some(format),
                samples: 1,
                ops: pass::AttachmentOps::new(
                    pass::AttachmentLoadOp::Load,
                    pass::AttachmentStoreOp::Store,
                ),
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::Layout::Undefined..i::Layout::Present,
            };

            let subpass = pass::SubpassDesc {
                colors: &[(0, i::Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };

            let dependency = pass::SubpassDependency {
                passes: pass::SubpassRef::External..pass::SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT
                    ..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: i::Access::empty()
                    ..(i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            };

            unsafe { device.create_render_pass(&[attachment], &[subpass], &[dependency]) }
                .expect("Can't create render pass")
        };
        // Step 7: Collect framebuffers
        let (frame_images, framebuffers) = match backbuffer {
            Backbuffer::Images(images) => {
                println!["Image backbuffer"];
                let pairs = images
                    .into_iter()
                    .map(|image| unsafe {
                        let rtv = device
                            .create_image_view(
                                &image,
                                i::ViewKind::D2,
                                format,
                                Swizzle::NO,
                                COLOR_RANGE.clone(),
                            )
                            .unwrap();
                        (image, rtv)
                    })
                    .collect::<Vec<_>>();
                let fbos = pairs
                    .iter()
                    .map(|&(_, ref rtv)| unsafe {
                        device
                            .create_framebuffer(&render_pass, Some(rtv), extent)
                            .unwrap()
                    })
                    .collect();
                (pairs, fbos)
            }
            Backbuffer::Framebuffer(fbo) => {
                println!["Framebuffer backbuffer"];
                (Vec::new(), vec![fbo])
            }
        };
        // Step 8: Set up a viewport
        let viewport = pso::Viewport {
            rect: pso::Rect {
                x: 0,
                y: 0,
                w: extent.width as _,
                h: extent.height as _,
            },
            depth: 0.0..1.0,
        };

        Self {
            command_pool,
            device,
            format,
            frame_fence,
            frame_semaphore,
            framebuffers,
            queue_group,
            swap_chain,
            viewport,
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Option<hal::SwapImageIndex> {
        unsafe {
            self.device.reset_fence(&self.frame_fence).unwrap();
            self.command_pool.reset();
            match self
                .swap_chain
                .acquire_image(u64::max_value(), FrameSync::Semaphore(&mut self.frame_semaphore))
            {
                Ok(i) => Some(i),
                Err(_) => None,
            }
        }
    }

    pub fn swap_it(&mut self, frame: hal::SwapImageIndex) {
        unsafe {
            self.device.wait_for_fence(&self.frame_fence, u64::max_value()).unwrap();
            if let Err(_) = self
                .swap_chain
                .present_nosemaphores(&mut self.queue_group.queues[0], frame)
            {
                // self.recreate_swapchain = true;
            }
        }
    }

    pub fn clear(&mut self, frame: hal::SwapImageIndex, r: f32) {
        let render_pass = {
            let color_attachment = pass::Attachment {
                format: Some(self.format),
                samples: 1,
                ops: pass::AttachmentOps {
                    load: pass::AttachmentLoadOp::Clear,
                    store: pass::AttachmentStoreOp::Store,
                },
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::Layout::Undefined..i::Layout::Present,
            };
            let subpass = pass::SubpassDesc {
                colors: &[(0, i::Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };
            unsafe {
                self.device
                    .create_render_pass(&[color_attachment], &[subpass], &[])
                    .map_err(|_| "Couldn't create a render pass!")
                    .unwrap()
            }
        };
        let mut cmd_buffer = self
            .command_pool
            .acquire_command_buffer::<command::OneShot>();
        unsafe {
            cmd_buffer.begin();

            cmd_buffer.set_viewports(0, &[self.viewport.clone()]);
            cmd_buffer.set_scissors(0, &[self.viewport.rect]);
            // cmd_buffer.bind_graphics_pipeline(&self.pipeline);
            // cmd_buffer.bind_vertex_buffers(0, Some((&self.vertex_buffer, 0)));
            // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

            cmd_buffer.begin_render_pass_inline(
                &render_pass,
                &self.framebuffers[frame as usize],
                self.viewport.rect,
                &[command::ClearValue::Color(command::ClearColor::Float([
                    r, 0.8, 0.8, 1.0,
                ]))],
            );

            cmd_buffer.finish();

            let submission = Submission {
                command_buffers: Some(&cmd_buffer),
                wait_semaphores: Some((&self.frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)),
                signal_semaphores: &[],
            };
            // self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence));
            self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence));
        }
    }
}

// impl std::ops::Drop for Draw {
//     fn drop(&mut self) {
//         self.device.wait_idle().unwrap();
//         unsafe {
//             self.device.destroy_command_pool(self.command_pool.into_raw());
//             self.device.destroy_descriptor_pool(self.desc_pool);
//             self.device.destroy_descriptor_set_layout(self.set_layout);

//             self.device.destroy_buffer(self.vertex_buffer);
//             self.device.destroy_buffer(self.image_upload_buffer);
//             self.device.destroy_image(self.image_logo);
//             self.device.destroy_image_view(self.image_srv);
//             self.device.destroy_sampler(self.sampler);
//             self.device.destroy_fence(self.frame_fence);
//             self.device.destroy_semaphore(self.frame_semaphore);
//             self.device.destroy_render_pass(self.render_pass);
//             self.device.free_memory(self.buffer_memory);
//             self.device.free_memory(self.image_memory);
//             self.device.free_memory(self.mage_upload_memory);
//             self.device.destroy_graphics_pipeline(self.pipeline);
//             self.device.destroy_pipeline_layout(self.pipeline_layout);
//             for framebuffer in self.framebuffers {
//                 self.device.destroy_framebuffer(selfframebuffer);
//             }
//             for (_, rtv) in self.frame_images {
//                 self.device.destroy_image_view(rtv);
//             }

//             self.device.destroy_swapchain(self.swap_chain);
//         }
//     }
// }
