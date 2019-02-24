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

pub trait Canvas {
    fn get_framebuffer(&mut self) -> &mut <back::Backend as Backend>::Framebuffer;
    fn get_queue_group(&mut self) -> &mut hal::QueueGroup<back::Backend, hal::Graphics>;
    fn get_viewport(&mut self) -> &pso::Viewport;
}

pub struct ScreenCanvas<'a> {
    pub frame: hal::SwapImageIndex,
    framebuffer: &'a mut <back::Backend as Backend>::Framebuffer,
    queue_group: &'a mut hal::QueueGroup<back::Backend, hal::Graphics>,
    viewport: &'a pso::Viewport,
}

impl<'a> Canvas for ScreenCanvas<'a> {
    fn get_framebuffer(&mut self) -> &mut <back::Backend as Backend>::Framebuffer {
        self.framebuffer
    }
    fn get_queue_group(&mut self) -> &mut hal::QueueGroup<back::Backend, hal::Graphics> {
        self.queue_group
    }
    fn get_viewport(&mut self) -> &pso::Viewport {
        self.viewport
    }
}

pub struct StaticTexture2DRectangle {
    buffer: <back::Backend as Backend>::Buffer,
    cmd_buffer: CommandBuffer<back::Backend, hal::Graphics, command::OneShot, Primary>,
    image_upload_buffer: <back::Backend as Backend>::Buffer,
    memory: <back::Backend as Backend>::Memory,
    pipeline: <back::Backend as Backend>::GraphicsPipeline,
    render_pass: <back::Backend as Backend>::RenderPass,
}
impl StaticTexture2DRectangle {
    pub fn draw(&mut self, surface: &mut impl Canvas) {
        unsafe {
            self.cmd_buffer.begin();

            // let mut x = draw.viewport.clone();
            // self.cmd_buffer.set_viewports(0, &[x]);
            // self.cmd_buffer.set_scissors(0, &[draw.viewport.rect]);
            self.cmd_buffer.bind_graphics_pipeline(&self.pipeline);
            self.cmd_buffer.bind_vertex_buffers(0, Some((&self.buffer, 0)));
            // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

            {
                let rect = surface.get_viewport().rect.clone();
                let mut encoder = self.cmd_buffer.begin_render_pass_inline(
                    &self.render_pass,
                    surface.get_framebuffer(),
                    rect,
                    &[],
                );
                encoder.draw(0..6, 0..1);
            }

            self.cmd_buffer.finish();

            surface.get_queue_group().queues[0].submit_nosemaphores(std::iter::once(&self.cmd_buffer), None);
        }
    }
}
pub struct StaticWhite2DTriangle {
    buffer: <back::Backend as Backend>::Buffer,
    cmd_buffer: CommandBuffer<back::Backend, hal::Graphics, MultiShot, Primary>,
    memory: <back::Backend as Backend>::Memory,
    pipeline: <back::Backend as Backend>::GraphicsPipeline,
    render_pass: <back::Backend as Backend>::RenderPass,
}

