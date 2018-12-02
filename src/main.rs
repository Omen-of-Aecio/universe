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

// ---

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

fn run_client_or_server(s: glocals::Main) {
    let options = s.options.clone();
    if let Some(_connect) = options.value_of("connect") {
    } else {
    };
}

// ---

fn main() {
    let mut s = glocals::Main::default();
    parse_command_line_arguments(&mut s.options);
    run_client_or_server(s);
}
