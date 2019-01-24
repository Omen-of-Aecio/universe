use crate::glocals::{GameShell, Log};
use crate::libs::{logger::Logger, metac::{Data, Evaluate}};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::thread::{self, JoinHandle};

pub fn spawn(logger: Logger<Log>) -> JoinHandle<()> {
    thread::spawn(move || game_shell_thread(GameShell {
        logger,
    }))
}

fn connection_loop(s: &mut GameShell, mut stream: TcpStream) -> io::Result<()> {
    s.logger.debug(Log::Static("gsh: Acquired new stream"));
    const BUFFER_SIZE: usize = 129;
    let mut buffer = [0; BUFFER_SIZE];
    'receiver: loop {
        let read_count = stream.read(&mut buffer);
        s.logger.debug(Log::Static("gsh: Received message from farend"));
        if let Ok(count) = read_count {
            if count == BUFFER_SIZE {
                s.logger.debug(Log::Usize("gsh: Message exceeds maximum length, disconnecting to prevent further messages", "max", BUFFER_SIZE-1));
                write![stream, "Response: Message exceeds maximum length, disconnecting to prevent further messages, max={}", BUFFER_SIZE-1]?;
                break 'receiver;
            }
            s.logger.debug(Log::Usize("gsh: Message from farend", "length", count));
            if count == 0 {
                break;
            }
            let string = from_utf8(&buffer[0..count]);
            if let Ok(string) = string {
                s.logger.debug(Log::Static("gsh: Converted farend message to UTF-8, calling interpret"));
                let result = s.interpret(string);
                if let Ok(result) = result {
                    s.logger.debug(Log::Static("gsh: Message parsing succeeded and evaluated, sending response to client"));
                    stream.write((String::from("Response: ") + &result).as_bytes())?;
                    stream.flush()?;
                } else {
                    s.logger.debug(Log::Static("gsh: Message parsing failed"));
                    stream.write(b"Unable to complete query")?;
                    stream.flush()?;
                }
            } else {
                s.logger.debug(Log::Static("Malformed UTF-8 received"));
            }
        } else {
            s.logger.debug(Log::StaticDynamic("gsh: Unable to read", "reason", format!["{:?}", read_count]));
            break;
        }
    }
    Ok(())
}

fn game_shell_thread(mut s: GameShell) {
    let listener = TcpListener::bind("127.0.0.1:32931");
    match listener {
        Ok(listener) => {
            s.logger.info(Log::Static("Started GameShell server"));
            loop {
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            let mut shell_clone = s.clone();
                            thread::spawn(move || {
                                let _ = connection_loop(&mut shell_clone, stream);
                            });
                        }
                        Err(error) => {
                            s.logger.error(Log::StaticDynamic("Got a stream but there was an error", "reason", format!["{:?}", error]));
                        }
                    }
                }
            }
        }
        Err(error) => {
            s.logger.error(Log::StaticDynamic("Unable to start gameshell", "reason", format!["{:?}", error]));
        }
    }
}

// ---

enum X {
    Atom(&'static str),
    Predicate(&'static str, fn(&str) -> bool),
}

fn any_u8_function(input: &str) -> bool {
    input.parse::<u8>().is_ok()
}

const any_u8: X = X::Predicate("<u8>", any_u8_function);

impl Evaluate<String> for GameShell {
    fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> String {
        let spec = [
            ([X::Atom("log"), X::Atom("level"), any_u8], GameShell::log)
        ];

        fn samplify(xs: &[X]) -> String {
            let mut string = String::new();
            for x in xs {
                match x {
                    X::Atom(sample) => string += sample,
                    X::Predicate(name, _) => string += name,
                }
            }
            string
        }

        let mut args = vec![];

        'outer: for command in spec.iter() {
            args.clear();
            if command.0.len() == commands.len() {
                for (idx, part) in command.0.iter().enumerate() {
                    match part {
                        X::Atom(string) => {
                            match commands[idx] {
                                Data::Atom(other) => {
                                    if string == &other {
                                        // OK
                                    } else {
                                        continue 'outer;
                                    }
                                }
                                _ => {
                                    continue 'outer;
                                }
                            }
                        }
                        X::Predicate(desc, predf) => {
                            match commands[idx] {
                                Data::Atom(other) => {
                                    if predf(&other) {
                                        // OK
                                        args.push(Data::Atom(other));
                                    } else {
                                        return format!["Expected {}", desc];
                                    }
                                }
                                _ => {
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }
                // We have gotten here, so the command matches.
                return command.1(self, &args);
            }
        }
        return format!["Unknown command"];
    }
}

// ---

impl GameShell {
    fn log<'a>(&mut self, commands: &[Data<'a>]) -> String {
        match commands[0] {
            Data::Atom(number) => {
                let value = number.parse::<u8>();
                if let Ok(value) = value {
                    self.logger.set_log_level(value);
                    "OK: Changed log level".into()
                } else {
                    self.logger.info(Log::Dynamic(String::from("|") + number.into() + "|"));
                    "Err: Unable to parse number".into()
                }
            }
            _ => {
                "Usage: log level <u8>".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_u8() {
        "10".parse::<u8>().unwrap();
    }
}
