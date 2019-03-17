use benchmarker::Benchmarker;
use clap;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
pub use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
use glium::{implement_vertex, texture::Texture2d};
use input;
use ketimer::WeakTimer;
use logger::Logger;
use priority_queue::PriorityQueue;
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

#[derive(Default)]
pub struct Main<'a> {
    pub commandline: clap::ArgMatches<'a>,
    pub config: Config,
    pub config_change_recv: Option<mpsc::Receiver<fn(&mut Config)>>,
    pub network: Option<Socket<i32>>,
    pub threads: Threads,
    pub timers: Timers,
}

pub struct Timers {
    pub time: Instant,
    pub network_timer: WeakTimer<Socket<i32>, Result<bool, Error>>,
}

impl<'a> Default for Timers {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            network_timer: WeakTimer::new(|x, i| x.update(i), Duration::new(1, 0), now),
            time: now,
        }
    }
}

#[derive(Default)]
pub struct Threads {
    pub logger: Option<std::thread::JoinHandle<()>>,
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

// ---

#[derive(Clone, Debug)]
pub enum Log {
    Bool(&'static str, &'static str, bool),
    Char(&'static str, &'static str, char),
    Coordinates(Vec2, Vec2),
    Dynamic(String),
    I64(&'static str, &'static str, i64),
    Static(&'static str),
    StaticDynamic(&'static str, &'static str, String),
    StaticDynamics(&'static str, Vec<(&'static str, String)>),
    U64(&'static str, &'static str, u64),
    U8(&'static str, &'static str, u8),
    Usize(&'static str, &'static str, usize),
    Usize2(&'static str, &'static str, usize, &'static str, usize),
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Log::Bool(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Char(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Coordinates(world, mouse) => {
                write![f, "Mouse on screen, world={:?}, mouse={:?}", world, mouse]
            }
            Log::Dynamic(str) => write![f, "{}", str],
            Log::I64(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Static(str) => write![f, "{}", str],
            Log::StaticDynamic(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::StaticDynamics(msg, kvs) => {
                write![f, "{}", msg]?;
                if !kvs.is_empty() {
                    write![f, ", "]?;
                    for kv in kvs.iter().take(kvs.len() - 1) {
                        write![f, "{}={}, ", kv.0, kv.1]?;
                    }
                    let kv = kvs.last().unwrap();
                    write![f, "{}={}", kv.0, kv.1]?;
                }
                Ok(())
            }
            Log::U8(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::U64(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Usize(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Usize2(msg, key, value, key2, value2) => {
                write![f, "{}, {}={}, {}={}", msg, key, value, key2, value2]
            }
        }
    }
}

pub struct Client<'a> {
    pub logger: Logger<Log>,
    pub should_exit: bool,
    pub main: Main<'a>,
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
pub struct Server<'a> {
    pub main: Main<'a>,
    pub game: ServerGame,
    pub connections: HashMap<std::net::SocketAddr, Connection>,

    /// Frame duration in seconds (used only for how long to sleep. FPS is in GameConfig)
    pub tick_duration: Duration,
}

#[derive(Clone, Default)]
pub struct Connection {
    /// Unique id in the ECS
    pub ecs_id: u32,
    pub last_snapshot: u32, // frame#
    pub snapshot_rate: f32,
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
