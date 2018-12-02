use clap;
use libs::{
    geometry::{cam::Camera, vec::Vec2},
    input,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
    vec::Vec,
};

#[derive(Default)]
pub struct Main<'a> {
    pub config: Config,
    pub commandline: clap::ArgMatches<'a>,
    pub threads: Threads,
}

pub struct EntryPointLogger {
    pub receiver: std::sync::mpsc::Receiver<LogMessage>,
    pub log_channel_full_count: Arc<Mutex<usize>>,
}

#[derive(Default)]
pub struct Threads {
    pub logger: Option<std::thread::JoinHandle<()>>,
    pub log_channel: Option<std::sync::mpsc::SyncSender<LogMessage>>,
    pub log_channel_full_count: Arc<Mutex<usize>>,
}

#[derive(Default)]
pub struct LogMessage {
    pub loglevel: u8,
    pub context: String,
    pub message: String,
    pub kvpairs: HashMap<String, String>,
}

#[derive(Default)]
pub struct Client<'a> {
    pub main: Main<'a>,
    // pub game: Game,
    // pub input: input::Input,
    // pub display: glium::Display,

    // Networking
    // pub server: SocketAddr,
}

#[derive(Default)]
pub struct Server<'a> {
    pub main: Main<'a>,
    pub game: ServerGame,
    pub connections: HashMap<SocketAddr, Connection>,

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

#[derive(Default)]
pub struct Game {
    // pub world: World,
    pub cam: Camera,

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

// ---

impl Default for CameraMode {
    fn default() -> CameraMode {
        CameraMode::Interactive
    }
}
