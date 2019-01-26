use crate::glocals::{GameShell, Log};
use crate::libs::{
    logger::Logger,
    metac::{Data, Evaluate},
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

pub fn spawn(logger: Logger<Log>) -> (JoinHandle<()>, Arc<AtomicBool>) {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    (
        thread::spawn(move || {
            let mut nest = Nest::new();
            for spell in SPEC {
                build_nest(&mut nest, spell.0, spell.1);
            }
            game_shell_thread(GameShell {
                logger,
                keep_running,
                commands: Arc::new(nest),
            })
        }),
        keep_running_clone,
    )
}

// ---

fn clone_and_spawn_connection_handler(s: &Gsh, stream: TcpStream) -> JoinHandle<()> {
    let mut logger = s.logger.clone();
    let keep_running = s.keep_running.clone();
    thread::spawn(move || {
        let mut nest = Nest::new();
        for spell in SPEC {
            logger.info("gsh", Log::Static("Building nest..."));
            build_nest(&mut nest, spell.0, spell.1);
        }
        let mut shell_clone = GameShell {
            logger,
            keep_running,
            commands: Arc::new(nest),
        };
        let result = connection_loop(&mut shell_clone, stream);
        match result {
            Ok(()) => {
                shell_clone
                    .logger
                    .debug("gsh", Log::Static("Connection ended ok"));
            }
            Err(error) => {
                shell_clone.logger.debug(
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
}

fn connection_loop<'a>(s: &mut Gsh<'a>, mut stream: TcpStream) -> io::Result<()> {
    s.logger.debug("gsh", Log::Static("Acquired new stream"));
    const BUFFER_SIZE: usize = 129;
    let mut buffer = [0; BUFFER_SIZE];
    'receiver: loop {
        let read_count = stream.read(&mut buffer);
        s.logger
            .debug("gsh", Log::Static("Received message from farend"));
        if let Ok(count) = read_count {
            if count == BUFFER_SIZE {
                s.logger.debug(
                    "gsh",
                    Log::Usize(
                        "Message exceeds maximum length, disconnecting to prevent further messages",
                        "max",
                        BUFFER_SIZE - 1,
                    ),
                );
                write![stream, "Response: Message exceeds maximum length, disconnecting to prevent further messages, max={}", BUFFER_SIZE-1]?;
                break 'receiver;
            }
            s.logger
                .debug("gsh", Log::Usize("Message from farend", "length", count));
            if count == 0 {
                break;
            }
            let string = from_utf8(&buffer[0..count]);
            if let Ok(string) = string {
                s.logger.debug(
                    "gsh",
                    Log::Static("Converted farend message to UTF-8, calling interpret"),
                );
                let result = s.interpret_single(string);
                if let Ok(result) = result {
                    s.logger.debug(
                        "gsh",
                        Log::Static(
                            "Message parsing succeeded and evaluated, sending response to client",
                        ),
                    );
                    stream.write((String::from("Response: ") + &result).as_bytes())?;
                    stream.flush()?;
                } else {
                    s.logger.debug("gsh", Log::Static("Message parsing failed"));
                    stream.write(b"Unable to complete query")?;
                    stream.flush()?;
                }
            } else {
                s.logger
                    .debug("gsh", Log::Static("Malformed UTF-8 received"));
            }
        } else {
            s.logger.debug(
                "gsh",
                Log::StaticDynamic("Unable to read", "reason", format!["{:?}", read_count]),
            );
            break;
        }
    }
    Ok(())
}

fn game_shell_thread<'a>(mut s: Gsh<'a>) {
    let listener = TcpListener::bind("127.0.0.1:32931");
    match listener {
        Ok(listener) => {
            s.logger
                .info("gsh", Log::Static("Started GameShell server"));
            'outer_loop: loop {
                for stream in listener.incoming() {
                    if !s.keep_running.load(Ordering::Relaxed) {
                        break 'outer_loop;
                    }
                    match stream {
                        Ok(stream) => {
                            clone_and_spawn_connection_handler(&s, stream);
                        }
                        Err(error) => {
                            s.logger.error(
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
            s.logger.error(
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

type Gsh<'a> = GameShell<Arc<Nest<'a>>>;
type Pred<'a> = (&'a str, fn(&str) -> Option<Input>);
#[derive(Clone, Copy)]
enum X<'a> {
    Atom(&'a str),
    Predicate(&'a str, Pred<'a>),
}

impl<'a> X<'a> {
    fn name(&self) -> &'a str {
        match self {
            X::Atom(name) => name,
            X::Predicate(name, _) => name,
        }
    }
}
impl<'a> std::fmt::Debug for X<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            X::Atom(atom) => {
                write![f, "X::Atom({})", atom]
            }
            X::Predicate(descriptor, _) => {
                write![f, "X::Predicate({}, <function>)", descriptor]
            }
        }
    }
}

fn any_u8_function(input: &str) -> Option<Input> {
    input.parse::<u8>().ok().map(|x| Input::U8(x))
}

fn any_atom_function(input: &str) -> Option<Input> {
    for i in input.chars() {
        if i.is_whitespace() {
            return None;
        }
    }
    Some(Input::Atom(input.into()))
}
const any_u8: Pred = ("<u8>", any_u8_function);
const any_atom: Pred = ("<atom>", any_atom_function);

type Fun<'a> = fn(&mut Gsh<'a>, &[Input]) -> String;
type Ether<'a> = Either<Nest<'a>, Fun<'a>>;

impl<'a> std::fmt::Debug for Ether<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Either::Left(atom) => {
                write![f, "Nest {:?}", atom]
            }
            Either::Right(atom) => {
                write![f, "Right <function>"]
            }
        }
    }
}

