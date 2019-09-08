use crate::game::{Client, GraphicsSettings, Main, Server};
use clap::{App, Arg, ArgMatches};
use failure;
use fast_logger::{debug, Logger};
use file_rotate::{FileRotate, RotationMode};
use laminar::Packet;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::{error::Error, fmt, fs, io};
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

fn read_config(path: &str) -> Result<Config, failure::Error> {
    let contents = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn wait_for_threads_to_exit(s: &mut Client) {
    if let Some(ref x) = &s.threads.game_shell_keep_running {
        x.store(false, Ordering::Relaxed);
    }

    let tcp = TcpStream::connect("127.0.0.1:32931");
    std::mem::drop(tcp);

    s.threads
        .game_shell
        .take()
        .map(std::thread::JoinHandle::join);
}

// ---

fn main() -> io::Result<()> {
    let _ = fs::create_dir("logs");
    let writer = OrWriter {
        left: FileRotate::new("logs/log", RotationMode::Lines(10_000), 3),
        right: io::stdout(),
    };
    let mut logger = Logger::spawn_with_writer("main", writer);
    logger.set_colorize(true);
    logger.set_context_specific_log_level("benchmark", 0);
    logger.set_log_level(255);

    let config = read_config("config.toml").unwrap();

    // Parse arguments
    let matches = parse_arguments();
    if let Some(address) = matches.value_of("connect") {
        let address: SocketAddr = address.parse().expect("Not a valid ip:port argument");

        let mut cli = Client::new(
            logger.clone_with_context("client"),
            GraphicsSettings::EnableGraphics,
        );
        cli.apply_config(config.clone());

        debug![logger, "Sending message to"; "address" => address];
        cli.network
            .send(Packet::reliable_unordered(address, vec![65, 66, 67, 68]))
            .expect("Unable to send message");
        unimplemented!();
    } else {
        // Run client + server
        let mut cli = Client::new(
            logger.clone_with_context("client"),
            GraphicsSettings::EnableGraphics,
        );
        cli.apply_config(config.clone());
        let mut srv = Server::new(logger.clone_with_context("server"));
        srv.apply_config(config.clone());
        let mut main = Main::new(Some(cli), Some(srv), logger.clone());
        main.entry_point();

        if let Some(ref mut cli) = main.cli.take() {
            wait_for_threads_to_exit(cli);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct DualError {
    left: Option<io::Error>,
    right: Option<io::Error>,
}

impl Error for DualError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for DualError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref err) = self.left {
            write![f, "{}", err]?;
        }
        if let Some(ref err) = self.right {
            if self.left.is_some() {
                writeln![f]?;
            }
            write![f, "{}", err]?;
        }
        Ok(())
    }
}

/// Writes input
struct OrWriter<L: io::Write, R: io::Write> {
    left: L,
    right: R,
}

impl<L: io::Write, R: io::Write> io::Write for OrWriter<L, R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let left_result = self.left.write_all(buf);
        let right_result = self.right.write_all(buf);

        if left_result.is_err() || right_result.is_err() {
            Err(io::Error::new(
                io::ErrorKind::Other,
                Box::new(DualError {
                    left: left_result.err(),
                    right: right_result.err(),
                }),
            ))
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