impl StaticWhite2DTriangle {
    pub fn draw(&mut self, surface: &mut impl Canvas) {
        unsafe {
            self.cmd_buffer.begin(false);

            // let mut x = draw.viewport.clone();
            // self.cmd_buffer.set_viewports(0, &[x]);
            // self.cmd_buffer.set_scissors(0, &[draw.viewport.rect]);
            self.cmd_buffer.bind_graphics_pipeline(&self.pipeline);
            self.cmd_buffer.bind_vertex_buffers(0, Some((&self.buffer, 0)));
            // cmd_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, Some(&self.desc_set), &[]);

            {
                let rect = surface.get_viewport().rect.clone();
                let mut encoder = self.cmd_buffer.begin_render_pass_inline(
                    &self.render_pass,
                    surface.get_framebuffer(),
                    rect,
                    &[],
                );
                encoder.draw(0..3, 0..1);
            }

            self.cmd_buffer.finish();

            surface.get_queue_group().queues[0].submit_nosemaphores(std::iter::once(&self.cmd_buffer), None);
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
    pub fn prepare_canvas(&mut self) -> ScreenCanvas {
        let image = self.acquire_swapchain_image().unwrap();
        self.clear(image, 0.3);
        ScreenCanvas {
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

    fn acquire_swapchain_image(&mut self) -> Option<hal::SwapImageIndex> {
        unsafe {
            // self.command_pool.reset();
            match self
                .swap_chain
                .acquire_image(u64::max_value(), FrameSync::Semaphore(&mut self.frame_semaphore[self.frame_index]))
            {
                Ok(i) => {
                    self.frame_index = (self.frame_index + 1) % self.image_count;
                    self.device.reset_fence(&self.frame_fence[self.frame_index]).unwrap();
                    Some(i)
                }
                Err(_) => None,
            }
        }
    }
    pub fn swap_it(&mut self, frame: hal::SwapImageIndex) {
        unsafe {
            if let Err(_) = self
                .swap_chain
                .present_nosemaphores(&mut self.queue_group.queues[0], frame)
            {
                // self.recreate_swapchain = true;
            }
        }
    }

    pub fn create_static_texture_2d_rectangle(&mut self) -> StaticTexture2DRectangle {
        const VERTEX_SOURCE: &str = "#version 450
        #extension GL_ARB_separate_shader_objects : enable

        layout(constant_id = 0) const float scale = 1.2f;

        layout(location = 0) in vec2 a_pos;
        layout(location = 1) in vec2 a_uv;
        layout(location = 0) out vec2 v_uv;

        out gl_PerVertex {
            vec4 gl_Position;
        };

        void main() {
            v_uv = a_uv;
            gl_Position = vec4(scale * a_pos, 0.0, 1.0);
        }";

        const FRAGMENT_SOURCE: &str = "#version 450
        #extension GL_ARB_separate_shader_objects : enable

        layout(location = 0) in vec2 v_uv;
        layout(location = 0) out vec4 target0;

        layout(set = 0, binding = 0) uniform texture2D u_texture;
        layout(set = 0, binding = 1) uniform sampler u_sampler;

        void main() {
            target0 = texture(sampler2D(u_texture, u_sampler), v_uv);
        }";
        let set_layout = unsafe {
            self.device.create_descriptor_set_layout(
                &[
                    pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: pso::DescriptorType::SampledImage,
                        count: 1,
                        stage_flags: ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    pso::DescriptorSetLayoutBinding {
                        binding: 1,
                        ty: pso::DescriptorType::Sampler,
                        count: 1,
                        stage_flags: ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                ],
                &[],
            )
        }
        .expect("Can't create descriptor set layout");

        // Descriptors
        let mut desc_pool = unsafe {
            self.device.create_descriptor_pool(
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
                // pso::DescriptorPoolCreateFlags::empty(),
            )
        }
        .expect("Can't create descriptor pool");
        let desc_set = unsafe { desc_pool.allocate_set(&set_layout) }.unwrap();

        // Allocate memory for Vertices and UV
        const F32_SIZE: u64 = std::mem::size_of::<f32>() as u64;
        const F32_PER_VERTEX: u64 = 2 + 2; // (x, y, u, v)
        const VERTICES: u64 = 6; // Using a triangle fan, which is the most optimal
        let mut vertex_buffer =
            unsafe { self.device.create_buffer(F32_SIZE * F32_PER_VERTEX * VERTICES, buffer::Usage::VERTEX) }.unwrap();
        let requirements = unsafe { self.device.get_buffer_requirements(&vertex_buffer) };

        use gfx_hal::{adapter::MemoryTypeId, memory::Properties};
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

        let buffer_memory = unsafe { self.device.allocate_memory(memory_type_id, requirements.size) }.unwrap();
        unsafe { self.device.bind_buffer_memory(&buffer_memory, 0, &mut vertex_buffer) }.unwrap();
        unsafe {
            const QUAD: [f32; (F32_PER_VERTEX * VERTICES) as usize] = [
                -0.5,  0.33, 0.0, 1.0,
                 0.5,  0.33, 1.0, 1.0,
                 0.5, -0.33, 1.0, 0.0,

                -0.5,  0.33, 0.0, 1.0,
                 0.5, -0.33, 1.0, 0.0,
                -0.5, -0.33, 0.0, 0.0];
            let mut vertices = self.device
                .acquire_mapping_writer::<f32>(&buffer_memory, 0..requirements.size)
                .unwrap();
            vertices[0..QUAD.len()].copy_from_slice(&QUAD);
            self.device.release_mapping_writer(vertices).unwrap();
        }

        let img_data = include_bytes!["data/logo.png"];
        let img = image::load(Cursor::new(&img_data[..]), image::PNG)
            .unwrap()
            .to_rgba();
        let (width, height) = img.dimensions();
        let kind = i::Kind::D2(width as i::Size, height as i::Size, 1, 1);
        let limits = self.adapter.physical_device.limits();
        let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
        let image_stride = 4usize;
        let row_pitch = (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
        let upload_size = (height * row_pitch) as u64;

        let mut image_upload_buffer =
            unsafe { self.device.create_buffer(upload_size, buffer::Usage::TRANSFER_SRC) }.unwrap();
        let image_mem_reqs = unsafe { self.device.get_buffer_requirements(&image_upload_buffer) };
        let image_upload_memory = unsafe { self.device.allocate_memory(memory_type_id, image_mem_reqs.size) }.unwrap();
        unsafe { self.device.bind_buffer_memory(&image_upload_memory, 0, &mut image_upload_buffer) }.unwrap();

        unsafe {
            let mut data = self.device
                .acquire_mapping_writer::<u8>(&image_upload_memory, 0..image_mem_reqs.size)
                .unwrap();
            for y in 0..height as usize {
                let row = &(*img)
                    [y * (width as usize) * image_stride..(y+1) * (width as usize) * image_stride];
                let dest_base = y * row_pitch as usize;
                data[dest_base..dest_base + row.len()].copy_from_slice(row);
            }
            self.device.release_mapping_writer(data).unwrap();
        }

        let mut image_logo = unsafe {
            self.device.create_image(
                kind,
                1,
                ColorFormat::SELF,
                i::Tiling::Optimal,
                i::Usage::TRANSFER_DST | i::Usage::SAMPLED,
                i::ViewCapabilities::empty(),
            )
        }
        .unwrap();
        let image_req = unsafe { self.device.get_image_requirements(&image_logo) };
        let device_type = self.adapter
            .physical_device
            .memory_properties()
            .memory_types
            .iter()
            .enumerate()
            .find(|&(id, memory_type)| {
                image_req.type_mask & (1 << id) != 0
                  && memory_type.properties.contains(Properties::DEVICE_LOCAL)
            })
            .map(|(id, _)| MemoryTypeId(id))
            .unwrap();
        let image_memory = unsafe { self.device.allocate_memory(device_type, image_req.size) }.unwrap();

        unsafe { self.device.bind_image_memory(&image_memory, 0, &mut image_logo) }.unwrap();

        let image_srv = unsafe {
            self.device.create_image_view(
                &image_logo,
                i::ViewKind::D2,
                ColorFormat::SELF,
                Swizzle::NO,
                COLOR_RANGE.clone()
            )
        }
        .unwrap();

        let sampler = unsafe {
            self.device.create_sampler(i::SamplerInfo::new(i::Filter::Linear, i::WrapMode::Clamp))
        }
        .expect("unable to make sampler");

        unsafe {
            self.device.write_descriptor_sets(vec![
                pso::DescriptorSetWrite {
                    set: &desc_set,
                    binding: 0,
                    array_offset: 0,
                    descriptors: Some(pso::Descriptor::Image(&image_srv, i::Layout::Undefined)),
                },
                pso::DescriptorSetWrite {
                    set: &desc_set,
                    binding: 1,
                    array_offset: 0,
                    descriptors: Some(pso::Descriptor::Sampler(&sampler)),
                }
            ])
        }

        let mut upload_fence = self.device.create_fence(false).expect("cant make fence");

        let cmd_buffer = unsafe {
            let mut cmd_buffer = self.command_pool.acquire_command_buffer::<command::OneShot>();
            cmd_buffer.begin();

            let image_barrier = m::Barrier::Image {
                states: (i::Access::empty(), i::Layout::Undefined)..(i::Access::TRANSFER_WRITE, i::Layout::TransferDstOptimal),
                target: &image_logo,
                families: None,
                range: COLOR_RANGE.clone(),
            };

            cmd_buffer.pipeline_barrier(
                PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
                m::Dependencies::empty(),
                &[image_barrier],
            );

            cmd_buffer.copy_buffer_to_image(
                &image_upload_buffer,
                &image_logo,
                i::Layout::TransferDstOptimal,
                &[command::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_width: row_pitch / (image_stride as u32),
                    buffer_height: height as u32,
                    image_layers: i::SubresourceLayers {
                        aspects: f::Aspects::COLOR,
                        level: 0,
                        layers: 0..1,
                    },
                    image_offset: i::Offset { x: 0, y: 0, z: 0 },
                    image_extent: i::Extent {
                        width,
                        height,
                        depth: 1,
                    }
                }],
            );

            let image_barrier = m::Barrier::Image {
                states: (i::Access::TRANSFER_WRITE, i::Layout::TransferDstOptimal)
                    ..(i::Access::SHADER_READ, i::Layout::ShaderReadOnlyOptimal),
                target: &image_logo,
                families: None,
                range: COLOR_RANGE.clone(),
            };
            cmd_buffer.pipeline_barrier(
                PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
                m::Dependencies::empty(),
                &[image_barrier],
            );

            cmd_buffer.finish();

            self.queue_group.queues[0].submit_nosemaphores(Some(&cmd_buffer), Some(&mut upload_fence));

            self.device.wait_for_fence(&upload_fence, u64::max_value())
                .expect("cant wait for fence");
            self.device.destroy_fence(upload_fence);

            cmd_buffer
        };

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
                specialization: pso::Specialization {
                    constants: &[pso::SpecializationConstant { id: 0, range: 0..4 }],
                    data: unsafe { std::mem::transmute::<&f32, &[u8; 4]>(&0.8f32) },
                },
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

            let dependency = pass::SubpassDependency {
                passes: pass::SubpassRef::External..pass::SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT
                    ..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: i::Access::empty()
                    ..(i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            };

            unsafe { self.device.create_render_pass(&[attachment], &[subpass], &[dependency]) }
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
                std::iter::once(&set_layout),
                // &[], // No descriptor set layout (no texture/sampler)
                &[(pso::ShaderStageFlags::VERTEX, 0..8)],
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
            stride: 16 as u32,
            rate: 0, // VertexInputRate::Vertex,
            // 0 = Per Vertex
            // 1 = Per Instance
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

        pipeline_desc.attributes.push(pso::AttributeDesc {
            location: 1,
            binding: 0,
            element: pso::Element {
                format: f::Format::Rg32Float,
                offset: 8,
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
        StaticTexture2DRectangle {
            buffer: vertex_buffer,
            cmd_buffer: cmd_buffer,
            image_upload_buffer,
            memory: image_memory,
            pipeline,
            render_pass,
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
            // 0 = Per Vertex
            // 1 = Per Instance
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

        let cmd_buffer = self
            .command_pool
            .acquire_command_buffer::<command::MultiShot>();

        StaticWhite2DTriangle {
            buffer,
            cmd_buffer,
            memory,
            pipeline,
            render_pass,
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
