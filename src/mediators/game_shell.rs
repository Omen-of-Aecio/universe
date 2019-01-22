use crate::glocals::{GameShell, Log};
use crate::libs::{logger::Logger, metac::{Data, Evaluate}};
use std::thread::{self, JoinHandle};

pub fn spawn(logger: Logger<Log>) -> JoinHandle<()> {
    let shell = Shell { };
    thread::spawn(move || game_shell_thread(GameShell { logger, evaluator: shell }))
}

fn game_shell_thread(mut s: GameShell<Shell>) {
    s.logger.info(Log::Static("Started GameShell"));
    loop {
        // Read from socket
        // Interpret each line
        // That's it lol
    }
}

// ---

struct Shell { }

impl Evaluate<String> for Shell {
    fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> String {
        "".into()
    }
}

// ---
