use benchmarker::Benchmarker;
use clap;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use input;
use ketimer::WeakTimer;
use logger::Logger;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
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

// ---

pub struct Main<'a> {
    pub audio: Option<rodio::Sink>,
    pub commandline: clap::ArgMatches<'a>,
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
    pub player_quads: Vec<crate::mediators::vxdraw::quads::QuadHandle>,
    pub bullets_texture: crate::mediators::vxdraw::dyntex::TextureHandle,
    pub weapons_texture: crate::mediators::vxdraw::dyntex::TextureHandle,
    pub grid: crate::mediators::vxdraw::strtex::TextureHandle,
    pub windowing: vxdraw::Windowing,
}

impl Default for Main<'_> {
    fn default() -> Self {
        Self {
            audio: None,
            commandline: clap::ArgMatches::default(),
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

pub struct Client {}

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

    pub destruction: i32,

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
    pub weapon_sprite: Option<crate::mediators::vxdraw::dyntex::SpriteHandle>,
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
    pub game_config: GameConfig,
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