#[derive(Clone)]
enum Either<L: Clone, R: Clone> {
    Left(L),
    Right(R),
}
#[derive(Clone, Debug)]
struct Nest<'a> {
    pub head: HashMap<&'a str, (X<'a>, Ether<'a>)>,
}
impl<'a> Nest<'a> {
    fn new() -> Self {
        Self {
            head: HashMap::new(),
        }
    }
}

const SPEC: &[(&[X], Fun)] = &[
    (
        &[X::Atom("log"), X::Atom("global"), X::Predicate("level", any_u8)],
        log,
    ),
    (
        &[X::Atom("ex")], number
    ),
    (
       &[ 
            X::Atom("log"),
            X::Predicate("context", any_atom),
            X::Predicate("level", any_u8),
        ],
        log_context,
    ),
];

fn build_nest<'a>(nest: &mut Nest<'a>, commands: &'a [X], handler: Fun<'a>) -> Option<Ether<'a>> {
    if commands.len() != 0 {
        // Does the nest already contain this command?
        if nest.head.get_mut(&commands[0].name()).is_some() {
            match nest.head.get_mut(&commands[0].name()) {
                Some((_, Either::Left(subnest))) => {
                    let ether = build_nest(subnest , &commands[1..], handler);
                    if let Some(ether) = ether {
                        subnest.head.insert(commands[0].name(), (commands[0], ether));
                    }
                    None
                }
                Some((_, Either::Right(handler))) => {
                    unreachable![];
                }
                None => {
                    unreachable![];
                }
            }
        } else {
            let mut ether = Nest::new();
            let result = build_nest(&mut ether, &commands[1..], handler);
            if result.is_some() {
                nest.head.insert(commands[0].name(), (commands[0].clone(), result.unwrap()));
            } else {
                nest.head.insert(commands[0].name(), (commands[0].clone(), Either::Left(ether)));
            }
            None
        }
    } else {
        Some(Either::Right(handler))
    }
}

#[derive(Clone)]
enum Input {
    U8(u8),
    Atom(String),
}

// ---

