use benchmarker::Benchmarker;
use clap;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use input;
use ketimer::WeakTimer;
use logger::Logger;
use rodio;
use serde_derive::Deserialize;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::{Duration, Instant},
    vec::Vec,
};
use udp_ack::Socket;

pub mod config;
pub mod log;
pub mod vxdraw;

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
    pub audio: rodio::Sink,
    pub logic_benchmarker: Benchmarker,
    pub drawing_benchmarker: Benchmarker,
    pub windowing: Option<vxdraw::Windowing>,
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
    pub direction: Vec2,
    pub position: Vec2,

    pub animation_sequence: usize,
    pub animation_block_begin: (f32, f32),
    pub animation_block_end: (f32, f32),
    pub height: usize,
    pub width: usize,
    pub current_uv_begin: (f32, f32),
    pub current_uv_end: (f32, f32),

    pub handle: Option<crate::mediators::vxdraw::dyntex::SpriteHandle>,
}

pub struct PlayerData {
    pub position: Vec2,
}

#[derive(Default)]
pub struct Game {
    pub grid: Grid<u8>,
    pub game_config: GameConfig,
    pub players2: Vec<PlayerData>,
    pub bullets: Vec<Bullet>,
    pub cam: Camera,
    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    pub cam_mode: CameraMode,
    pub bullets_handle: Option<crate::mediators::vxdraw::dyntex::TextureHandle>,
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone)]
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
