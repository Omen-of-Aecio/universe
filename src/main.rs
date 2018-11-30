#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate bincode;
extern crate byteorder;
#[macro_use]
extern crate clap;
extern crate derive_more;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate glium;
extern crate hibitset;
extern crate isatty;
extern crate num_traits;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
#[macro_use]
extern crate slog_scope;
extern crate slog_stream;
extern crate slog_term;
extern crate specs;
extern crate tilenet;
extern crate tilenet_ren;
extern crate time;
extern crate toml;

mod global;
// ---
mod addons;
mod conf;
mod err;
mod glocals;
mod libs;
mod input;
mod tilenet_gen;

use clap::{App, Arg};
use conf::Config;
use glocals::{Client, Server};
use slog::{Drain, Level};

// ---

fn create_logger(s: &mut Option<slog_scope::GlobalLoggerGuard>) {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = drain.filter_level(Level::Debug).fuse();
    *s = Some(slog_scope::set_global_logger(slog::Logger::root(
        drain,
        o!(),
    )));
}

fn load_configuration_file(s: &mut Option<conf::Config>) {
    *s = Config::from_file("config.toml").ok();
}

fn parse_command_line_arguments<'a>(s: &mut clap::ArgMatches<'a>) {
    *s = {
        App::new("Universe")
            .version("0.1.0")
            .author(crate_authors!["\n"])
            .arg(
                Arg::with_name("connect")
                    .short("c")
                    .help("Run client and connect to specified server of form `ipaddress:port`")
                    .takes_value(true),
            )
            .get_matches()
    };
}

fn run_client_or_server(s: &mut glocals::Main) {
    let err = if let Some(connect) = s.options.value_of("connect") {
        info!("Running client");
        let mut client = Client::new(connect).unwrap();
        addons::cli::run(&mut client)
    } else {
        info!("Running server");
        addons::srv::run(&mut Server::new(s.config.as_ref().unwrap()))
    };
    if let Err(err) = err {
        println!("Error: {}", err);
        println!("Backtrace: {}", err.backtrace());
    }
}

// ---

fn main() {
    let mut s = glocals::Main::default();
    create_logger(&mut s._logger_guard);
    parse_command_line_arguments(&mut s.options);
    load_configuration_file(&mut s.config);
    run_client_or_server(&mut s);
}
