use glium;
use glium::glutin;
use glium::glutin::{MouseScrollDelta, VirtualKeyCode as KeyCode};
use glium::DisplayBuild;
use global::Tile;
use graphics::Graphics;
use tilenet::TileNet;

use err::*;
use input::Input;
use rand;
use rand::Rng;

use net::msg::Message;
use net::{to_socket_addr, Socket};
use specs::DispatcherBuilder;
use srv::system::MaintainSys;
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
