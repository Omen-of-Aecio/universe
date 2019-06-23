use fast_logger::Logger;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use input;
use laminar::{Packet, Socket, SocketEvent};
use rand_pcg::Pcg64Mcg;
use rodio;
use std::net::TcpStream;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::thread::{self, JoinHandle};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Instant,
    vec::Vec,
};
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
    pub network: Socket2,
    pub random: Pcg64Mcg,
    pub threads: Threads,
    pub time: Instant,
}

pub struct Graphics {
    pub basic_text: vxdraw::text::Handle,
    pub player_quads: Vec<vxdraw::quads::Handle>,
    pub bullets_texture: vxdraw::dyntex::Layer,
    pub weapons_texture: vxdraw::dyntex::Layer,
    pub grid: vxdraw::strtex::Layer,
    pub windowing: vxdraw::VxDraw,
}

impl Default for Main {
    fn default() -> Self {
        Self {
            audio: None,
            graphics: None,
            logger: Logger::spawn_void(),
            logic: Logic::default(),
            network: random_port_socket(),
            random: Pcg64Mcg::new(0),
            threads: Threads::default(),
            time: Instant::now(),
        }
    }
}

pub type GshChannelRecv = mpsc::Receiver<Box<dyn Fn(&mut Main) + Send>>;
pub type GshChannelSend = mpsc::SyncSender<Box<dyn Fn(&mut Main) + Send>>;

pub struct GshSpawn {
    pub thread_handle: JoinHandle<()>,
    pub keep_running: Arc<AtomicBool>,
    pub channel: mpsc::Receiver<Box<dyn Fn(&mut Main) + Send>>,
    pub port: u16,
    pub channel_send: mpsc::SyncSender<Box<dyn Fn(&mut Main) + Send>>,
}

#[derive(Default)]
pub struct Threads {
    pub game_shell: Option<std::thread::JoinHandle<()>>,
    pub game_shell_keep_running: Option<Arc<AtomicBool>>,
    pub game_shell_port: Option<u16>,
    pub game_shell_channel: Option<GshChannelRecv>,
    pub game_shell_channel_send: Option<mpsc::SyncSender<Box<dyn Fn(&mut Main) + Send>>>,
    pub game_shell_connection: Option<TcpStream>,
}

// ---

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

    pub handle: Option<vxdraw::dyntex::Handle>,
}

#[derive(Default)]
pub struct PlayerData {
    pub position: Vec2,
    pub velocity: Vec2,
    pub weapon_sprite: Option<vxdraw::dyntex::Handle>,
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

    pub grid: Grid<(u8, u8, u8, u8)>,
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

use crossbeam_channel::{Receiver, Sender};
// Not sure where to put this. Helper for laminar::Socket
fn random_port_socket() -> Socket2 {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 0);
    Socket::bind(socket)
        .map(|s| {
            let port = s.0.get_port();
            Socket2 {
                socket: s.0,
                send: s.1,
                recv: s.2,
                port,
            }
        })
        .unwrap() // TODO laminar error not compatible with failure?
}
/// Temporary wrapper around Socket, until we can get all these from laminar::Socket hopefully
pub struct Socket2 {
    pub socket: Socket,
    pub send: Sender<Packet>,
    pub recv: Receiver<SocketEvent>,
    pub port: u16,
}
