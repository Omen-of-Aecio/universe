#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]
#![feature(discriminant_value)]

extern crate bgjk;
#[macro_use]
extern crate glium;
extern crate isatty;
extern crate rand;
#[macro_use (o, slog_log, slog_trace, slog_debug, slog_info, slog_warn, slog_error)]
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

pub mod geometry;
pub mod global;
pub mod graphics;
pub mod input;
pub mod world;
pub mod cli;
pub mod srv;
pub mod net;
pub mod err;
use clap::{Arg, App};

use slog::{DrainExt, Level};
use cli::Client;
use srv::Server;

fn setup_logger() {
    let logger = if isatty::stderr_isatty() {
        let drain = slog_term::streamer()
            .async()
            .stderr()
            .full()
            .use_utc_timestamp()
            .build();
        let d = slog::level_filter(Level::Info, drain);
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

    if let Some(connect) = options.value_of("connect") {
        Client::new(connect).unwrap().run().unwrap();
    } else {
        Server::new().run().unwrap();
    }
}

