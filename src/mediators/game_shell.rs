use self::{command_handlers::*, incconsumer::*, predicates::*, types::*};
use crate::glocals::{GameShell, GameShellContext, Log, Main};
use cmdmat::{self, LookError, SVec};
use either::Either;
use logger::{self, Logger};
use metac::{Data, Evaluate, ParseError, PartialParse, PartialParseOp};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};

mod command_handlers;
mod incconsumer;
mod predicates;
mod types;

// ---

#[rustfmt::skip]
const SPEC: &[cmdmat::Spec<Input, GshDecision, GameShellContext>] = &[
    (&[("%", MANY_I32)], modulo),
    (&[("&", MANY_I32)], band),
    (&[("*", MANY_I32)], mul),
    (&[("+", MANY_I32)], add),
    (&[("-", MANY_I32)], sub),
    (&[("/", MANY_I32)], div),
    (&[("^", MANY_I32)], xor),
    (&[("cat", MANY_STRING)], cat),
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

pub fn make_new_gameshell(logger: Logger<Log>) -> Gsh<'static> {
    let keep_running = Arc::new(AtomicBool::new(true));
    let mut cmdmat = cmdmat::Mapping::default();
    cmdmat.register_many(SPEC).unwrap();
    GameShell {
        gshctx: GameShellContext {
            config_change: None,
            logger,
            keep_running,
            variables: HashMap::new(),
        },
        commands: Arc::new(cmdmat),
    }
}

// ---

fn spawn_with_listener(logger: Logger<Log>, listener: TcpListener, port: u16) -> GshSpawn {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    let (tx, rx) = mpsc::sync_channel(2);
    GshSpawn {
        thread_handle: thread::Builder::new()
            .name("gsh/server".to_string())
            .spawn(move || {
                let mut cmdmat = cmdmat::Mapping::default();
                cmdmat.register_many(SPEC).unwrap();
                game_shell_thread(
                    GameShell {
                        gshctx: GameShellContext {
                            config_change: Some(tx),
                            logger,
                            keep_running,
                            variables: HashMap::new(),
                        },
                        commands: Arc::new(cmdmat),
                    },
                    listener,
                )
            })
            .unwrap(),
        keep_running: keep_running_clone,
        channel: rx,
        port,
    }
}

pub fn spawn(mut logger: Logger<Log>) -> Option<GshSpawn> {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:32931") {
        Some(spawn_with_listener(logger, listener, 32931))
    } else {
        logger.info("gsh", "Unable to bind to tcp port");
        None
    }
}

