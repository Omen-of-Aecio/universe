use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use input;
use ketimer::WeakTimer;
use logger::Logger;
use rand_pcg::Pcg64Mcg;
use rodio;
use std::net::TcpStream;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::{Duration, Instant},
    vec::Vec,
};
use udp_ack::Socket;
use vxdraw;

pub mod config;
pub mod log;

pub use self::config::Config;
pub use log::Log;

pub type Error = failure::Error;

// ---

pub struct Main {
    pub audio: Option<rodio::Sink>,
    pub graphics: Option<Graphics>,
    pub logger: Logger<Log>,
    pub logic: Logic,
    pub network: Socket<i32>,
    pub random: Pcg64Mcg,
    pub threads: Threads,
    pub time: Instant,
    pub timers: Timers,
}

pub struct Graphics {
    pub player_quads: Vec<vxdraw::quads::QuadHandle>,
    pub bullets_texture: vxdraw::dyntex::TextureHandle,
    pub weapons_texture: vxdraw::dyntex::TextureHandle,
    pub grid: vxdraw::strtex::TextureHandle,
    pub windowing: vxdraw::VxDraw,
}

impl Default for Main {
    fn default() -> Self {
        Self {
            audio: None,
            graphics: None,
            logger: Logger::spawn_void(),
            logic: Logic::default(),
            network: Socket::default(),
            random: Pcg64Mcg::new(0),
            threads: Threads::default(),
            time: Instant::now(),
            timers: Timers::default(),
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

pub type GshChannelRecv = mpsc::Receiver<Box<Fn(&mut Main) + Send>>;
pub type GshChannelSend = mpsc::SyncSender<Box<Fn(&mut Main) + Send>>;

#[derive(Default)]
pub struct Threads {
    pub game_shell: Option<std::thread::JoinHandle<()>>,
    pub game_shell_keep_running: Option<Arc<AtomicBool>>,
    pub game_shell_port: Option<u16>,
    pub game_shell_channel: Option<GshChannelRecv>,
    pub game_shell_channel_send: Option<mpsc::SyncSender<Box<Fn(&mut Main) + Send>>>,
    pub game_shell_connection: Option<TcpStream>,
}

// ---

#[derive(Clone)]
pub struct GameShell<T: Send + Sync> {
    pub gshctx: GameShellContext,
    pub commands: T,
}

#[derive(Clone)]
pub struct GameShellContext {
    pub config_change: Option<GshChannelSend>,
    pub logger: Logger<Log>,
    pub keep_running: Arc<AtomicBool>,
    pub variables: HashMap<String, String>,
}

pub struct Client {}

pub struct Bullet {
    pub direction: Vec2,
    pub position: Vec2,

    pub destruction: i32,

    pub animation_sequence: usize,
    pub animation_block_begin: (f32, f32),
    pub animation_block_end: (f32, f32),
    pub height: usize,
    pub width: usize,
    pub current_uv_begin: (f32, f32),
    pub current_uv_end: (f32, f32),

    pub handle: Option<vxdraw::dyntex::SpriteHandle>,
}

#[derive(Default)]
pub struct PlayerData {
    pub position: Vec2,
    pub velocity: Vec2,
    pub weapon_sprite: Option<vxdraw::dyntex::SpriteHandle>,
}

#[derive(PartialEq)]
pub enum Weapon {
    Hellfire,
    Ak47,
}

impl Default for Weapon {
    fn default() -> Self {
        Weapon::Hellfire
    }
}

#[derive(Default)]
pub struct Logic {
    pub should_exit: bool,
    pub input: input::Input,

    pub grid: Grid<u8>,
    pub config: Config,
    pub players: Vec<PlayerData>,

    pub current_weapon: Weapon,
    pub current_weapon_cooldown: usize,

    pub bullets: Vec<Bullet>,
    pub cam: Camera,
    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    pub cam_mode: CameraMode,

    pub changed_tiles: Vec<(usize, usize)>,
    pub bullets_added: Vec<Vec2>,
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone, PartialEq)]
pub enum CameraMode {
    Interactive,
    FollowPlayer,
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
