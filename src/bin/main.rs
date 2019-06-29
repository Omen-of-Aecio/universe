use std::net::TcpStream;
use std::sync::atomic::Ordering;
use universe::{glocals::*, *};

// ---

fn parse_arguments(s: &mut Main) {
    use clap::{App, Arg};
    use std::net::SocketAddr;
    let matches = App::new("Universe")
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
        .get_matches();

    if let Some(address) = matches.value_of("connect") {
        let address: SocketAddr = address.parse().expect("Not a valid ip:port argument");
        eprintln!("\nSent message to {:?}\n", address);
        use laminar::Packet;
        s.network
            .send(Packet::reliable_unordered(address, vec![65, 66, 67, 68]))
            .expect("Unable to send message");
    }
}

fn read_config(path: &str) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn run_game(s: &mut glocals::Main) {
    s.logger = fast_logger::Logger::spawn();
    s.logger.set_colorize(true);
    s.logger.set_context_specific_log_level("benchmark", 0);
    let game_shell = crate::mediators::game_shell::spawn_with_any_port(s.logger.clone());
    s.threads.game_shell = Some(game_shell.thread_handle);
    s.threads.game_shell_keep_running = Some(game_shell.keep_running);
    s.threads.game_shell_channel = Some(game_shell.channel);
    s.threads.game_shell_port = Some(game_shell.port);

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
    s.logic.config = read_config("config.toml").unwrap();
    parse_arguments(&mut s);
    run_game(&mut s);
    wait_for_threads_to_exit(s);
}
