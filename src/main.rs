#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

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

pub mod geometry;
pub mod global;
pub mod graphics;
pub mod input;
pub mod world;
pub mod cli;
pub mod srv;
use clap::{Arg, App, SubCommand};

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
        let d = slog::level_filter(Level::Critical, drain);
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
        .arg(Arg::with_name("s")
             .short("s")
             .help("Run server instead of client")
             .takes_value(false))
        .get_matches();

    if let 0 = options.occurrences_of("s") {
        Client::new().run();
    } else {
        Server::new().run();
    }
}

