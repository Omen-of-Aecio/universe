use clap::crate_authors;
use clap::{App, Arg};
use glium::{glutin, DisplayBuild};
use universe::{glocals::*, mediators::logger::*, mediators::*, *};

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

fn run_client_or_server(s: glocals::Main) -> glocals::Main {
    let commandline = s.commandline.clone();
    if let Some(_connect) = commandline.value_of("connect") {
        {
            let mut client = Client {
                should_exit: false,
                main: s,
                game: Game::default(),
                display: glutin::WindowBuilder::new()
                    .with_dimensions(1024, 768)
                    .with_title("Universe")
                    .build_glium()
                    .unwrap(),
                input: libs::input::Input::default(),
            };
            mediators::client::entry_point_client(&mut client);
            client.main
        }
    } else {
        s
    }
}

fn wait_for_threads_to_exit(mut s: glocals::Main) {
    std::mem::replace(&mut s.threads.log_channel, None);
    s.threads.logger.map(|x| x.join());
}

// ---

fn main() {
    let mut s = glocals::Main::default();
    create_logger(&mut s.threads);
    parse_command_line_arguments(&mut s.commandline);
    s = run_client_or_server(s);
    wait_for_threads_to_exit(s);
}
