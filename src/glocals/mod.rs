use glium;
use graphics::Graphics;

use input::Input;

use net::Socket;
use std::net::SocketAddr;

use std::{
    collections::HashMap,
    time::Duration,
};

pub mod cam;
pub mod game;
pub mod system;
use self::game::Game;
use clap;
use conf;

pub struct Main<'a> {
    pub _logger_guard: slog_scope::GlobalLoggerGuard,
    pub config: Option<conf::Config>,
    pub look: u32,
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
    pub game: ::srv::game::Game,
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
