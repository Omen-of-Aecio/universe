use benchmarker::Benchmarker;
use clap;
use core::ptr::read;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
use gfx_hal::{
    command,
    device::Device,
    format::{self, ChannelType, Swizzle},
    image, pass, pool,
    window::{Extent2D, PresentMode::*, Surface, Swapchain},
    Adapter, Backbuffer, Backend, FrameSync, Instance, SwapchainConfig,
};
pub use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
use glium::{implement_vertex, texture::Texture2d};
use input;
use ketimer::WeakTimer;
use logger::Logger;
use rodio;
use serde_derive::Deserialize;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    mem::ManuallyDrop,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::{Duration, Instant},
    vec::Vec,
};
use udp_ack::Socket;

pub mod config;
pub mod log;

pub use log::Log;

pub type Error = failure::Error;

pub struct NamedFn {
    pub name: &'static str,
    pub func: fn(&mut Main),
}
impl Default for NamedFn {
    fn default() -> Self {
        Self {
            name: "",
            func: |&mut _| {},
        }
    }
}
impl PartialEq for NamedFn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for NamedFn {}
impl Hash for NamedFn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

pub struct Main<'a> {
    pub client: Option<Client>,
    pub commandline: clap::ArgMatches<'a>,
    pub config: Config,
    pub config_change_recv: Option<mpsc::Receiver<fn(&mut Config)>>,
    pub network: Option<Socket<i32>>,
    pub server: Option<Server>,
    pub threads: Threads,
    pub time: Instant,
    pub timers: Timers,
    pub windowing: Option<Windowing>,
}

impl Default for Main<'_> {
    fn default() -> Self {
        Self {
            client: None,
            commandline: clap::ArgMatches::default(),
            config: Config::default(),
            config_change_recv: None,
            network: None,
            server: None,
            threads: Threads::default(),
            time: Instant::now(),
            timers: Timers::default(),
            windowing: None,
        }
    }
}

pub struct Windowing {
    pub window: winit::Window,
    pub events_loop: winit::EventsLoop,

    pub current_frame: usize,
    pub image_count: usize,
    pub render_area: gfx_hal::pso::Rect,

    pub frame_render_fences: Vec<<back::Backend as Backend>::Fence>,
    pub frame_fences: Vec<<back::Backend as Backend>::Fence>,
    pub acquire_image_semaphores: Vec<<back::Backend as Backend>::Semaphore>,
    pub present_wait_semaphores: Vec<<back::Backend as Backend>::Semaphore>,
    pub command_pool: ManuallyDrop<gfx_hal::pool::CommandPool<back::Backend, gfx_hal::Graphics>>,
    pub framebuffers: Vec<<back::Backend as Backend>::Framebuffer>,
    pub command_buffers: Vec<
        gfx_hal::command::CommandBuffer<
            back::Backend,
            gfx_hal::Graphics,
            gfx_hal::command::MultiShot,
            gfx_hal::command::Primary,
        >,
    >,
    pub image_views: Vec<<back::Backend as Backend>::ImageView>,
    pub render_pass: ManuallyDrop<<back::Backend as Backend>::RenderPass>,
    pub queue_group: ManuallyDrop<gfx_hal::QueueGroup<back::Backend, gfx_hal::Graphics>>,
    pub swapchain: ManuallyDrop<<back::Backend as Backend>::Swapchain>,
    pub device: ManuallyDrop<back::Device>,
    pub adapter: Adapter<back::Backend>,
    pub surf: <back::Backend as Backend>::Surface,

    pub vk_inst: ManuallyDrop<back::Instance>,
}

impl Drop for Windowing {
    fn drop(&mut self) {
        let _ = self.device.wait_idle();

        for fence in self.frame_fences.drain(..) {
            unsafe {
                self.device.destroy_fence(fence);
            }
        }

        for fence in self.frame_render_fences.drain(..) {
            unsafe {
                self.device.destroy_fence(fence);
            }
        }

        for sema in self.acquire_image_semaphores.drain(..) {
            unsafe {
                self.device.destroy_semaphore(sema);
            }
        }

        for sema in self.present_wait_semaphores.drain(..) {
            unsafe {
                self.device.destroy_semaphore(sema);
            }
        }

        for fb in self.framebuffers.drain(..) {
            unsafe {
                self.device.destroy_framebuffer(fb);
            }
        }

        for iv in self.image_views.drain(..) {
            unsafe {
                self.device.destroy_image_view(iv);
            }
        }

        unsafe {
            self.device.destroy_command_pool(
                ManuallyDrop::into_inner(read(&self.command_pool)).into_raw(),
            );
            self.device
                .destroy_render_pass(ManuallyDrop::into_inner(read(&self.render_pass)));
            self.device
                .destroy_swapchain(ManuallyDrop::into_inner(read(&self.swapchain)));
            ManuallyDrop::drop(&mut self.queue_group);
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.vk_inst);
        }
    }
}

