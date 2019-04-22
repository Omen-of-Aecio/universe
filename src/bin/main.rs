use benchmarker::Benchmarker;
use clap::crate_authors;
use clap::{App, Arg};
use input;
use rodio;
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use universe::{glocals::*, mediators::vxdraw::*, *};

// ---

fn parse_command_line_arguments<'a>() -> clap::ArgMatches<'a> {
    App::new("Universe")
        .version("0.1.0")
        .author(crate_authors!["\n"])
        .arg(
            Arg::with_name("connect")
                .short("c")
                .help("Run client and connect to specified server of form `ipaddress:port`")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .short("h")
                .help("Host a server on a port")
                .takes_value(true),
        )
        .get_matches()
}

fn run_game(s: &mut glocals::Main) {
    let commandline = s.commandline.clone();
    s.logger = logger::Logger::spawn();
    s.logger.set_colorize(true);
    s.logger.set_context_specific_log_level("benchmark", 0);
    if let Some(game_shell) = crate::mediators::game_shell::spawn(s.logger.clone()) {
        s.threads.game_shell = Some(game_shell.0);
        s.threads.game_shell_keep_running = Some(game_shell.1);
    }
    mediators::client::entry_point_client(s);
}

fn wait_for_threads_to_exit(s: glocals::Main) {
    if let Some(x) = s.threads.game_shell_keep_running {
        x.store(false, Ordering::Relaxed);
    }

    let tcp = TcpStream::connect("127.0.0.1:32931");
    std::mem::drop(tcp);

    s.threads.game_shell.map(std::thread::JoinHandle::join);
}

// ---

fn main() {
    let mut s = glocals::Main::default();
    s.commandline = parse_command_line_arguments();
    run_game(&mut s);
    wait_for_threads_to_exit(s);
}
