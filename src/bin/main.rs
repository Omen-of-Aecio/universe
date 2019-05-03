use std::net::TcpStream;
use std::sync::atomic::Ordering;
use universe::{glocals::*, *};

// ---
fn read_config(path: &str) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn run_game(s: &mut glocals::Main) {
    s.logger = logger::Logger::spawn();
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
    run_game(&mut s);
    wait_for_threads_to_exit(s);
}
