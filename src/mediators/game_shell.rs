use crate::glocals::{GameShell, Log};
use crate::libs::{logger::Logger, metac::{Data, Evaluate}};
use std::io::Read;
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
            let mut buffer = [0; 128];
            stream.read(&mut buffer);
            let string = from_utf8(&buffer[..]);
            if let Ok(string) = string {
                s.logger.info(Log::Dynamic(string.into()));
                s.interpret(string);
            } else {
                s.logger.warn(Log::Static("Malformed data from client"));
            }
        }
    }
}

// ---

impl Evaluate<String> for GameShell {
    fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> String {
        match commands[0] {
            Data::Atom("log") => {
                self.log(&commands[1..]);
            }
            _ => {}
        }
        "".into()
    }
}

// ---

impl GameShell {
    fn log<'a>(&mut self, commands: &[Data<'a>]) -> String {
        match commands[0] {
            Data::Atom(string) => {
                self.logger.info(Log::Dynamic(string.into()));
            }
            _ => {}
        }
        "".into()
    }
}