pub struct Timers {
    pub network_timer: WeakTimer<Socket<i32>, Result<bool, Error>>,
}

impl<'a> Default for Timers {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            network_timer: WeakTimer::new(Socket::update, Duration::new(1, 0), now),
        }
    }
}

#[derive(Default)]
pub struct Threads {
    pub game_shell: Option<std::thread::JoinHandle<()>>,
    pub game_shell_keep_running: Option<Arc<AtomicBool>>,
}

// ---

#[derive(Clone)]
pub struct GameShell<T: Send + Sync> {
    pub gshctx: GameShellContext,
    pub commands: T,
}

#[derive(Clone)]
pub struct GameShellContext {
    pub config_change: Option<mpsc::SyncSender<fn(&mut Config)>>,
    pub logger: Logger<Log>,
    pub keep_running: Arc<AtomicBool>,
    pub variables: HashMap<String, String>,
}

pub struct Client {
    pub logger: Logger<Log>,
    pub should_exit: bool,
    pub game: Game,
    pub input: input::Input,
    pub display: glium::Display,
    pub audio: rodio::Sink,
    pub logic_benchmarker: Benchmarker,
    pub drawing_benchmarker: Benchmarker,
    // Networking
    // pub server: SocketAddr,
}

#[derive(Default)]
pub struct Server {
    pub game: ServerGame,
    pub connections: HashMap<std::net::SocketAddr, Connection>,

    /// Frame duration in seconds (used only for how long to sleep. FPS is in GameConfig)
    pub tick_duration: Duration,
}

#[derive(Clone, Default)]
pub struct Connection {
    pub last_snapshot: u32, // frame#
    pub snapshot_rate: u64,
}

#[derive(Default)]
pub struct ServerGame {
    pub frame: u32,
    pub game_conf: GameConfig,

    /// Mapping from unique ID to specs Entity
    // pub entities: HashMap<u32, specs::Entity>,
    pub entity_id_seq: u32,

    /// Width of the generated world
    pub width: usize,
    /// Height of the generated world
    pub height: usize,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,
}

#[derive(Copy, Clone, Default)]
pub struct GameConfig {
    pub hori_acc: f32,
    pub jump_duration: f32,
    pub jump_delay: f32,
    pub jump_acc: f32,
    pub gravity: Vec2,
    pub gravity_on: bool,
    pub srv_tick_duration: Duration,
    pub air_fri: Vec2,
    pub ground_fri: f32,
}

#[derive(Default, Deserialize, Clone)]
pub struct Config {
    pub player: PlayerConfig,
    pub world: WorldConfig,
    pub srv: ServerConfig,
}

#[derive(Default, Deserialize, Clone)]
pub struct PlayerConfig {
    pub hori_acc: f32,
    pub jump_duration: f32,
    pub jump_delay: f32,
    pub jump_acc: f32,
    pub snapshot_rate: f32,
}
#[derive(Default, Deserialize, Clone)]
pub struct WorldConfig {
    pub width: u32,
    pub height: u32,
    pub gravity: f32,
    pub air_fri: (f32, f32),
    pub ground_fri: f32,
}

#[derive(Default, Deserialize, Clone)]
pub struct ServerConfig {
    pub ticks_per_second: u32,
}

pub struct Bullet {
    pub render: PolygonRenderData,
    pub direction: Vec2,
    pub position: Vec2,
}

#[derive(Default)]
pub struct Game {
    pub grid: Grid<u8>,
    pub game_config: GameConfig,
    pub players: Vec<PolygonRenderData>,
    pub bullets: Vec<Bullet>,
    pub cam: Camera,
    pub grid_render: Option<GridU8RenderData>,
    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    pub cam_mode: CameraMode,
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone)]
pub enum CameraMode {
    Interactive,
    FollowPlayer,
}

pub struct GridU8RenderData {
    pub net_width: usize,
    pub net_height: usize,

    pub shader_prg: glium::Program,
    pub quad_vbo: glium::VertexBuffer<Vertex>,
    pub texture: Texture2d,

    pub bg_col: [f32; 3],
    pub minify_filter: MinifySamplerFilter,
    pub magnify_filter: MagnifySamplerFilter,
    pub smooth: bool,
}

pub struct PolygonRenderData {
    pub prg: glium::Program,
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub position: Vec2,
    pub velocity: Vec2,
}

// ---

impl Default for CameraMode {
    fn default() -> CameraMode {
        CameraMode::Interactive
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub pos: [f32; 2],
}

implement_vertex!(Vertex, pos);
