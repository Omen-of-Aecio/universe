use crate::glocals::{GameShell, Log};
use crate::libs::{logger::Logger, metac::{Data, Evaluate}};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::thread::{self, JoinHandle};

pub fn spawn(logger: Logger<Log>) -> JoinHandle<()> {
    thread::spawn(move || game_shell_thread(GameShell { logger }))
}

fn game_shell_thread(mut s: GameShell) {
    s.logger.info(Log::Static("Started GameShell"));
    let listener = TcpListener::bind("127.0.0.1:32931").unwrap();
    loop {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            s.logger.info(Log::Static("new stream"));
            let mut buffer = [0; 128];
            loop {
                s.logger.info(Log::Static("READING ST"));
                let read_count = stream.read(&mut buffer);
                s.logger.info(Log::Dynamic(format!["Attr{:?}", read_count]));
                if let Ok(count) = read_count {
                    if count == 0 {
                        break;
                    }
                    let string = from_utf8(&buffer[0..count]);
                    if let Ok(string) = string {
                        s.logger.info(Log::Static("BOUTA INTERPRET"));
                        s.logger.info(Log::Dynamic(string.into()));
                        let result = s.interpret(string);
                        s.logger.info(Log::Static("interpret done"));
                        if let Ok(result) = result {
                            s.logger.info(Log::Static("After OK RES"));
                            stream.write(result.as_bytes());
                            s.logger.info(Log::Static("After OK RES w"));
                            stream.flush();
                        } else {
                            stream.write(b"Unable to complete query");
                            stream.flush();
                        }
                    } else {
                        s.logger.warn(Log::Static("Malformed data from client"));
                    }
                } else {
                    s.logger.info(Log::Static("Going into break"));
                    break;
                }
            }
        }
    }
}

// ---

impl Evaluate<String> for GameShell {
    fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> String {
        match commands[0] {
            Data::Atom("log") => {
                self.log(&commands[1..])
            }
            _ => {
                "".into()
            }
        }
    }
}

// ---

impl GameShell {
    fn log<'a>(&mut self, commands: &[Data<'a>]) -> String {
        match commands[0] {
            Data::Atom("level") => {
                if commands.len() == 2 {
                    match commands[1] {
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
                } else {
                    "Usage: log level <u8>".into()
                }
            }
            Data::Atom(string) => {
                self.logger.info(Log::Dynamic(string.into()));
                "Unknown atom".into()
            }
            _ => {
                "Unknown command".into()
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