impl<'a> Evaluate<String> for Gsh<'a> {
    fn evaluate<'b>(&mut self, commands: &[Data<'b>]) -> String {

        let mut args: Vec<Input> = vec![];

        let mut nest = self.commands.head.clone();
        let mut to_handle: Fun = unrecognized_command;
        let mut to_predicate: Option<(&str, &str, fn(&str) -> Option<Input>)> = None;

        // Note change format to HashMap<&'a str, (X, Ether<'a>)>
        // So we can avoid get_key_value (and nightly)
        // and we get a more sane result from the table, also we don't need to create Hash,
        // PartialEq, and so on, those feel like hacks in this case.
        for c in commands {
            match c {
                Data::Atom(atom) => {
                    if let Some((name, desc, pred)) = to_predicate {
                        let result = pred(atom);
                        // self.logger.debug("gsh", Log::Bool("Predicate evaluated", "result", result));
                        if let Some(result) = result {
                            to_predicate = None;
                            args.push(result.clone());
                        } else {
                            return format!["Expected: {}, but got: {:?}", desc, atom];
                        }
                    } else {
                        match nest.get(atom) {
                            Some((x, up_next)) => {
                                match x {
                                    X::Atom(_) => {
                                        self.logger.debug("gsh", Log::StaticDynamic("Found command", "entry", (*atom).into()));
                                    }
                                    X::Predicate(name, (desc, pred)) => {
                                        self.logger.debug("gsh", Log::StaticDynamic("Found predicate", "entry", (*atom).into()));
                                        to_predicate = Some((name, desc, *pred));
                                    }
                                }
                                match up_next {
                                    Either::Left(next) => {
                                        nest = next.head.clone();
                                    }
                                    Either::Right(handler) => {
                                        to_handle = *handler;
                                    }
                                }
                            }
                            None => {
                                return format!["Unrecognized command"];
                            }
                        }
                    }
                }
                Data::Command(cmd) => {
                    self.logger.debug("gsh", Log::StaticDynamic("CMD", "input", (*cmd).into()));
                    let result = match self.interpret_single(cmd) {
                        Ok(result) => result,
                        Err(error) => return format!["Error parsing: {}", cmd],
                    };
                    let atom: &str = &result;
                    if let Some((name, desc, pred)) = to_predicate {
                        let result = pred(atom);
                        // self.logger.debug("gsh", Log::Bool("Predicate evaluated", "result", result));
                        if let Some(result) = result {
                            to_predicate = None;
                            args.push(result.clone());
                        } else {
                            return format!["Expected: {}, but got: {:?}", desc, atom];
                        }
                    } else {
                        match nest.get(atom) {
                            Some((x, up_next)) => {
                                match x {
                                    X::Atom(_) => {
                                        self.logger.debug("gsh", Log::StaticDynamic("Found command", "entry", (*atom).into()));
                                    }
                                    X::Predicate(name, (desc, pred)) => {
                                        self.logger.debug("gsh", Log::StaticDynamic("Found predicate", "entry", (*atom).into()));
                                        to_predicate = Some((name, desc, *pred));
                                    }
                                }
                                match up_next {
                                    Either::Left(next) => {
                                        nest = next.head.clone();
                                    }
                                    Either::Right(handler) => {
                                        to_handle = *handler;
                                    }
                                }
                            }
                            None => {
                                return format!["Unrecognized command"];
                            }
                        }
                    }
                }
            }
        }

        if let Some((name, desc, _)) = to_predicate {
            return format!["Missing predicate value: {} of type: {}", name, desc];
        }

        to_handle(self, &args[..])
    }
}

// ---

fn unrecognized_command<'a>(s: &mut Gsh<'a>, commands: &[Input]) -> String {
    "Command not found".into()
}

fn log<'a>(s: &mut Gsh<'a>, commands: &[Input]) -> String {
    match commands[0] {
        Input::U8(level) => {
            s.logger.set_log_level(level);
            "OK: Changed log level".into()
        }
        _ => "Usage: log level <u8>".into(),
    }
}

fn number(s: &mut Gsh, commands: &[Input]) -> String {
    "0".into()
}

fn log_context<'a>(s: &mut Gsh<'a>, commands: &[Input]) -> String {
    let ctx;
    match commands[0] {
        Input::Atom(ref context) => {
            ctx = match &context[..] {
                "cli" => "cli",
                "gsh" => "gsh",
                "benchmark" => "benchmark",
                "logger" => "logger",
                _ => return "Invalid logging context".into(),
            };
        }
        _ => return "Usage: log context <atom> level <u8>".into(),
    }
    match commands[1] {
        Input::U8(level) => {
            s.logger.set_context_specific_log_level(ctx, level);
            "OK: Changed log level".into()
        }
        _ => "Usage: log context <atom> level <u8>".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u8() {
        "10".parse::<u8>().unwrap();
    }
}
