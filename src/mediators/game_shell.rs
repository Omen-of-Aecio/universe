use self::command_handlers::*;
use crate::glocals::*;
use fast_logger::{self, Logger};
use gameshell::Spec;
use gameshell::{predicates::*, types::Type, Evaluator, GameShell, IncConsumer};
use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};

mod command_handlers;

// ---

#[rustfmt::skip]
const SPEC: &[Spec<Type, String, GameShellContext>] = &[
    (&[("%", MANY_I32)], modulo),
    (&[("&", MANY_I32)], band),
    (&[("*", MANY_I32)], mul),
    (&[("+", MANY_I32)], add),
    (&[("-", MANY_I32)], sub),
    (&[("/", MANY_I32)], div),
    (&[("^", MANY_I32)], xor),
    (&[("cat", MANY_STRING)], cat),
    (&[("config", None), ("fps", None), ("get", None)], get_fps),
    (&[("config", None), ("fps", None), ("set", ANY_F32)], set_fps),
    (&[("config", None), ("gravity", None), ("enable", ANY_BOOL)], enable_gravity),
    (&[("config", None), ("gravity", None), ("set", None), ("y", ANY_F32)], set_gravity),
    (&[("get", ANY_STRING)], do_get),
    (&[("log", None), ("context", ANY_ATOM), ("level", ANY_U8)], log_context),
    (&[("log", None), ("global", None), ("level", ANY_U8)], log),
    (&[("log", None), ("trace", ANY_STRING)], log_trace),
    (&[("set", TWO_STRINGS)], do_set),
    (&[("str", ANY_STRING)], create_string),
    (&[("void", IGNORE_ALL)], void),
    (&[("|", MANY_I32)], bor),
];

// ---

#[derive(Clone)]
pub struct GameShellContext {
    pub config_change: Option<GshChannelSend>,
    pub logger: Logger<Log>,
    pub keep_running: Arc<AtomicBool>,
    pub variables: HashMap<String, String>,
}

impl Default for GameShellContext {
    fn default() -> Self {
        Self {
            config_change: None,
            logger: Logger::spawn_void(),
            keep_running: Arc::new(AtomicBool::new(true)),
            variables: HashMap::new(),
        }
    }
}

pub fn make_new_gameshell() -> Evaluator<'static, GameShellContext> {
    let gsh_spawn = spawn_with_any_port(Logger::spawn_void());
    let mut gsh = Evaluator::new(GameShellContext {
        config_change: Some(gsh_spawn.channel_send),
        logger: Logger::spawn_void(),
        keep_running: gsh_spawn.keep_running.clone(),
        variables: HashMap::new(),
    });
    gsh.register_many(SPEC).unwrap();
    gsh
}

fn spawn_with_listener(logger: Logger<Log>, listener: TcpListener, port: u16) -> GshSpawn {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    let (tx, rx) = mpsc::sync_channel(2);
    let tx_clone = tx.clone();
    GshSpawn {
        thread_handle: thread::Builder::new()
            .name("gsh/server".to_string())
            .spawn(move || {
                game_shell_thread(
                    GameShellContext {
                        config_change: Some(tx_clone),
                        logger: logger.clone_with_context("gsh"),
                        keep_running,
                        variables: HashMap::new(),
                    },
                    listener,
                )
            })
            .unwrap(),
        keep_running: keep_running_clone,
        channel: rx,
        port,
        channel_send: tx,
    }
}

pub fn spawn(mut logger: Logger<Log>) -> Option<GshSpawn> {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:32931") {
        Some(spawn_with_listener(logger, listener, 32931))
    } else {
        logger.info("Unable to bind to tcp port");
        None
    }
}

pub fn spawn_with_any_port(mut logger: Logger<Log>) -> GshSpawn {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:32931") {
        spawn_with_listener(logger, listener, 32931)
    } else {
        logger.info("Unable to bind to tcp port");
        let (listener, port) = bind_to_any_tcp_port();
        spawn_with_listener(logger, listener, port)
    }
}

fn bind_to_any_tcp_port() -> (TcpListener, u16) {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 0);
    let listener = TcpListener::bind(socket).expect("Unable to find an available port");
    let port = listener
        .local_addr()
        .expect("Listener has no local address")
        .port();
    (listener, port)
}

// ---

fn clone_and_spawn_connection_handler(s: &GameShellContext, stream: TcpStream) -> JoinHandle<()> {
    let ctx = s.clone();
    thread::Builder::new()
        .name("gsh/server/handler".to_string())
        .spawn(move || {
            let mut logger = ctx.logger.clone();
            let result = connection_loop(ctx, stream);
            match result {
                Ok(()) => {
                    logger.debug("Connection ended ok");
                }
                Err(error) => {
                    logger.debug(Log::StaticDynamic(
                        "Connection errored out",
                        "reason",
                        format!["{:?}", error],
                    ));
                }
            }
        })
        .unwrap()
}

// ---

