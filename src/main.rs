extern crate bincode;
extern crate byteorder;
#[macro_use]
extern crate clap;
extern crate derive_more;
extern crate glium;
extern crate hibitset;
extern crate isatty;
extern crate num_traits;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tilenet;
extern crate tilenet_ren;
extern crate time;
extern crate toml;

mod macros;
// ---
mod glocals;
mod libs;
mod mediators;

use clap::{App, Arg};
use glocals::*;
use mediators::log;
use std::sync::{Arc, Mutex};

// ---

fn create_logger(s: &mut Threads) {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let buffer_full_count = Arc::new(Mutex::new(0));
    s.log_channel = Some(tx);
    s.log_channel_full_count = buffer_full_count.clone();
    s.logger = Some(std::thread::spawn(move || {
        mediators::entry_point_logger(EntryPointLogger {
            log_channel_full_count: buffer_full_count,
            receiver: rx,
        });
    }));
    log(s, 128, "MAIN", "Logger thread created", &[]);
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
    let options = s.options.clone();
    if let Some(_connect) = options.value_of("connect") {
    } else {
    };
}

fn wait_for_threads_to_exit(mut s: glocals::Main) {
    std::mem::replace(&mut s.threads.log_channel, None);
    s.threads.logger.map(|x| x.join());
}

// ---

fn main() {
    let mut s = glocals::Main::default();
    create_logger(&mut s.threads);
    parse_command_line_arguments(&mut s.options);
    run_client_or_server(&mut s);
    wait_for_threads_to_exit(s);
}