pub fn spawn_with_any_port(mut logger: Logger<Log>) -> GshSpawn {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:32931") {
        spawn_with_listener(logger, listener, 32931)
    } else {
        logger.info("gsh", "Unable to bind to tcp port");
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

fn clone_and_spawn_connection_handler(s: &Gsh, stream: TcpStream) -> JoinHandle<()> {
    let logger = s.gshctx.logger.clone();
    let keep_running = s.gshctx.keep_running.clone();
    let channel = s.gshctx.config_change.clone();
    thread::Builder::new()
        .name("gsh/server/handler".to_string())
        .spawn(move || {
            let mut cmdmat = cmdmat::Mapping::default();
            cmdmat.register_many(SPEC).unwrap();
            let mut shell_clone = GameShell {
                gshctx: GameShellContext {
                    config_change: channel,
                    logger,
                    keep_running,
                    variables: HashMap::new(),
                },
                commands: Arc::new(cmdmat),
            };
            let result = connection_loop(&mut shell_clone, stream);
            match result {
                Ok(()) => {
                    shell_clone
                        .gshctx
                        .logger
                        .debug("gsh", "Connection ended ok");
                }
                Err(error) => {
                    shell_clone.gshctx.logger.debug(
                        "gsh",
                        Log::StaticDynamic(
                            "Connection errored out",
                            "reason",
                            format!["{:?}", error],
                        ),
                    );
                }
            }
        })
        .unwrap()
}

// ---

impl<'a, 'b> IncConsumer for GshTcp<'a, 'b> {
    fn consume(&mut self, output: &mut [u8]) -> Consumption {
        match self.stream.read(output) {
            Ok(0) => Consumption::Stop,
            Ok(count) => Consumption::Consumed(count),
            Err(_) => Consumption::Stop,
        }
    }
    fn validate(&mut self, input: u8) -> Validation {
        match self.parser.parse_increment(input) {
            PartialParseOp::Ready => Validation::Ready,
            PartialParseOp::Unready => Validation::Unready,
            PartialParseOp::Discard => Validation::Discard,
        }
    }
    fn process(&mut self, input: &[u8]) -> Process {
        let string = from_utf8(input);
        if let Ok(string) = string {
            self.gsh.gshctx.logger.debug(
                "gsh",
                Log::StaticDynamic(
                    "Converted farend message to UTF-8, calling interpret",
                    "content",
                    string.into(),
                ),
            );
            let result = self.gsh.interpret_single(string);
            if let Ok(result) = result {
                self.gsh.gshctx.logger.debug(
                    "gsh",
                    "Message parsing succeeded and evaluated, sending response to client",
                );
                match result {
                    EvalRes::Ok(res) => {
                        if !res.is_empty() {
                            if self.stream.write_all(res.as_bytes()).is_err() {
                                return Process::Stop;
                            }
                        } else if self.stream.write_all(b"Ok").is_err() {
                            return Process::Stop;
                        }
                    }
                    EvalRes::Err(res) => {
                        if self
                            .stream
                            .write_all(format!["Err: {}", res].as_bytes())
                            .is_err()
                        {
                            return Process::Stop;
                        }
                    }
                    EvalRes::Help(res) => {
                        if !res.is_empty() {
                            if self.stream.write_all(res.as_bytes()).is_err() {
                                return Process::Stop;
                            }
                        } else {
                            self.gsh
                                .gshctx
                                .logger
                                .warn("gsh", "Sending empty help message");
                            if self.stream.write_all(b"Empty help message").is_err() {
                                return Process::Stop;
                            }
                        }
                    }
                }
                if self.stream.flush().is_err() {
                    return Process::Stop;
                }
            } else {
                self.gsh
                    .gshctx
                    .logger
                    .error("gsh", "Message parsing failed");
                if self
                    .stream
                    .write_all(b"Unable to complete query (parse error)")
                    .is_err()
                {
                    return Process::Stop;
                }
                if self.stream.flush().is_err() {
                    return Process::Stop;
                }
            }
            Process::Continue
        } else {
            self.gsh.gshctx.logger.warn(
                "gsh",
                "Malformed UTF-8 received, this should never happen. Ending connection",
            );
            Process::Stop
        }
    }
}

// ---

fn connection_loop(s: &mut Gsh, stream: TcpStream) -> io::Result<()> {
    s.gshctx.logger.debug("gsh", "Acquired new stream");
    let mut gshtcp = GshTcp {
        gsh: s,
        stream,
        parser: PartialParse::default(),
    };
    gshtcp.run(2048);
    Ok(())
}

fn game_shell_thread(mut s: Gsh, listener: TcpListener) {
    s.gshctx.logger.info("gsh", "Started GameShell server");
    'outer_loop: loop {
        for stream in listener.incoming() {
            if !s.gshctx.keep_running.load(Ordering::Acquire) {
                s.gshctx.logger.info("gsh", "Stopped GameShell server");
                break 'outer_loop;
            }
            match stream {
                Ok(stream) => {
                    clone_and_spawn_connection_handler(&s, stream);
                }
                Err(error) => {
                    s.gshctx.logger.error(
                        "gsh",
                        Log::StaticDynamic(
                            "Got a stream but there was an error",
                            "reason",
                            format!["{:?}", error],
                        ),
                    );
                }
            }
        }
    }
}

// ---

#[derive(Clone, Debug, PartialEq)]
pub enum EvalRes {
    Err(String),
    Help(String),
    Ok(String),
}

impl Default for EvalRes {
    fn default() -> Self {
        EvalRes::Ok(String::default())
    }
}

fn lookerr_to_evalres(err: LookError<GshDecision>, allow_help: bool) -> EvalRes {
    match err {
        LookError::DeciderAdvancedTooFar => EvalRes::Err("Decider advanced too far".into()),
        LookError::DeciderDenied(desc, GshDecision::Err(decider)) => {
            EvalRes::Err(format!["Expected {} but got: {}", desc, decider])
        }
        LookError::DeciderDenied(desc, GshDecision::Help(help)) => {
            if allow_help {
                EvalRes::Help(help)
            } else {
                EvalRes::Err(format!["Expected {} but got denied: {}", desc, help])
            }
        }
        LookError::FinalizerDoesNotExist => EvalRes::Err("Finalizer does not exist".into()),
        LookError::UnknownMapping(token) => {
            EvalRes::Err(format!["Unrecognized mapping: {}", token])
        }
    }
}

// ---

impl<'a> Gsh<'a> {
    fn parse_subcommands(&mut self, cmds: &[Data]) -> Result<Vec<String>, EvalRes> {
        let mut content: Vec<String> = Vec::new();
        for cmd in cmds {
            match cmd {
                Data::Atom(string) => {
                    content.push((*string).into());
                }
                Data::Command(string) => {
                    if let Some('#') = string.chars().next() {
                        content.push((string[1..]).into());
                    } else {
                        let res = self.interpret_single(string);
                        match res {
                            Ok(EvalRes::Ok(string)) => {
                                content.push(string);
                            }
                            Ok(ref res @ EvalRes::Help(_)) => {
                                return Err(res.clone());
                            }
                            Ok(ref res @ EvalRes::Err(_)) => {
                                return Err(res.clone());
                            }
                            Err(ParseError::DanglingLeftParenthesis) => {
                                return Err(EvalRes::Err("Dangling left parenthesis".into()));
                            }
                            Err(ParseError::PrematureRightParenthesis) => {
                                return Err(EvalRes::Err("Right parenthesis encountered with no matching left parenthesis".into()));
                            }
                        }
                    }
                }
            }
        }
        Ok(content)
    }
}

