use glium;
use graphics::Graphics;

use input::Input;

use net::Socket;
use std::net::SocketAddr;

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
