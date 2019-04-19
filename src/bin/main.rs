use benchmarker::Benchmarker;
use clap::crate_authors;
use clap::{App, Arg};
use glium::{glutin, DisplayBuild};
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
            Arg::with_name("vx")
                .short("v")
                .help("Use the new renderer")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("host")
                .short("h")
                .help("Host a server on a port")
                .takes_value(true),
        )
        .get_matches()
}

fn run_client_or_server(s: &mut glocals::Main) {
    let commandline = s.commandline.clone();
    if let Some(_connect) = commandline.value_of("connect") {
        let mut logger = logger::Logger::spawn();
        logger.set_colorize(true);
        logger.set_context_specific_log_level("benchmark", 0);
        if let Some(game_shell) = crate::mediators::game_shell::spawn(logger.clone()) {
            s.threads.game_shell = Some(game_shell.0);
            s.threads.game_shell_keep_running = Some(game_shell.1);
        }
        let client = Client {
            logger,
            should_exit: false,
            game: Game::default(),
            display: glutin::WindowBuilder::new()
                .with_dimensions(1024, 768)
                .with_title("Universe")
                .build_glium()
                .unwrap(),
            input: input::Input::default(),
            audio: rodio::Sink::new(&rodio::default_output_device().unwrap()),
            logic_benchmarker: Benchmarker::new(99),
            drawing_benchmarker: Benchmarker::new(99),
            windowing: None,
        };
        s.client = Some(client);
        mediators::client::entry_point_client(s);
    } else if commandline.is_present("vx") {
        let mut logger = logger::Logger::spawn();
        logger.set_colorize(true);
        logger.set_context_specific_log_level("benchmark", 0);
        if let Some(game_shell) = crate::mediators::game_shell::spawn(logger.clone()) {
            s.threads.game_shell = Some(game_shell.0);
            s.threads.game_shell_keep_running = Some(game_shell.1);
        }
        let client = Client {
            windowing: Some(init_window_with_vulkan(&mut logger, ShowWindow::Enable)),
            logger,
            should_exit: false,
            game: Game::default(),
            display: glutin::WindowBuilder::new()
                .with_dimensions(1024, 768)
                .with_title("Universe")
                .with_visibility(false)
                .build_glium()
                .unwrap(),
            input: input::Input::default(),
            audio: rodio::Sink::new(&rodio::default_output_device().unwrap()),
            logic_benchmarker: Benchmarker::new(99),
            drawing_benchmarker: Benchmarker::new(99),
        };
        s.client = Some(client);
        mediators::client::entry_point_client_vulkan(s);
    } else if let Some(_port) = commandline.value_of("host") {
        s.server = Some(Server::default());
        mediators::server::entry_point_server(s);
    }
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
    run_client_or_server(&mut s);
    wait_for_threads_to_exit(s);
}