impl<'a> Evaluate<EvalRes> for Gsh<'a> {
    fn evaluate(&mut self, commands: &[Data]) -> EvalRes {
        let content = match self.parse_subcommands(commands) {
            Ok(content) => content,
            Err(err) => return err,
        };
        let content_ref = content.iter().map(|s| &s[..]).collect::<Vec<_>>();

        if let Some(front) = content_ref.first() {
            if *front == "autocomplete" {
                match self.commands.partial_lookup(&content_ref[1..]) {
                    Ok(Either::Left(mapping)) => {
                        let mut col = mapping
                            .get_direct_keys()
                            .map(|k| {
                                let mut s = String::new() + *k.0;
                                if k.1.is_some() {
                                    s += " ";
                                }
                                s += if k.1.is_some() { k.1.unwrap() } else { "" };
                                if k.2 {
                                    s += " ";
                                }
                                s += if k.2 { "(final)" } else { "" };
                                s
                            })
                            .collect::<Vec<_>>();
                        if col.is_empty() {
                            return EvalRes::Ok("No more handlers".into());
                        } else {
                            col.sort();
                            return EvalRes::Ok(col.join(", "));
                        }
                    }
                    Ok(Either::Right(name)) => {
                        return EvalRes::Ok(name.into());
                    }
                    Err(err) => {
                        return lookerr_to_evalres(err, true);
                    }
                }
            }
        }

        let res = self.commands.lookup(&content_ref[..]);
        match res {
            Ok(fin) => {
                let res = fin.0(&mut self.gshctx, &fin.1);
                match res {
                    Ok(res) => EvalRes::Ok(res),
                    Err(res) => EvalRes::Err(res),
                }
            }
            Err(err) => lookerr_to_evalres(err, false),
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use test::{black_box, Bencher};

    // ---

    #[test]
    fn change_log_level() -> io::Result<()> {
        let logger = logger::Logger::spawn_void();
        assert_ne![123, logger.get_log_level()];
        let (listener, port) = bind_to_any_tcp_port();
        let mut gshspawn = spawn_with_listener(logger.clone(), listener, port);
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
    fn fuzzing_result_does_not_crash() -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };
        let input = "y\u{000b}1111-31492546713013106(\u{00cc}\u{00a7}121B)1\u{00f0}\u{0094}\u{00a0}\u{0080}02291\0";
        assert_eq![
            EvalRes::Err("Unrecognized mapping: Ì§121B".into()),
            gsh.interpret_single(input).unwrap()
        ];

        // cleanup
        Ok(())
    }

    #[test]
    fn check_variable_statements() -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        assert_eq![
            EvalRes::Ok("Ok".into()),
            gsh.interpret_single("set key some-value").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("some-value".into()),
            gsh.interpret_single("get key").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Unrecognized mapping: extra".into()),
            gsh.interpret_single("set key some-value extra").unwrap()
        ];

        assert_eq![
            EvalRes::Ok("Ok".into()),
            gsh.interpret_single("set a 123").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("130".into()),
            gsh.interpret_single("+ 7 (get a)").unwrap()
        ];

        // cleanup
        Ok(())
    }

