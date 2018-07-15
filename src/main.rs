#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#[macro_use]
extern crate glium;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;
extern crate isatty;
extern crate rand;
#[macro_use (o, slog_log, slog_trace, slog_debug, slog_info)]
extern crate slog;
extern crate slog_json;
#[macro_use]
extern crate slog_scope;
extern crate slog_stream;
extern crate slog_term;
extern crate tile_net;
extern crate tilenet_ren;
extern crate time;
extern crate clap;
extern crate byteorder;
extern crate bincode;
extern crate rustc_serialize;
extern crate num_traits;
extern crate specs;
extern crate toml;

pub mod err;
pub mod net;
pub mod geometry;
pub mod global;
pub mod graphics;
pub mod input;
pub mod cli;
pub mod srv;
pub mod tilenet_gen;
pub mod collision;
pub mod component;
pub mod conf;

use clap::{Arg, App};

use slog::{DrainExt, Level};
use cli::Client;
use srv::Server;
use conf::Config;

fn setup_logger() {
    let logger = if isatty::stderr_isatty() {
        let drain = slog_term::streamer()
            .async()
            .stderr()
            .full()
            .use_utc_timestamp()
            .build();
        let d = slog::level_filter(Level::Debug, drain);
        slog::Logger::root(d.fuse(), o![])
    } else {
        slog::Logger::root(slog_stream::stream(std::io::stderr(), slog_json::default()).fuse(),
                           o![])
    };
    slog_scope::set_global_logger(logger);
}


fn main() {
    setup_logger();
    let options = App::new("Universe")
        .arg(Arg::with_name("connect")
             .short("c")
             .help("Run client and connect to specified server of form `ipaddress:port`")
             .takes_value(true))
        .get_matches();

    // Read config
    let config = Config::from_file("config.toml").unwrap();

    let err = if let Some(connect) = options.value_of("connect") {

        info!("Running client");
        let mut client = Client::new(connect).unwrap();
        let err = client.run();

        match err {
            Ok(_) => std::process::exit(0),
            Err(err) => err,
        }
    } else {

        info!("Running server");
        let err = Server::new(config).run();

        match err {
            Ok(_) => std::process::exit(0),
            Err(err) => err,
        }
    };
    println!("Error: {}", err);
    for e in err.iter().skip(1) {
        println!("  caused by: {}", e);
    }


    std::process::exit(0);
}


// Functions that don't have a home...

use tile_net::TileNet;
use global::Tile;
use component::*;
use geometry::Vec2;

pub fn map_tile_value_via_color(tile: &Tile, color: Color) -> Tile {
	match (tile, color) {
		(&0, Color::Black) => 255,
		(&255, Color::Black) => 0,
		_ => *tile,
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
            if let (Some(x_coord), Some(y_coord)) = ((coord.0 + x).checked_sub(1),
                                                     (coord.1 + y).checked_sub(1)) {
                tilenet.get((x_coord, y_coord)).map(|&v| dx += kernel[y][x] * cmap(&v, color) as f32 / 255.0);
                tilenet.get((x_coord, y_coord)).map(|&v| dy += kernel[x][y] * cmap(&v, color) as f32 / 255.0);
            }
        }
    }
    Vec2::new(dx, dy)
}

pub fn i32_to_usize(mut from: (i32, i32)) -> (usize, usize) {
    if from.0 < 0 { from.0 = 0; }
    if from.1 < 0 { from.1 = 0; }
    (from.0 as usize, from.1 as usize)
}
