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
use gfx_hal::command::{CommandBuffer, MultiShot, Primary};

use std::fs;
use std::io::{Cursor, Read};

#[cfg_attr(rustfmt, rustfmt_skip)]
const DIMS: Extent2D = Extent2D { width: 1024, height: 768 };

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0..1,
    layers: 0..1,
};

pub struct LinearSurface<'a> {
    pub frame: hal::SwapImageIndex,
    framebuffer: &'a mut <back::Backend as Backend>::Framebuffer,
    queue_group: &'a mut hal::QueueGroup<back::Backend, hal::Graphics>,
    viewport: &'a pso::Viewport,
}

pub struct StaticWhite2DTriangle {
    buffer: <back::Backend as Backend>::Buffer,
    cmd_buffer: CommandBuffer<back::Backend, hal::Graphics, MultiShot, Primary>,
    memory: <back::Backend as Backend>::Memory,
    pipeline: <back::Backend as Backend>::GraphicsPipeline,
    render_pass: <back::Backend as Backend>::RenderPass,
    signal_semaphore: Vec<<back::Backend as Backend>::Semaphore>,
}

impl StaticWhite2DTriangle {
    pub fn draw(&mut self, surface: &mut LinearSurface) {
        unsafe {
            self.cmd_buffer.begin(false);

            // let mut x = draw.viewport.clone();
            // self.cmd_buffer.set_viewports(0, &[x]);
            // self.cmd_buffer.set_scissors(0, &[draw.viewport.rect]);
            self.cmd_buffer.bind_graphics_pipeline(&self.pipeline);
            self.cmd_buffer.bind_vertex_buffers(0, Some((&self.buffer, 0)));
            // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

            {
                let mut encoder = self.cmd_buffer.begin_render_pass_inline(
                    &self.render_pass,
                    surface.framebuffer,
                    surface.viewport.rect,
                    &[],
                );
                encoder.draw(0..3, 0..1);
            }

            self.cmd_buffer.finish();

            surface.queue_group.queues[0].submit_nosemaphores(std::iter::once(&self.cmd_buffer), None);
        }
    }

