use crate::game::{Client, GraphicsSettings, Main, Server};
use clap::{App, Arg, ArgMatches};
use failure::Error;
use fast_logger::{debug, Logger};
use laminar::Packet;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use universe::{glocals::*, *};

// ---

fn parse_arguments() -> ArgMatches<'static> {
    App::new("Universe")
        .version("0.1.0")
        .about("Does awesome things")
        .arg(
            Arg::with_name("connect")
                .short("c")
                .long("connect")
                .value_name("ip:port")
                .help("Connect to another player")
                .takes_value(true),
        )
        .get_matches()
}

fn read_config(path: &str) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn wait_for_threads_to_exit(s: Client) {
    if let Some(x) = s.threads.game_shell_keep_running {
        x.store(false, Ordering::Relaxed);
    }

    let tcp = TcpStream::connect("127.0.0.1:32931");
    std::mem::drop(tcp);

    s.threads.game_shell.map(std::thread::JoinHandle::join);
}

// ---

fn main() {
    let mut logger = Logger::spawn("main");
    logger.set_colorize(true);
    logger.set_context_specific_log_level("benchmark", 0);
    logger.set_log_level(196);

    let config = read_config("config.toml").unwrap();

    // Parse arguments
    let matches = parse_arguments();
    if let Some(address) = matches.value_of("connect") {
        let address: SocketAddr = address.parse().expect("Not a valid ip:port argument");

        let mut cli = Client::new(logger.clone(), GraphicsSettings::EnableGraphics);
        cli.apply_config(config.clone());

        debug![logger, "Sending message to"; "address" => address];
        cli.network
            .send(Packet::reliable_unordered(address, vec![65, 66, 67, 68]))
            .expect("Unable to send message");
        unimplemented!();
    } else {
        // Run client + server
        let mut cli = Client::new(logger.clone(), GraphicsSettings::EnableGraphics);
        cli.apply_config(config.clone());
        let mut srv = Server::new(logger.clone());
        srv.apply_config(config.clone());
        let mut main = Main::new(Some(cli), Some(srv), logger.clone());
        main.entry_point();
    }
}