fn connection_loop(mut s: GameShellContext, stream: TcpStream) -> io::Result<()> {
    s.logger.debug("Acquired new stream");
    let mut gsh = GameShell::new(s, stream.try_clone().unwrap(), stream);
    gsh.register_many(SPEC).unwrap();
    let buffer = &mut [0u8; 2048];
    gsh.run(buffer);
    Ok(())
}

fn game_shell_thread(mut s: GameShellContext, listener: TcpListener) {
    s.logger.info("Started GameShell server");
    'outer_loop: loop {
        for stream in listener.incoming() {
            if !s.keep_running.load(Ordering::Acquire) {
                s.logger.info("Stopped GameShell server");
                break 'outer_loop;
            }
            match stream {
                Ok(stream) => {
                    clone_and_spawn_connection_handler(&s, stream);
                }
                Err(error) => {
                    s.logger.error(Log::StaticDynamic(
                        "Got a stream but there was an error",
                        "reason",
                        format!["{:?}", error],
                    ));
                }
            }
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use gameshell::Evaluate;
    use gameshell::Feedback;
    use std::io::{self, Read, Write};
    use test::{black_box, Bencher};

    // ---

    #[test]
    fn change_log_level() -> io::Result<()> {
        let logger = fast_logger::Logger::spawn_void();
        assert_ne![123, logger.get_log_level()];
        let (listener, port) = bind_to_any_tcp_port();
        let gshspawn = spawn_with_listener(logger.clone(), listener, port);
        let mut listener =
            TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref()).unwrap();
        {
            writeln![listener, "log global level 123"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 256])?;
        }
        assert_eq![123, logger.get_log_level()];
        gshspawn.keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        let _ = TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;

        Ok(())
    }

    #[test]
    fn fuzzing_result_does_not_crash() {
        let mut gsh = make_new_gameshell();
        let input = "y\u{000b}1111-31492546713013106(\u{00cc}\u{00a7}121B)1\u{00f0}\u{0094}\u{00a0}\u{0080}02291\0";
        assert_eq![
            Feedback::Err("Unrecognized mapping: Ì§121B".into()),
            gsh.interpret_single(input).unwrap()
        ];
    }

    #[test]
    fn check_variable_statements() {
        let mut gsh = make_new_gameshell();

        assert_eq![
            Feedback::Ok("Ok".into()),
            gsh.interpret_single("set key some-value").unwrap()
        ];
        assert_eq![
            Feedback::Ok("some-value".into()),
            gsh.interpret_single("get key").unwrap()
        ];

        assert_eq![
            Feedback::Err("Unrecognized mapping: extra".into()),
            gsh.interpret_single("set key some-value extra").unwrap()
        ];

        assert_eq![
            Feedback::Ok("Ok".into()),
            gsh.interpret_single("set a 123").unwrap()
        ];
        assert_eq![
            Feedback::Ok("130".into()),
            gsh.interpret_single("+ 7 (get a)").unwrap()
        ];
    }

    #[test]
    fn check_idempotent_statements_work() {
        let mut gsh = make_new_gameshell();

        assert_eq![
            Feedback::Err("Unrecognized mapping: hello".into()),
            gsh.interpret_single("hello world").unwrap()
        ];
        assert_eq![
            Feedback::Ok("some thing\n new ".into()),
            gsh.interpret_single("str (#some thing\n new )").unwrap()
        ];
        assert_eq![
            Feedback::Ok("6".into()),
            gsh.interpret_single("+ 1 2 3").unwrap()
        ];
        assert_eq![
            Feedback::Ok("21".into()),
            gsh.interpret_single("+ 1 (+ 8 9) 3").unwrap()
        ];
        assert_eq![
            Feedback::Ok("21".into()),
            gsh.interpret_single("+ 1 (+ 8 (+) 9) 3").unwrap()
        ];
        assert_eq![
            Feedback::Ok("22".into()),
            gsh.interpret_single("+ 1 (+ 8 (+ 1) 9) 3").unwrap()
        ];
        assert_eq![
            Feedback::Ok("".into()),
            gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 9) 3\nvoid").unwrap()
        ];
        assert_eq![
            Feedback::Err("Unrecognized mapping: 0.6".into()),
            gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 0.6 9) (+ 3\n1\n)")
                .unwrap()
        ];
        assert_eq![
            Feedback::Err("Unrecognized mapping: undefined".into()),
            gsh.interpret_single("+ (undefined)").unwrap()
        ];
        assert_eq![
            Feedback::Ok("1".into()),
            gsh.interpret_single("+ (+ 1)").unwrap()
        ];
        assert_eq![
            Feedback::Ok("2".into()),
            gsh.interpret_single("+ (+ 1 0 0 0 0 0 0 0 0 1)").unwrap()
        ];
        assert_eq![
            Feedback::Ok("-3".into()),
            gsh.interpret_single("- 3").unwrap()
        ];
        assert_eq![Feedback::Ok("0".into()), gsh.interpret_single("-").unwrap()];
        assert_eq![
            Feedback::Ok("3".into()),
            gsh.interpret_single("- 3 0").unwrap()
        ];
        assert_eq![
            Feedback::Ok("6".into()),
            gsh.interpret_single("* 3 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("1".into()),
            gsh.interpret_single("/ 3 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("1".into()),
            gsh.interpret_single("% 7 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("3".into()),
            gsh.interpret_single("^ 1 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("0".into()),
            gsh.interpret_single("& 1 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("6".into()),
            gsh.interpret_single("| 4 2").unwrap()
        ];
        assert_eq![
            Feedback::Ok("<atom>".into()),
            gsh.interpret_single("autocomplete log context").unwrap()
        ];
        assert_eq![
            Feedback::Ok("<u8>".into()),
            gsh.interpret_single("autocomplete log context gsh level ")
                .unwrap()
        ];
        assert_eq![
            Feedback::Ok("context <atom>, global, trace <string> (final)".into()),
            gsh.interpret_single("autocomplete log").unwrap()
        ];
        assert_eq![
            Feedback::Ok("<string> <string>".into()),
            gsh.interpret_single("autocomplete set").unwrap()
        ];

        assert_eq![
            Feedback::Err("Finalizer does not exist".into()),
            gsh.interpret_single("log").unwrap()
        ];
        assert_eq![
            Feedback::Err("Expected <u8>. Decider: got string: -1".into()),
            gsh.interpret_single("log context gsh level -1").unwrap()
        ];
        assert_eq![
            Feedback::Err("Expected <u8>. Decider: got string: -1".into()),
            gsh.interpret_single("log context gsh level (+ 1 2 -4)")
                .unwrap()
        ];
        assert_eq![
            Feedback::Err("Unrecognized mapping: xyz".into()),
            gsh.interpret_single("log context gsh level (+ xyz)")
                .unwrap()
        ];
        assert_eq![
            Feedback::Err("Invalid logging context".into()),
            gsh.interpret_single("log context gsh level 123").unwrap()
        ];
        assert_eq![
            Feedback::Ok("alphabetagammayotta6Hello World".into()),
            gsh.interpret_single("cat alpha beta (cat gamma yotta) (+ 1 2 3) (#Hello World)")
                .unwrap()
        ];
        assert_eq![
            Feedback::Ok("".into()),
            gsh.interpret_single("void alpha beta (cat gamma yotta) (+ 1 2 3) (#Hello World)")
                .unwrap()
        ];
    }

    #[test]
    fn check_integer_overflow() {
        let mut gsh = make_new_gameshell();

        assert_eq![
            Feedback::Err("Addition overflow".into()),
            gsh.interpret_single("+ 2147483647 1").unwrap()
        ];

        assert_eq![
            Feedback::Err("Addition overflow".into()),
            gsh.interpret_single("+ -2147483648 -1").unwrap()
        ];

        assert_eq![
            Feedback::Err("Subtraction overflow".into()),
            gsh.interpret_single("- -2147483648").unwrap()
        ];

        assert_eq![
            Feedback::Err("Subtraction overflow".into()),
            gsh.interpret_single("- -2147483647 2").unwrap()
        ];

        assert_eq![
            Feedback::Err("Multiplication overflow".into()),
            gsh.interpret_single("* 2147483647 2").unwrap()
        ];

        assert_eq![
            Feedback::Err("Division by zero".into()),
            gsh.interpret_single("/ 1 0").unwrap()
        ];

        assert_eq![
            Feedback::Err("Division overflow".into()),
            gsh.interpret_single("/ -2147483648 -1").unwrap()
        ];

        assert_eq![
            Feedback::Err("Modulo by zero".into()),
            gsh.interpret_single("% 1 0").unwrap()
        ];

        assert_eq![
            Feedback::Err("Modulo overflow".into()),
            gsh.interpret_single("% -2147483648 -1").unwrap()
        ];
    }

    // // ---

    #[bench]
    fn speed_of_interpreting_a_raw_command(b: &mut Bencher) {
        let mut gsh = make_new_gameshell();
        b.iter(|| black_box(gsh.interpret_single(black_box("void"))));
    }

    #[bench]
    fn speed_of_interpreting_a_nested_command_with_parameters(b: &mut Bencher) {
        let mut gsh = make_new_gameshell();
        b.iter(|| black_box(gsh.interpret_single(black_box("void (void 123) abc"))));
    }

    #[bench]
    fn speed_of_adding_a_bunch_of_numbers(b: &mut Bencher) {
        let mut gsh = make_new_gameshell();
        b.iter(|| black_box(gsh.interpret_single(black_box("+ 1 2 3 (- 4 5 6) (* 9 9)"))));
    }

    #[bench]
    fn message_bandwidth_over_tcp(b: &mut Bencher) -> io::Result<()> {
        let mut logger = fast_logger::Logger::spawn_void();
        let (listener, port) = bind_to_any_tcp_port();
        let gshspawn = spawn_with_listener(logger.clone(), listener, port);
        logger.set_log_level(0);
        let mut listener =
            TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;
        b.iter(|| -> io::Result<()> {
            writeln![listener, "log global level 0"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 1024])?;
            Ok(())
        });
        gshspawn.keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        std::mem::drop(logger);
        let _ = TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;
        Ok(())
    }
}
