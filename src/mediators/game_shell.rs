use crate::glocals::{GameShell, GameShellContext, Log};
use cmdmat;
use logger::{self, Logger};
use metac::{Data, Evaluate, PartialParse};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

mod predicates {
    use super::*;
    use cmdmat::{Decider, Decision};
    fn any_atom_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        for i in input[0].chars() {
            if i.is_whitespace() {
                return Decision::Deny("Expected atom, item contains whitespace".into());
            }
        }
        out[0] = Input::Atom(input[0].to_string());
        Decision::Accept(1)
    }
    fn any_string_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        out[0] = Input::String(input[0].to_string());
        Decision::Accept(1)
    }
    fn any_u8_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        out[0] = input[0].parse::<u8>().ok().map(Input::U8).unwrap();
        Decision::Accept(1)
    }
    fn any_i32_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        out[0] = input[0].parse::<i32>().ok().map(Input::I32).unwrap();
        Decision::Accept(1)
    }
    pub const ANY_ATOM: Decider<Input, String> = Decider {
        description: "<atom>",
        decider: any_atom_function,
    };
    pub const ANY_STRING: &Decider<Input, String> = &Decider {
        description: "<string>",
        decider: any_string_function,
    };
    pub const ANY_U8: &Decider<Input, String> = &Decider {
        description: "<u8>",
        decider: any_u8_function,
    };
    pub const ANY_I32: &Decider<Input, String> = &Decider {
        description: "<i32>",
        decider: any_i32_function,
    };
}

#[rustfmt::skip]
const SPEC: &[cmdmat::Spec<Input, String, GameShellContext>] = &[
    (&[("log", None), ("global", None), ("level", Some(predicates::ANY_U8))], log),
    // (&[("str", None)], create_string),
    // (&[("ex", None)], number),
    // (&[("set, None"), ("key", predicates::ANY_STRING), ("value", predicates::ANY_STRING)], do_set),
    // (&[("get, None"), ("key", predicates::ANY_STRING)], do_get),
    // (&[("void", predicates::ANY_STRING)], void),
    // (&[("+", ANY_I32)], add),
    // (&[("autocomplete", predicates::ANY_STRING)], autocomplete),
    // (&[("log"), ("trace", predicates::ANY_STRING)], log_trace),
    // (&[("log", None), ("context", ANY_ATOM), ("level", ANY_U8)], log_context),
];

pub fn make_new_gameshell(logger: Logger<Log>) -> Gsh<'static> {
    let keep_running = Arc::new(AtomicBool::new(true));
    let mut cmdmat = cmdmat::Mapping::new();
    cmdmat.register_many(SPEC);
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

pub fn spawn(logger: Logger<Log>) -> (JoinHandle<()>, Arc<AtomicBool>) {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    (
        thread::Builder::new()
            .name("gsh/server".to_string())
            .spawn(move || {
                let mut cmdmat = cmdmat::Mapping::new();
                cmdmat.register_many(SPEC);
                game_shell_thread(GameShell {
                    gshctx: GameShellContext {
                        config_change: None,
                        logger,
                        keep_running,
                        variables: HashMap::new(),
                    },
                    commands: Arc::new(cmdmat),
                })
            })
            .unwrap(),
        keep_running_clone,
    )
}

// ---

fn clone_and_spawn_connection_handler(s: &Gsh, stream: TcpStream) -> JoinHandle<()> {
    let logger = s.gshctx.logger.clone();
    let keep_running = s.gshctx.keep_running.clone();
    thread::spawn(move || {
        let mut cmdmat = cmdmat::Mapping::new();
        cmdmat.register_many(SPEC);
        let mut shell_clone = GameShell {
            gshctx: GameShellContext {
                config_change: None,
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
                    .debug("gsh", Log::Static("Connection ended ok"));
            }
            Err(error) => {
                shell_clone.gshctx.logger.debug(
                    "gsh",
                    Log::StaticDynamic("Connection errored out", "reason", format!["{:?}", error]),
                );
            }
        }
    })
}