    #[test]
    fn check_idempotent_statements_work() -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        assert_eq![
            EvalRes::Err("Unrecognized mapping: hello".into()),
            gsh.interpret_single("hello world").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("some thing\n new ".into()),
            gsh.interpret_single("str (#some thing\n new )").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("6".into()),
            gsh.interpret_single("+ 1 2 3").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("21".into()),
            gsh.interpret_single("+ 1 (+ 8 9) 3").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("21".into()),
            gsh.interpret_single("+ 1 (+ 8 (+) 9) 3").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("22".into()),
            gsh.interpret_single("+ 1 (+ 8 (+ 1) 9) 3").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("".into()),
            gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 9) 3\nvoid").unwrap()
        ];
        assert_eq![
            EvalRes::Err("Unrecognized mapping: 0.6".into()),
            gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 0.6 9) (+ 3\n1\n)")
                .unwrap()
        ];
        assert_eq![
            EvalRes::Err("Unrecognized mapping: undefined".into()),
            gsh.interpret_single("+ (undefined)").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("1".into()),
            gsh.interpret_single("+ (+ 1)").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("2".into()),
            gsh.interpret_single("+ (+ 1 0 0 0 0 0 0 0 0 1)").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("-3".into()),
            gsh.interpret_single("- 3").unwrap()
        ];
        assert_eq![EvalRes::Ok("0".into()), gsh.interpret_single("-").unwrap()];
        assert_eq![
            EvalRes::Ok("3".into()),
            gsh.interpret_single("- 3 0").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("6".into()),
            gsh.interpret_single("* 3 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("1".into()),
            gsh.interpret_single("/ 3 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("1".into()),
            gsh.interpret_single("% 7 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("3".into()),
            gsh.interpret_single("^ 1 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("0".into()),
            gsh.interpret_single("& 1 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("6".into()),
            gsh.interpret_single("| 4 2").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("<atom>".into()),
            gsh.interpret_single("autocomplete log context").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("<u8>".into()),
            gsh.interpret_single("autocomplete log context gsh level ")
                .unwrap()
        ];
        assert_eq![
            EvalRes::Ok("context <atom>, global, trace <string> (final)".into()),
            gsh.interpret_single("autocomplete log").unwrap()
        ];
        assert_eq![
            EvalRes::Ok("<string> <string>".into()),
            gsh.interpret_single("autocomplete set").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Finalizer does not exist".into()),
            gsh.interpret_single("log").unwrap()
        ];
        assert_eq![
            EvalRes::Err("Expected <u8> but got: -1".into()),
            gsh.interpret_single("log context gsh level -1").unwrap()
        ];
        assert_eq![
            EvalRes::Err("Expected <u8> but got: -1".into()),
            gsh.interpret_single("log context gsh level (+ 1 2 -4)")
                .unwrap()
        ];
        assert_eq![
            EvalRes::Err("Unrecognized mapping: xyz".into()),
            gsh.interpret_single("log context gsh level (+ xyz)")
                .unwrap()
        ];
        assert_eq![
            EvalRes::Ok("alphabetagammayotta6Hello World".into()),
            gsh.interpret_single("cat alpha beta (cat gamma yotta) (+ 1 2 3) (#Hello World)")
                .unwrap()
        ];
        assert_eq![
            EvalRes::Ok("".into()),
            gsh.interpret_single("void alpha beta (cat gamma yotta) (+ 1 2 3) (#Hello World)")
                .unwrap()
        ];

        // cleanup
        Ok(())
    }

    #[test]
    fn check_integer_overflow() -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        assert_eq![
            EvalRes::Err("Addition overflow".into()),
            gsh.interpret_single("+ 2147483647 1").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Addition overflow".into()),
            gsh.interpret_single("+ -2147483648 -1").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Subtraction overflow".into()),
            gsh.interpret_single("- -2147483648").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Subtraction overflow".into()),
            gsh.interpret_single("- -2147483647 2").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Multiplication overflow".into()),
            gsh.interpret_single("* 2147483647 2").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Division by zero".into()),
            gsh.interpret_single("/ 1 0").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Division overflow".into()),
            gsh.interpret_single("/ -2147483648 -1").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Modulo by zero".into()),
            gsh.interpret_single("% 1 0").unwrap()
        ];

        assert_eq![
            EvalRes::Err("Modulo overflow".into()),
            gsh.interpret_single("% -2147483648 -1").unwrap()
        ];

        // cleanup
        Ok(())
    }

    // ---

    #[bench]
    fn speed_of_interpreting_a_raw_command(b: &mut Bencher) -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        // then
        b.iter(|| black_box(gsh.interpret_single(black_box("void"))));

        // cleanup
        Ok(())
    }

    #[bench]
    fn speed_of_interpreting_a_nested_command_with_parameters(b: &mut Bencher) -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        // then
        b.iter(|| black_box(gsh.interpret_single(black_box("void (void 123) abc"))));

        // cleanup
        Ok(())
    }

    #[bench]
    fn speed_of_adding_a_bunch_of_numbers(b: &mut Bencher) -> io::Result<()> {
        // given
        let mut logger = logger::Logger::spawn_void();
        logger.set_log_level(0);
        let keep_running = Arc::new(AtomicBool::new(true));
        let mut cmdmat = cmdmat::Mapping::default();
        cmdmat.register_many(SPEC).unwrap();
        let mut gsh = GameShell {
            gshctx: GameShellContext {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
            },
            commands: Arc::new(cmdmat),
        };

        // then
        b.iter(|| black_box(gsh.interpret_single(black_box("+ 1 2 3 (- 4 5 6) (* 9 9)"))));

        // cleanup
        Ok(())
    }

    #[bench]
    fn message_bandwidth_over_tcp(b: &mut Bencher) -> io::Result<()> {
        let mut logger = logger::Logger::spawn_void();
        let (listener, port) = bind_to_any_tcp_port();
        let mut gshspawn = spawn_with_listener(logger.clone(), listener, port);
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