    pub fn draw2(&mut self, draw: &mut Draw, surface: &<back::Backend as Backend>::Framebuffer) {
        unsafe {
            self.cmd_buffer.begin(false);

            // cmd_buffer.set_viewports(0, &[draw.viewport.clone()]);
            self.cmd_buffer.set_scissors(0, &[draw.viewport.rect]);
            self.cmd_buffer.bind_graphics_pipeline(&self.pipeline);
            self.cmd_buffer.bind_vertex_buffers(0, Some((&self.buffer, 0)));
            // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

            {
                let mut encoder = self.cmd_buffer.begin_render_pass_inline(
                    &self.render_pass,
                    surface,
                    draw.viewport.rect,
                    &[],
                );
                encoder.draw(0..3, 0..1);
            }

            self.cmd_buffer.finish();

            draw.queue_group.queues[0].submit_nosemaphores(std::iter::once(&self.cmd_buffer), None);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
  pub points: [[f32; 2]; 3],
}
impl Triangle {
  pub fn points_flat(self) -> [f32; 6] {
    let [[a, b], [c, d], [e, f]] = self.points;
    [a, b, c, d, e, f]
  }
}


pub struct Draw {
    adapter: hal::Adapter<back::Backend>,
    command_pool: hal::CommandPool<back::Backend, hal::Graphics>,
    device: back::Device,
    format: hal::format::Format,
    frame_fence: Vec<<back::Backend as Backend>::Fence>,
    frame_index: usize,
    frame_semaphore: Vec<<back::Backend as Backend>::Semaphore>,
    framebuffers: Vec<<back::Backend as Backend>::Framebuffer>,
    image_count: usize,
    queue_group: hal::QueueGroup<back::Backend, hal::Graphics>,
    render_finished_semaphore: Vec<<back::Backend as Backend>::Semaphore>,
    swap_chain: <back::Backend as Backend>::Swapchain,
    viewport: pso::Viewport,
}

impl Draw {
    pub fn get_linear_surface(&mut self) -> LinearSurface {
        let image = self.acquire_swapchain_image().unwrap();
        self.clear(image, 0.3);
        LinearSurface {
            frame: image,
            framebuffer: &mut self.framebuffers[image as usize],
            queue_group: &mut self.queue_group,
            viewport: &self.viewport,
        }
    }

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
        // Step 4: Set up swapchain
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

        use gfx_hal::window::PresentMode::*;
        let image_count = if present_mode == Mailbox {
            (caps.image_count.end - 1).min(3) as usize
        } else {
            (caps.image_count.end - 1).min(2) as usize
        };

        let swap_config = SwapchainConfig::from_caps(&caps, format, DIMS);
        println!("{:?}", swap_config);
        let extent = swap_config.extent.to_extent();

        let (swap_chain, backbuffer) =
            unsafe { device.create_swapchain(surface, swap_config, None) }
                .expect("Can't create swapchain");
        // Step 5: Create render pass
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
        // Step 6: Collect framebuffers
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

        // Step 7: Set up a viewport
        let viewport = pso::Viewport {
            rect: pso::Rect {
                x: 0,
                y: 0,
                w: extent.width as _,
                h: extent.height as _,
            },
            depth: 0.0..1.0,
        };

        // Step 8: Set up fences and semaphores
        let mut frame_fence = Vec::with_capacity(image_count);
        let mut frame_semaphore = Vec::with_capacity(image_count);
        let mut render_finished_semaphore = Vec::with_capacity(image_count);
        for i in 0..image_count {
            frame_fence.push(device.create_fence(false).expect("Can't create fence"));
            frame_semaphore.push(device.create_semaphore().expect("Can't create semaphore"));
            render_finished_semaphore.push(device.create_semaphore().expect("Can't create semaphore"));
        }

        Self {
            adapter,
            command_pool,
            device,
            format,
            frame_fence,
            frame_index: 0,
            frame_semaphore,
            framebuffers,
            image_count,
            queue_group,
            render_finished_semaphore,
            swap_chain,
            viewport,
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Option<hal::SwapImageIndex> {
        unsafe {
            // self.device.reset_fence(&self.frame_fence).unwrap();
            self.command_pool.reset();
            match self
                .swap_chain
                .acquire_image(u64::max_value(), FrameSync::Semaphore(&mut self.frame_semaphore[self.frame_index]))
            {
                Ok(i) => {
                    self.frame_index = (self.frame_index + 1) % self.image_count;
                    Some(i)
                }
                Err(_) => None,
            }
        }
    }

    pub fn swap_it(&mut self, frame: hal::SwapImageIndex) {
        unsafe {
            self.device.wait_for_fence(&self.frame_fence[frame as usize], u64::max_value()).unwrap();
            if let Err(_) = self
                .swap_chain
                .present_nosemaphores(&mut self.queue_group.queues[0], frame)
            {
                // self.recreate_swapchain = true;
            }
        }
    }

    pub fn create_static_white_2d_triangle(&mut self, triangle: &[f32; 6]) -> StaticWhite2DTriangle {
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

        // Create a buffer for the vertex data (this is rather involved)
        let (buffer, memory, requirements) = unsafe {
            const F32_XY_TRIANGLE: u64 = (std::mem::size_of::<f32>() * 2 * 3) as u64;
            use gfx_hal::{adapter::MemoryTypeId, memory::Properties};
            let mut buffer = self.device.create_buffer(F32_XY_TRIANGLE, gfx_hal::buffer::Usage::VERTEX).expect("cant make bf");
            let requirements = self.device.get_buffer_requirements(&buffer);
            let memory_type_id = self.adapter
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
            let memory = self.device
              .allocate_memory(memory_type_id, requirements.size)
              .expect("Couldn't allocate vertex buffer memory");
            println!["{:?}", memory];
            self.device
              .bind_buffer_memory(&memory, 0, &mut buffer)
              .expect("Couldn't bind the buffer memory!");
            // (buffer, memory, requirements)
            (buffer, memory, requirements)
        };

        // Upload vertex data
        unsafe {
            let mut data_target = self
              .device
              .acquire_mapping_writer(&memory, 0..requirements.size)
              .expect("Failed to acquire a memory writer!");
            let points = triangle;
            println!["Uploading points: {:?}", points];
            data_target[..points.len()].copy_from_slice(points);
            self
              .device
              .release_mapping_writer(data_target)
              .expect("Couldn't release the mapping writer!");
        }

        // Compile shader modules
        let vs_module = {
            let glsl = VERTEX_SOURCE;
            let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Vertex)
                .unwrap()
                .bytes()
                .map(|b| b.unwrap())
                .collect();
            unsafe { self.device.create_shader_module(&spirv) }.unwrap()
        };
        let fs_module = {
            let glsl = FRAGMENT_SOURCE;
            let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, glsl_to_spirv::ShaderType::Fragment)
                .unwrap()
                .bytes()
                .map(|b| b.unwrap())
                .collect();
            unsafe { self.device.create_shader_module(&spirv) }.unwrap()
        };

        // Describe the shaders
        const ENTRY_NAME: &str = "main";
        let vs_module: <back::Backend as Backend>::ShaderModule = vs_module;
        use hal::pso;
        let (vs_entry, fs_entry) = (
            pso::EntryPoint {
                entry: ENTRY_NAME,
                module: &vs_module,
                // specialization: pso::Specialization {
                //     constants: &[pso::SpecializationConstant { id: 0, range: 0..4 }],
                //     data: unsafe { std::mem::transmute::<&f32, &[u8; 4]>(&0.8f32) },
                // },
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
        let render_pass = {
            let attachment = pass::Attachment {
                format: Some(self.format),
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

            // let dependency = pass::SubpassDependency {
            //     passes: pass::SubpassRef::External..pass::SubpassRef::Pass(0),
            //     stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT
            //         ..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            //     accesses: i::Access::empty()
            //         ..(i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            // };

            unsafe { self.device.create_render_pass(&[attachment], &[subpass], &[]) }
                .expect("Can't create render pass")
        };

        let subpass = Subpass {
            index: 0,
            main_pass: &render_pass,
        };

        // Create a descriptor set layout (this is mainly for textures), we just create an empty
        // one
        // let bindings = Vec::<pso::DescriptorSetLayoutBinding>::new();
        // let immutable_samplers = Vec::<<back::Backend as Backend>::Sampler>::new();
        // let set_layout = unsafe {
        //     self.device.create_descriptor_set_layout(bindings, immutable_samplers)
        // };

        // Create a pipeline layout
        let pipeline_layout = unsafe {
            self.device.create_pipeline_layout(
                // &set_layout,
                &[], // No descriptor set layout (no texture/sampler)
                &[], // &[(pso::ShaderStageFlags::VERTEX, 0..4)],
            )
        }
        .expect("Cant create pipelinelayout");

        // Describe the pipeline (rasterization, triangle interpretation)
        let mut pipeline_desc = pso::GraphicsPipelineDesc::new(
            shader_entries,
            Primitive::TriangleList,
            pso::Rasterizer::FILL,
            &pipeline_layout,
            subpass,
        );

        pipeline_desc.vertex_buffers.push(pso::VertexBufferDesc {
            binding: 0,
            stride: 8 as u32,
            rate: 0, // VertexInputRate::Vertex,
        });

        pipeline_desc.blender.targets.push(pso::ColorBlendDesc(
            pso::ColorMask::ALL,
            pso::BlendState::ALPHA,
        ));

        pipeline_desc.attributes.push(pso::AttributeDesc {
            location: 0,
            binding: 0,
            element: pso::Element {
                format: f::Format::Rg32Float,
                offset: 0,
            },
        });

        let pipeline = unsafe {
          self.device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Couldn't create a graphics pipeline!")
        };

        unsafe {
            self.device.destroy_shader_module(vs_module);
        }
        unsafe {
            self.device.destroy_shader_module(fs_module);
        }

        let mut cmd_buffer = self
            .command_pool
            .acquire_command_buffer::<command::MultiShot>();

        // unsafe {
        //     cmd_buffer.begin();

        //     cmd_buffer.set_viewports(0, &[self.viewport.clone()]);
        //     cmd_buffer.set_scissors(0, &[self.viewport.rect]);
        //     cmd_buffer.bind_graphics_pipeline(&pipeline);
        //     cmd_buffer.bind_vertex_buffers(0, Some((&buffer, 0)));
        //     // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

        //     {
        //         let mut encoder = cmd_buffer.begin_render_pass_inline(
        //             &render_pass,
        //             &self.framebuffers[frame as usize],
        //             self.viewport.rect,
        //             &[],
        //         );
        //         encoder.draw(0..3, 0..1);
        //     }

        //     cmd_buffer.finish();

        //     let submission = Submission {
        //         command_buffers: Some(&cmd_buffer),
        //         wait_semaphores: Some((&self.frame_semaphore[self.frame_index], PipelineStage::COLOR_ATTACHMENT_OUTPUT)),
        //         signal_semaphores: Some(&self.render_finished_semaphore[self.frame_index]),
        //     };
        //     self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence[self.frame_index]));
        //     self.device.wait_for_fence(&self.frame_fence[self.frame_index], u64::max_value()).unwrap();
        //     self.device.reset_fence(&self.frame_fence[self.frame_index]).expect("Unable to reset fence");
        // }
        let signal_semaphore = vec![self.device.create_semaphore().expect("c sema")];
        StaticWhite2DTriangle {
            buffer,
            cmd_buffer,
            memory,
            pipeline,
            render_pass,
            signal_semaphore,
        }
    }

    fn clear(&mut self, frame: hal::SwapImageIndex, r: f32) {
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
                wait_semaphores: Some((&self.frame_semaphore[self.frame_index], PipelineStage::BOTTOM_OF_PIPE)),
                signal_semaphores: None, // Some(&self.render_finished_semaphore[self.frame_index]),
            };
            // self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence));
            // self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence[self.frame_index]));
            self.device.wait_for_fence(&self.frame_fence[self.frame_index], u64::max_value()).expect("Unable to wait on fence");
            self.device.reset_fence(&self.frame_fence[self.frame_index]).expect("Unable to reset fence");
            self.queue_group.queues[0].submit(submission, Some(&mut self.frame_fence[self.frame_index]));
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