fn connection_loop(s: &mut Gsh, mut stream: TcpStream) -> io::Result<()> {
    s.gshctx.logger.debug("gsh", Log::Static("Acquired new stream"));
    const BUFFER_SIZE: usize = 2048;
    let mut buffer = [0; BUFFER_SIZE];
    let mut begin = 0;
    let mut shift = 0;
    let mut partial_parser = PartialParse::default();
    'receiver: loop {
        for (base, idx) in (shift..begin).enumerate() {
            buffer[base] = buffer[idx];
        }
        s.gshctx.logger.trace(
            "gsh",
            Log::Usize2("Loop entry", "shift", shift, "begin", begin),
        );
        begin -= shift;
        s.gshctx.logger
            .trace("gsh", Log::Usize("Loop entry (new)", "begin", begin));
        if begin > 0 {
            match from_utf8(&buffer[0..begin]) {
                Ok(x) => {
                    s.gshctx.logger.trace(
                        "gsh",
                        Log::StaticDynamic(
                            "Buffer contents from partial parse",
                            "buffer",
                            x.into(),
                        ),
                    );
                }
                Err(error) => {
                    s.gshctx.logger.error(
                        "gsh",
                        Log::StaticDynamic(
                            "Shift buffer contains invalid UTF-8",
                            "error",
                            format!["{}", error],
                        ),
                    );
                    break 'receiver;
                }
            }
        }
        shift = 0;
        if begin == BUFFER_SIZE - 1 {
            s.gshctx.logger.warn(
                "gsh",
                Log::Usize(
                    "Message exceeds maximum length, disconnecting to prevent further messages",
                    "max",
                    BUFFER_SIZE,
                ),
            );
            write![stream, "Response: Message exceeds maximum length, disconnecting to prevent further messages, max={}", BUFFER_SIZE]?;
            break 'receiver;
        }
        let count = stream.read(&mut buffer[begin..])?;
        if count == 0 {
            s.gshctx.logger.info(
                "gsh",
                Log::Static("Received empty message from farend, connection forfeit"),
            );
            break 'receiver;
        }
        s.gshctx.logger
            .trace("gsh", Log::Usize("Message from farend", "length", count));
        for ch in buffer[begin..(begin + count)].iter() {
            begin += 1;
            match partial_parser.parse_increment(*ch) {
                Some(true) => {
                    shift = begin;
                    let string = from_utf8(&buffer[(begin - shift)..begin]);
                    if let Ok(string) = string {
                        s.gshctx.logger.debug(
                            "gsh",
                            Log::StaticDynamic(
                                "Converted farend message to UTF-8, calling interpret",
                                "content",
                                string.into(),
                            ),
                        );
                        let result = s.interpret_single(string);
                        if let Ok(result) = result {
                            s.gshctx.logger.debug(
                                "gsh",
                                Log::Static(
                                    "Message parsing succeeded and evaluated, sending response to client",
                                ),
                            );
                            if !result.is_empty() {
                                stream.write_all(result.as_bytes())?;
                            } else {
                                stream.write_all(b"OK")?;
                            }
                            stream.flush()?;
                        } else {
                            s.gshctx.logger.error("gsh", Log::Static("Message parsing failed"));
                            stream.write_all(b"Unable to complete query")?;
                            stream.flush()?;
                        }
                    } else {
                        s.gshctx.logger
                            .warn("gsh", Log::Static("Malformed UTF-8 received, this should never happen. Ending connection"));
                        break 'receiver;
                    }
                }
                Some(false) => {
                    // Do nothing
                }
                None => {
                    // Set the shift register = begin, this means that all bytes so far will
                    // not be used to interpret a command. They will instead be overwritten.
                    shift = begin;
                }
            }
        }
    }
    Ok(())
}

