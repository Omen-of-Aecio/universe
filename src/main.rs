#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;

extern crate bincode;
extern crate byteorder;
extern crate clap;
extern crate derive_more;
extern crate hibitset;
extern crate isatty;
extern crate num_traits;
extern crate rand;
extern crate serde;
extern crate slog_async;
extern crate slog_json;
extern crate slog_stream;
extern crate slog_term;
extern crate specs;
extern crate tilenet;
extern crate tilenet_ren;
extern crate time;
extern crate toml;

pub mod global;

pub mod collision;
pub mod component;
pub mod conf;
pub mod err;
pub mod geometry;
pub mod graphics;
pub mod input;
pub mod net;
pub mod srv;
pub mod tilenet_gen;

mod cli;
mod glocals;

use clap::{App, Arg};
use glocals::Client;
use component::*;
use conf::Config;
use geometry::Vec2;
use global::Tile;
use slog::{Drain, Level};
use srv::Server;
use tilenet::TileNet;

fn create_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = drain.filter_level(Level::Debug).fuse();
    slog::Logger::root(drain, o!())
}

fn parse_command_line_arguments<'a>() -> clap::ArgMatches<'a> {
    App::new("Universe")
        .arg(
            Arg::with_name("connect")
                .short("c")
                .help("Run client and connect to specified server of form `ipaddress:port`")
                .takes_value(true),
        )
        .get_matches()
}

fn get_nums(a: u32, b: u32) -> impl Iterator<Item = u32> {
    (a..b).filter(|x| x % 100 == 0)
}

fn main() {
    for i in get_nums(100, 300) {
        println!["{}", i];
    }
    let mut s = glocals::Main {
        // logger: create_logger(),
        _logger_guard: slog_scope::set_global_logger(create_logger()),
        look: 10,
        options: parse_command_line_arguments(),
        config: None,
    };

    s.config = Config::from_file("config.toml").ok();

    run_client_or_server(&mut s);
}

fn run_client_or_server(s: &mut glocals::Main) {
    let err = if let Some(connect) = s.options.value_of("connect") {
        info!("Running client");
        let mut client = Client::new(connect).unwrap();
        cli::run(&mut client)
    } else {
        info!("Running server");
        Server::new(s.config.as_ref().unwrap()).run()
    };
    if let Err(err) = err {
        println!("Error: {}", err);
        println!("Backtrace: {}", err.backtrace());
    }
}

// Stuff that don't have a home...

#[derive(Default, Copy, Clone)]
pub struct DeltaTime {
    secs: f32,
}
impl DeltaTime {
    pub fn from_duration(duration: std::time::Duration) -> DeltaTime {
        DeltaTime {
            secs: duration.as_secs() as f32 + (duration.subsec_nanos() as f32) / 1_000_000_000.0,
        }
    }
}

pub fn map_tile_value_via_color(tile: Tile, color: Color) -> Tile {
    match (tile, color) {
        (0, Color::Black) => 255,
        (255, Color::Black) => 0,
        _ => tile,
    }
}
pub fn get_normal(tilenet: &TileNet<Tile>, coord: (usize, usize), color: Color) -> Vec2 {
    let cmap = map_tile_value_via_color;
    /*
    let kernel = match color {
        Color::WHITE => [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]],
        Color::BLACK => [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]],
    };
    */
    let kernel = [[1.0, 0.0, -1.0], [2.0, 0.0, -2.0], [1.0, 0.0, -1.0]];
    let mut dx = 0.0;
    let mut dy = 0.0;
    for (y, row) in kernel.iter().enumerate() {
        for (x, _) in row.iter().enumerate() {
            if let (Some(x_coord), Some(y_coord)) =
                ((coord.0 + x).checked_sub(1), (coord.1 + y).checked_sub(1))
            {
                if let Some(&v) = tilenet.get((x_coord, y_coord)) {
                    dx += kernel[y][x] * f32::from(cmap(v, color)) / 255.0;
                }
                if let Some(&v) = tilenet.get((x_coord, y_coord)) {
                    dy += kernel[x][y] * f32::from(cmap(v, color)) / 255.0;
                }
            }
        }
    }
    Vec2::new(dx, dy)
}

pub fn i32_to_usize(mut from: (i32, i32)) -> (usize, usize) {
    if from.0 < 0 {
        from.0 = 0;
    }
    if from.1 < 0 {
        from.1 = 0;
    }
    (from.0 as usize, from.1 as usize)
}
