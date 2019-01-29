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
                    Log::StaticDynamic("Connection errored out", "reason", format!["{:?}", error]),
                );
            }
        }
    })
}

fn connection_loop(s: &mut Gsh, mut stream: TcpStream) -> io::Result<()> {
    s.logger.debug("gsh", Log::Static("Acquired new stream"));
    const BUFFER_SIZE: usize = 513;
    let mut buffer = [0; BUFFER_SIZE];
    'receiver: loop {
        let read_count = stream.read(&mut buffer);
        s.logger
            .debug("gsh", Log::Static("Received message from farend"));
        if let Ok(count) = read_count {
            if count == 0 {
                s.logger.debug(
                    "gsh",
                    Log::Static("Received empty message from farend, connection forfeit"),
                );
                break 'receiver;
            }
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
            let string = from_utf8(&buffer[0..count]);
            if let Ok(string) = string {
                s.logger.debug(
                    "gsh",
                    Log::StaticDynamic(
                        "Converted farend message to UTF-8, calling interpret",
                        "content",
                        string.into(),
                    ),
                );
                let result = s.interpret_single(string);
                if let Ok(result) = result {
                    s.logger.debug(
                        "gsh",
                        Log::Static(
                            "Message parsing succeeded and evaluated, sending response to client",
                        ),
                    );
                    if result.len() > 0 {
                        stream.write(format!["{}", &result].as_bytes())?;
                    } else {
                        stream.write(format!["OK"].as_bytes())?;
                    }
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
                    if !s.keep_running.load(Ordering::Acquire) {
                        s.logger
                            .info("gsh", Log::Static("Stopped GameShell server"));
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

mod predicates {
    use super::*;
    fn any_atom_function(input: &str) -> Option<Input> {
        for i in input.chars() {
            if i.is_whitespace() {
                return None;
            }
        }
        Some(Input::Atom(input.into()))
    }
    fn any_string_function(input: &str) -> Option<Input> {
        Some(Input::String(input.into()))
    }
    fn any_u8_function(input: &str) -> Option<Input> {
        input.parse::<u8>().ok().map(|x| Input::U8(x))
    }
    pub const ANY_ATOM: Pred = ("<atom>", any_atom_function);
    pub const ANY_STRING: Pred = ("<string>", any_string_function);
    pub const ANY_U8: Pred = ("<u8>", any_u8_function);
}
use self::command_handlers::*;
use self::predicates::*;

#[derive(Clone)]
pub enum Either<L: Clone, R: Clone> {
    Left(L),
    Right(R),
}
type Ether<'a> = Either<Nest<'a>, Fun<'a>>;
type Fun<'a> = fn(&mut Gsh<'a>, &[Input]) -> String;
type Gsh<'a> = GameShell<Arc<Nest<'a>>>;
#[derive(Clone)]
pub enum Input {
    U8(u8),
    Atom(String),
    String(String),
}

#[derive(Clone)]
pub struct Nest<'a> {
    pub head: HashMap<&'a str, (X<'a>, Ether<'a>)>,
}
impl<'a> Nest<'a> {
    fn new() -> Self {
        Self {
            head: HashMap::new(),
        }
    }
}
type Pred<'a> = (&'a str, fn(&str) -> Option<Input>);
#[derive(Clone, Copy)]
pub enum X<'a> {
    Atom(&'a str),
    Predicate(&'a str, Pred<'a>),
    Recurring(&'a str, Pred<'a>),
}
impl<'a> X<'a> {
    fn name(&self) -> &'a str {
        match self {
            X::Atom(name) => name,
            X::Predicate(name, _) => name,
            X::Recurring(name, _) => name,
        }
    }
}

// ---

const SPEC: &[(&[X], Fun)] = &[
    (
        &[
            X::Atom("log"),
            X::Atom("global"),
            X::Predicate("level", ANY_U8),
        ],
        log,
    ),
    (&[X::Atom("ex")], number),
    (&[X::Recurring("autocomplete", ANY_STRING)], autocomplete),
    (
        &[X::Atom("log"), X::Recurring("trace", ANY_STRING)],
        log_trace,
    ),
    (
        &[
            X::Atom("log"),
            X::Predicate("context", ANY_ATOM),
            X::Predicate("level", ANY_U8),
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
                    let ether = build_nest(subnest, &commands[1..], handler);
                    if let Some(ether) = ether {
                        subnest
                            .head
                            .insert(commands[0].name(), (commands[0], ether));
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
                nest.head
                    .insert(commands[0].name(), (commands[0].clone(), result.unwrap()));
            } else {
                nest.head.insert(
                    commands[0].name(),
                    (commands[0].clone(), Either::Left(ether)),
                );
            }
            None
        }
    } else {
        Some(Either::Right(handler))
    }
}

// ---

impl<'a> Evaluate<String> for Gsh<'a> {
    fn evaluate(&mut self, commands: &[Data]) -> String {
        let mut args: Vec<Input> = vec![];

        let mut nest = self.commands.head.clone();
        let mut to_handle: Fun = unrecognized_command;
        let mut to_predicate: Option<(&str, &str, fn(&str) -> Option<Input>, bool)> = None;

        for c in commands {
            let (atom, string);

            match c {
                Data::Atom(atom2) => atom = *atom2,
                Data::Command(cmd) => {
                    match self.interpret_single(cmd) {
                        Ok(result) => {
                            string = result;
                            atom = &string[..];
                        }
                        Err(error) => return format!["Error parsing: {}", cmd],
                    };
                }
            }
            if let Some((name, desc, pred, recur)) = to_predicate {
                let result = pred(atom);
                self.logger.debug(
                    "gsh",
                    Log::Bool("Predicate evaluated", "is_ok", result.is_some()),
                );
                if let Some(result) = result {
                    if !recur {
                        to_predicate = None;
                    }
                    args.push(result.clone());
                } else {
                    return format!["Expected: {}, but got: {:?}", desc, atom];
                }
            } else {
                match nest.get(atom) {
                    Some((x, up_next)) => {
                        match x {
                            X::Atom(_) => {
                                self.logger.debug(
                                    "gsh",
                                    Log::StaticDynamic("Found atom", "entry", (*atom).into()),
                                );
                            }
                            X::Predicate(name, (desc, pred)) => {
                                self.logger.debug(
                                    "gsh",
                                    Log::StaticDynamic("Found predicate", "entry", (*atom).into()),
                                );
                                to_predicate = Some((name, desc, *pred, false));
                            }
                            X::Recurring(name, (desc, pred)) => {
                                self.logger.debug(
                                    "gsh",
                                    Log::StaticDynamic(
                                        "Found recurring predicate",
                                        "entry",
                                        (*atom).into(),
                                    ),
                                );
                                to_predicate = Some((name, desc, *pred, true));
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

        if let Some((name, desc, _, false)) = to_predicate {
            self.logger.debug(
                "gsh",
                Log::StaticDynamics(
                    "Unresolved predicate. Aborting",
                    vec![("name", name.into()), ("type", desc.into())],
                ),
            );
            return format!["Missing predicate, value={}, type={}", name, desc];
        }

        self.logger
            .debug("gsh", Log::Static("Command resolved, calling handler"));
        to_handle(self, &args[..])
    }
}

// ---

mod command_handlers {
    use super::*;

    pub fn unrecognized_command(s: &mut Gsh, commands: &[Input]) -> String {
        "Command not finished".into()
    }

    pub fn autocomplete(s: &mut Gsh, commands: &[Input]) -> String {
        let mut nesthead = s.commands.head.clone();
        let mut waspred = false;
        let mut predname = "";
        let mut recur = false;
        for cmd in commands {
            if waspred {
                waspred = recur;
                continue;
            }
            match cmd {
                Input::String(string) => match nesthead.clone().get(&string[..]) {
                    Some((x, Either::Left(nest))) => {
                        nesthead = nest.head.clone();
                        match x {
                            X::Atom(_) => {
                                waspred = false;
                            }
                            X::Predicate(_, (n, _)) => {
                                waspred = true;
                                predname = n;
                            }
                            X::Recurring(_, (n, _)) => {
                                waspred = true;
                                predname = n;
                                recur = true;
                            }
                        }
                    }
                    Some((x, Either::Right(_))) => match x {
                        X::Atom(_) => {
                            waspred = false;
                        }
                        X::Predicate(_, (n, _)) => {
                            waspred = true;
                            predname = n;
                        }
                        X::Recurring(_, (n, _)) => {
                            waspred = true;
                            predname = n;
                            recur = true;
                        }
                    },
                    None => {
                        return "Exceeded command parameter count".into();
                    }
                },
                _ => {
                    unreachable![];
                }
            }
        }
        if waspred {
            format!["{:?}", predname]
        } else {
            format!["{:?}", nesthead.keys()]
        }
    }

    pub fn log(s: &mut Gsh, commands: &[Input]) -> String {
        match commands[0] {
            Input::U8(level) => {
                s.logger.set_log_level(level);
                "OK: Changed log level".into()
            }
            _ => "Usage: log level <u8>".into(),
        }
    }

    pub fn number(s: &mut Gsh, commands: &[Input]) -> String {
        "0".into()
    }

    pub fn log_trace(s: &mut Gsh, commands: &[Input]) -> String {
        let mut sum = String::new();
        for (idx, cmd) in commands.iter().enumerate() {
            match cmd {
                Input::String(ref string) => {
                    if idx + 1 < commands.len() && idx != 0 {
                        sum.push(' ');
                    }
                    sum += string;
                }
                _ => return "Error".into(),
            }
        }
        s.logger.trace("user", Log::Dynamic(sum));
        "OK".into()
    }

    pub fn log_context(s: &mut Gsh, commands: &[Input]) -> String {
        let ctx;
        match commands[0] {
            Input::Atom(ref context) => {
                ctx = match &context[..] {
                    "cli" => "cli",
                    "trace" => "trace",
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, BufRead, Read, Write};
    use std::net::TcpStream;
    use std::sync::atomic::Ordering;
    use test::{black_box, Bencher};

    #[test]
    fn nondeterministic_change_log_level() -> io::Result<()> {
        let (mut logger, logger_handle) = crate::libs::logger::Logger::spawn();
        assert_ne![123, logger.get_log_level()];
        let (mut gsh, keep_running) = spawn(logger.clone());
        std::thread::sleep(std::time::Duration::new(0, 50_000_000));
        {
            let mut listener = TcpStream::connect("127.0.0.1:32931")?;
            write![listener, "log global level 123"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 256])?;
        }
        // std::thread::sleep(std::time::Duration::new(0, 50_000_000));
        assert_eq![123, logger.get_log_level()];
        keep_running.store(false, Ordering::Release);
        std::mem::drop(logger);
        let mut listener = TcpStream::connect("127.0.0.1:32931")?;
        logger_handle.join();
        Ok(())
    }

    #[bench]
    fn message_bandwidth_over_tcp(b: &mut Bencher) -> io::Result<()> {
        let (mut logger, logger_handle) = crate::libs::logger::Logger::spawn();
        let (mut gsh, keep_running) = spawn(logger.clone());
        std::thread::sleep(std::time::Duration::new(0, 50_000_000));
        logger.set_log_level(0);
        let mut listener = TcpStream::connect("127.0.0.1:32931")?;
        b.iter(|| -> io::Result<()> {
            write![listener, "log global level 0"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 1024])?;
            Ok(())
        });
        keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        std::mem::drop(logger);
        let listener = TcpStream::connect("127.0.0.1:32931")?;
        logger_handle.join();
        Ok(())
    }
}