fn game_shell_thread(mut s: Gsh) {
    let listener = TcpListener::bind("127.0.0.1:32931");
    match listener {
        Ok(listener) => {
            s.gshctx.logger
                .info("gsh", Log::Static("Started GameShell server"));
            'outer_loop: loop {
                for stream in listener.incoming() {
                    if !s.gshctx.keep_running.load(Ordering::Acquire) {
                        s.gshctx.logger
                            .info("gsh", Log::Static("Stopped GameShell server"));
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
        Err(error) => {
            s.gshctx.logger.error(
                "gsh",
                Log::StaticDynamic(
                    "Unable to start gameshell",
                    "reason",
                    format!["{:?}", error],
                ),
            );
        }
    }
}

// ---

use self::command_handlers::*;

type Gsh<'a> = GameShell<Arc<cmdmat::Mapping<'a, Input, String, GameShellContext>>>;
#[derive(Clone)]
pub enum Input {
    U8(u8),
    I32(i32),
    Atom(String),
    String(String),
    Command(String),
}

// ---

impl<'a> Evaluate<String> for Gsh<'a> {
    fn evaluate(&mut self, commands: &[Data]) -> String {
        use cmdmat::LookError;
        let mut stack = [Input::U8(0)];
        let mut content = Vec::new();
        for cmd in commands {
            content.push(cmd.content());
        }
        let res = self.commands.lookup(&content[..], &mut stack);
        match res {
            Ok(fin) => {
                fin.0(&mut self.gshctx, &stack[..fin.1]);
            }
            Err(LookError::DeciderAdvancedTooFar) => {
            }
            Err(LookError::DeciderDenied(_decider)) => {
            }
            Err(LookError::FinalizerDoesNotExist) => {
            }
            Err(LookError::UnknownMapping) => {
            }
        }
        "".into()
    }
}

// ---

mod command_handlers {
    use super::*;

//     pub fn unrecognized_command(_: &mut Gsh, _: &[Input]) -> String {
//         "Command not finished".into()
//     }

//     pub fn void(_: &mut Gsh, _: &[Input]) -> String {
//         "".into()
//     }

//     pub fn add(_: &mut Gsh, commands: &[Input]) -> String {
//         let mut sum = 0;
//         for cmd in commands {
//             match cmd {
//                 Input::I32(x) => {
//                     sum += x;
//                 }
//                 _ => {
//                     return "Expected i32".into();
//                 }
//             }
//         }
//         sum.to_string()
//     }

//     pub fn do_get(gsh: &mut Gsh, commands: &[Input]) -> String {
//         let key;
//         match commands[0] {
//             Input::String(ref string) => {
//                 key = string.clone();
//             }
//             _ => {
//                 return "F".into();
//             }
//         }
//         if let Some(string) = gsh.variables.get(&key) {
//             string.clone()
//         } else {
//             "Does not exist".into()
//         }
//     }

//     pub fn do_set(gsh: &mut Gsh, commands: &[Input]) -> String {
//         let (key, value);
//         match commands[0] {
//             Input::String(ref string) => {
//                 key = string.clone();
//             }
//             _ => {
//                 return "F".into();
//             }
//         }
//         match commands[1] {
//             Input::String(ref string) => {
//                 value = string.clone();
//             }
//             _ => {
//                 return "F".into();
//             }
//         }
//         gsh.variables.insert(key, value);
//         "OK".into()
//     }

//     pub fn create_string(_: &mut Gsh, commands: &[Input]) -> String {
//         if commands.len() != 1 {
//             return "Did not get command".into();
//         }
//         match commands[0] {
//             Input::Command(ref cmd) => cmd.clone(),
//             _ => "Error: Not a command".into(),
//         }
//     }

//     pub fn autocomplete(s: &mut Gsh, commands: &[Input]) -> String {
//         let mut nesthead = s.commands.head.clone();
//         let mut waspred = false;
//         let mut predname = "";
//         let mut recur = false;
//         for cmd in commands {
//             if waspred {
//                 waspred = recur;
//                 continue;
//             }
//             match cmd {
//                 Input::String(string) => match nesthead.clone().get(&string[..]) {
//                     Some((x, Either::Left(nest))) => {
//                         nesthead = nest.head.clone();
//                         match x {
//                             X::Atom(_) => {
//                                 waspred = false;
//                             }
//                             X::Macro(_) => {
//                                 waspred = false;
//                             }
//                             X::Predicate(_, (n, _)) => {
//                                 waspred = true;
//                                 predname = n;
//                             }
//                             X::Recurring(_, (n, _)) => {
//                                 waspred = true;
//                                 predname = n;
//                                 recur = true;
//                             }
//                         }
//                     }
//                     Some((x, Either::Right(_))) => match x {
//                         X::Atom(_) => {
//                             waspred = false;
//                         }
//                         X::Macro(_) => {
//                             waspred = false;
//                         }
//                         X::Predicate(_, (n, _)) => {
//                             waspred = true;
//                             predname = n;
//                         }
//                         X::Recurring(_, (n, _)) => {
//                             waspred = true;
//                             predname = n;
//                             recur = true;
//                         }
//                     },
//                     None => {
//                         return "Exceeded command parameter count".into();
//                     }
//                 },
//                 _ => {
//                     unreachable![];
//                 }
//             }
//         }
//         if waspred {
//             predname.into()
//         } else {
//             format!["{:?}", nesthead.keys()]
//         }
//     }

    pub fn log(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        match commands[0] {
            Input::U8(level) => {
                s.logger.set_log_level(level);
                Ok("OK: Changed log level".into())
            }
            _ => Err("Usage: log level <u8>".into()),
        }
    }

//     pub fn number(_: &mut Gsh, _: &[Input]) -> String {
//         "0".into()
//     }

//     pub fn log_trace(s: &mut Gsh, commands: &[Input]) -> String {
//         let mut sum = String::new();
//         for (idx, cmd) in commands.iter().enumerate() {
//             match cmd {
//                 Input::String(ref string) => {
//                     if idx + 1 < commands.len() && idx != 0 {
//                         sum.push(' ');
//                     }
//                     sum += string;
//                 }
//                 _ => return "Error".into(),
//             }
//         }
//         s.gshctx.logger.trace("user", Log::Dynamic(sum));
//         "OK".into()
//     }

//     pub fn log_context(s: &mut Gsh, commands: &[Input]) -> String {
//         let ctx;
//         match commands[0] {
//             Input::Atom(ref context) => {
//                 ctx = match &context[..] {
//                     "cli" => "cli",
//                     "trace" => "trace",
//                     "gsh" => "gsh",
//                     "benchmark" => "benchmark",
//                     "logger" => "logger",
//                     _ => return "Invalid logging context".into(),
//                 };
//             }
//             _ => return "Usage: log context <atom> level <u8>".into(),
//         }
//         match commands[1] {
//             Input::U8(level) => {
//                 s.gshctx.logger.set_context_specific_log_level(ctx, level);
//                 "OK: Changed log level".into()
//             }
//             _ => "Usage: log context <atom> level <u8>".into(),
//         }
//     }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read, Write};
    use std::net::TcpStream;
    use std::sync::atomic::Ordering;
    use test::{black_box, Bencher};

    #[test]
    #[cfg(test_nondeterministic)]
    fn nondeterministic_change_log_level() -> io::Result<()> {
        let (logger, logger_handle) = logger::Logger::spawn();
        assert_ne![123, logger.get_log_level()];
        let (_gsh, keep_running) = spawn(logger.clone());
        std::thread::sleep(std::time::Duration::new(0, 50_000_000));
        {
            let mut listener = TcpStream::connect("127.0.0.1:32931")?;
            writeln![listener, "log global level 123"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 256])?;
        }
        assert_eq![123, logger.get_log_level()];
        keep_running.store(false, Ordering::Release);
        std::mem::drop(logger);
        let listener = TcpStream::connect("127.0.0.1:32931")?;
        logger_handle.join().unwrap();
        Ok(())
    }

    #[test]
    fn check_variable_statements() -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
            logger.set_log_level(0);
            let keep_running = Arc::new(AtomicBool::new(true));
            let mut cmdmat = cmdmat::Mapping::new();
            cmdmat.register_many(SPEC);
            let mut gsh = GameShell {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
                commands: Arc::new(cmdmat),
            };

            assert_eq![
                "OK",
                gsh.interpret_single("set key lorem value ipsum").unwrap()
            ];
            assert_eq![
                "ipsum",
                gsh.interpret_single("get key lorem")
                    .unwrap()
            ];

            // then
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[test]
    fn check_idempotent_statements_work() -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
            logger.set_log_level(0);
            let keep_running = Arc::new(AtomicBool::new(true));
            let mut cmdmat = cmdmat::Mapping::new();
            cmdmat.register_many(SPEC);
            let mut gsh = GameShell {
                config_change: None,
                logger,
                keep_running,
                variables: HashMap::new(),
                commands: Arc::new(cmdmat),
            };

            assert_eq![
                "Unrecognized command",
                gsh.interpret_single("hello world").unwrap()
            ];
            assert_eq![
                "some thing\n new ",
                gsh.interpret_single("str (some thing\n new )").unwrap()
            ];
            assert_eq!["6", gsh.interpret_single("+ 1 2 3").unwrap()];
            assert_eq!["21", gsh.interpret_single("+ 1 (+ 8 9) 3").unwrap()];
            assert_eq!["21", gsh.interpret_single("+ 1 (+ 8 (+) 9) 3").unwrap()];
            assert_eq!["22", gsh.interpret_single("+ 1 (+ 8 (+ 1) 9) 3").unwrap()];
            assert_eq![
                "",
                gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 9) 3\nvoid").unwrap()
            ];
            assert_eq![
                "Expected: <i32>, but got: \"Expected: <i32>, but got: \\\"0.6\\\"\"",
                gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 0.6 9) (+ 3\n1\n)")
                    .unwrap()
            ];
            assert_eq![
                "<atom>",
                gsh.interpret_single("autocomplete log context").unwrap()
            ];
            assert_eq![
                "<u8>",
                gsh.interpret_single("autocomplete log context gsh level ")
                    .unwrap()
            ];

            // then
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    // ---

    #[bench]
    fn speed_of_interpreting_a_raw_command(b: &mut Bencher) -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
            logger.set_log_level(0);
            let keep_running = Arc::new(AtomicBool::new(true));
            let mut cmdmat = cmdmat::Mapping::new();
            cmdmat.register_many(SPEC);
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
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[bench]
    fn speed_of_interpreting_a_nested_command_with_parameters(b: &mut Bencher) -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
            logger.set_log_level(0);
            let keep_running = Arc::new(AtomicBool::new(true));
            let mut cmdmat = cmdmat::Mapping::new();
            cmdmat.register_many(SPEC);
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
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[bench]
    fn message_bandwidth_over_tcp(b: &mut Bencher) -> io::Result<()> {
        let (mut logger, logger_handle) = logger::Logger::spawn();
        let (mut _gsh, keep_running) = spawn(logger.clone());
        std::thread::sleep(std::time::Duration::new(0, 50_000_000));
        logger.set_log_level(0);
        let mut listener = TcpStream::connect("127.0.0.1:32931")?;
        b.iter(|| -> io::Result<()> {
            writeln![listener, "log global level 0"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 1024])?;
            Ok(())
        });
        keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        std::mem::drop(logger);
        let _ = TcpStream::connect("127.0.0.1:32931")?;
        let _ = logger_handle.join().unwrap();
        Ok(())
    }
}
