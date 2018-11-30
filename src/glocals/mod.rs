use self::game::Game;
use clap;
use conf;
use libs::geometry::vec::Vec2;
use glium;
use graphics::Graphics;
use input::Input;
use net::Socket;
use specs::{self, World};
use std::{collections::HashMap, net::SocketAddr, time::Duration, vec::Vec};

pub mod cam;
pub mod game;
pub mod system;

#[derive(Default)]
pub struct Main<'a> {
    pub _logger_guard: Option<slog_scope::GlobalLoggerGuard>,
    pub config: Option<conf::Config>,
    pub options: clap::ArgMatches<'a>,
}

pub struct Client {
    pub game: Game,
    pub input: Input,
    pub display: glium::Display,
    pub graphics: Graphics,

    // Networking
    pub socket: Socket,
    pub server: SocketAddr,
}

pub struct Server {
    pub game: ServerGame,
    pub connections: HashMap<SocketAddr, Connection>,
    pub socket: Socket,

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

pub struct ServerGame {
    pub frame: u32,
    pub world: World,
    pub game_conf: GameConfig,

    /// Mapping from unique ID to specs Entity
    pub entities: HashMap<u32, specs::Entity>,
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
