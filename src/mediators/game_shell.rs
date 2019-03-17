use self::command_handlers::*;
use self::predicates::*;
use crate::glocals::{GameShell, GameShellContext, Log};
use cmdmat::{self, LookError, SVec};
use either::Either;
use logger::{self, Logger};
use metac::{Data, Evaluate, ParseError, PartialParse, PartialParseOp};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

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

mod command_handlers {
    use super::*;

    pub fn void(_: &mut GameShellContext, _: &[Input]) -> Result<String, String> {
        Ok("".into())
    }

    pub fn add(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum: i32 = 0;
        for cmd in commands {
            match cmd {
                Input::I32(x) => {
                    sum = if let Some(num) = sum.checked_add(*x) {
                        num
                    } else {
                        return Err("Addition overflow".into());
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn sub(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = if commands.len() == 1 {
                        if let Some(num) = x.checked_neg() {
                            num
                        } else {
                            return Err("Subtraction overflow".into());
                        }
                    } else {
                        *x
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum = if let Some(num) = sum.checked_sub(*x) {
                        num
                    } else {
                        return Err("Subtraction overflow".into());
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn mul(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum: i32 = 1;
        for cmd in commands {
            match cmd {
                Input::I32(x) => {
                    sum = if let Some(num) = sum.checked_mul(*x) {
                        num
                    } else {
                        return Err("Multiplication overflow".into());
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn div(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum: i32 = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = *x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum = if let Some(num) = sum.checked_div(*x) {
                        num
                    } else if *x == 0 {
                        return Err("Division by zero".into());
                    } else {
                        return Err("Division overflow".into());
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn modulo(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = *x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum = if let Some(num) = sum.checked_rem(*x) {
                        num
                    } else if *x == 0 {
                        return Err("Modulo by zero".into());
                    } else {
                        return Err("Modulo overflow".into());
                    };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn xor(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = *x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum ^= x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn band(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = *x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum &= x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn bor(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        if let Some(cmd) = commands.iter().next() {
            match cmd {
                Input::I32(x) => {
                    sum = *x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum |= x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn cat(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut string = String::new();
        for cmd in commands {
            match cmd {
                Input::String(res) => {
                    string += res;
                }
                _ => {
                    return Err("Expected string".into());
                }
            }
        }
        Ok(string)
    }

    pub fn do_get(gsh: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let key;
        match commands[0] {
            Input::String(ref string) => {
                key = string.clone();
            }
            _ => {
                return Err("F".into());
            }
        }
        if let Some(string) = gsh.variables.get(&key) {
            Ok(string.clone())
        } else {
            Err(format!["Variable not exist: {}", key])
        }
    }

    pub fn do_set(gsh: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let (key, value);
        match commands[0] {
            Input::String(ref string) => {
                key = string.clone();
            }
            _ => {
                return Err("F".into());
            }
        }
        match commands[1] {
            Input::String(ref string) => {
                value = string.clone();
            }
            _ => {
                return Err("F".into());
            }
        }
        gsh.variables.insert(key, value);
        Ok("Ok".into())
    }

    pub fn create_string(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        if commands.len() != 1 {
            return Err("Did not get command".into());
        }
        match commands[0] {
            Input::String(ref cmd) => Ok(cmd.clone()),
            _ => Err("Error: Not a command".into()),
        }
    }

    pub fn log(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        match commands[0] {
            Input::U8(level) => {
                s.logger.set_log_level(level);
                Ok("Ok: Changed log level".into())
            }
            _ => Err("Usage: log level <u8>".into()),
        }
    }

    pub fn log_trace(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = String::new();
        for (idx, cmd) in commands.iter().enumerate() {
            match cmd {
                Input::String(ref string) => {
                    if idx + 1 < commands.len() && idx != 0 {
                        sum.push(' ');
                    }
                    sum += string;
                }
                _ => return Err("Error".into()),
            }
        }
        s.logger.trace("user", Log::Dynamic(sum));
        Ok("Ok".into())
    }

    pub fn log_context(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let ctx;
        match commands[0] {
            Input::Atom(ref context) => {
                ctx = match &context[..] {
                    "cli" => "cli",
                    "trace" => "trace",
                    "gsh" => "gsh",
                    "benchmark" => "benchmark",
                    "logger" => "logger",
                    _ => return Err("Invalid logging context".into()),
                };
            }
            _ => return Err("Usage: log context <atom> level <u8>".into()),
        }
        match commands[1] {
            Input::U8(level) => {
                s.logger.set_context_specific_log_level(ctx, level);
                Ok("Ok: Changed log level".into())
            }
            _ => Err("Usage: log context <atom> level <u8>".into()),
        }
    }
}

pub enum GshDecision {
    Help(String),
    Err(String),
}

mod predicates {
    use super::*;
    use cmdmat::{Decider, Decision};

    macro_rules! ret_if_err {
        ($e:expr) => {{
            let res = $e;
            match res {
                Ok(x) => x,
                Err(res) => {
                    return res;
                }
            }
        }};
    }

    fn aslen(input: &[&str], input_l: usize) -> Result<(), Decision<GshDecision>> {
        if input.len() < input_l {
            Err(Decision::Deny(GshDecision::Err(format![
                "Too few elements: {:?}",
                input
            ])))
        } else {
            Ok(())
        }
    }

    fn any_atom_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        ret_if_err![aslen(input, 1)];
        for i in input[0].chars() {
            if i.is_whitespace() {
                return Decision::Deny(GshDecision::Err(input[0].into()));
            }
        }
        out.push(Input::Atom(input[0].to_string()));
        Decision::Accept(1)
    }

    fn any_string_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        ret_if_err![aslen(input, 1)];
        out.push(Input::String(input[0].to_string()));
        Decision::Accept(1)
    }

    fn many_string_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        ret_if_err![aslen(input, input.len())];
        let mut cnt = 0;
        for (idx, i) in input.iter().enumerate() {
            out.push(Input::String((*i).into()));
            cnt = idx + 1;
        }
        Decision::Accept(cnt)
    }

    fn two_string_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        if input.len() == 1 {
            return Decision::Deny(GshDecision::Help("<string>".into()));
        }
        ret_if_err![aslen(input, 2)];
        out.push(Input::String(input[0].to_string()));
        out.push(Input::String(input[1].to_string()));
        Decision::Accept(2)
    }

    fn any_u8_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        ret_if_err![aslen(input, 1)];
        match input[0].parse::<u8>().ok().map(Input::U8) {
            Some(num) => {
                out.push(num);
            }
            None => {
                return Decision::Deny(GshDecision::Err(input[0].into()));
            }
        }
        Decision::Accept(1)
    }

    fn many_i32_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
        let mut cnt = 0;
        for i in input.iter() {
            if let Some(num) = i.parse::<i32>().ok().map(Input::I32) {
                ret_if_err![aslen(input, cnt + 1)];
                out.push(num);
                cnt += 1;
            } else {
                break;
            }
        }
        Decision::Accept(cnt)
    }

    fn ignore_all(input: &[&str], _: &mut SVec<Input>) -> Decision<GshDecision> {
        Decision::Accept(input.len())
    }

    type SomeDec = Option<&'static Decider<Input, GshDecision>>;
    pub const ANY_ATOM: SomeDec = Some(&Decider {
        description: "<atom>",
        decider: any_atom_function,
    });

    pub const ANY_STRING: SomeDec = Some(&Decider {
        description: "<string>",
        decider: any_string_function,
    });

    pub const MANY_STRING: SomeDec = Some(&Decider {
        description: "<string> ...",
        decider: many_string_function,
    });

    pub const TWO_STRINGS: SomeDec = Some(&Decider {
        description: "<string> <string>",
        decider: two_string_function,
    });

    pub const ANY_U8: SomeDec = Some(&Decider {
        description: "<u8>",
        decider: any_u8_function,
    });

    pub const MANY_I32: SomeDec = Some(&Decider {
        description: "<i32> ...",
        decider: many_i32_function,
    });

    pub const IGNORE_ALL: SomeDec = Some(&Decider {
        description: "<anything> ...",
        decider: ignore_all,
    });
}

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

fn spawn_with_listener(
    logger: Logger<Log>,
    listener: TcpListener,
) -> (JoinHandle<()>, Arc<AtomicBool>) {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    (
        thread::Builder::new()
            .name("gsh/server".to_string())
            .spawn(move || {
                let mut cmdmat = cmdmat::Mapping::default();
                cmdmat.register_many(SPEC).unwrap();
                game_shell_thread(
                    GameShell {
                        gshctx: GameShellContext {
                            config_change: None,
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
        keep_running_clone,
    )
}

pub fn spawn(mut logger: Logger<Log>) -> Option<(JoinHandle<()>, Arc<AtomicBool>)> {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:32931") {
        Some(spawn_with_listener(logger, listener))
    } else {
        logger.info("gsh", Log::Static("Unable to bind to tcp port"));
        None
    }
}

// ---

fn clone_and_spawn_connection_handler(s: &Gsh, stream: TcpStream) -> JoinHandle<()> {
    let logger = s.gshctx.logger.clone();
    let keep_running = s.gshctx.keep_running.clone();
    thread::Builder::new()
        .name("gsh/server/handler".to_string())
        .spawn(move || {
            let mut cmdmat = cmdmat::Mapping::default();
            cmdmat.register_many(SPEC).unwrap();
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

mod proc {
    pub enum Consumption {
        Consumed(usize),
        Stop,
    }
    pub enum Validation {
        Ready,
        Unready,
        Discard,
    }
    pub enum Process {
        Continue,
        Stop,
    }
    /// Incremental consumer of bytes
    ///
    /// Consume bytes until a complete set of bytes has been found, then, run a handler
    /// function on just that set of bytes.
    ///
    /// This is used for accepting bytes from some external stream, note that we set a maximum
    /// size on the buffer, so no external input can cause excessive memory usage.
    /// Parsing must verify the legitimacy of the stream.
    pub trait IncConsumer {
        /// Consume bytes and place them on an output stack
        fn consume(&mut self, output: &mut [u8]) -> Consumption;
        /// Validate part of the bytestream, as soon as we return `Validation::Ready`, `process`
        /// will be run on the current accumulated bytes, after which these bytes will be deleted.
        fn validate(&mut self, output: u8) -> Validation;
        /// Process do actual stuff with the bytes to affect the system.
        ///
        /// The sequence of bytes input here will have been verified by the `validate`
        /// function.
        fn process(&mut self, input: &[u8]) -> Process;

        /// Runs the incremental consumer until it is signalled to quit
        fn run(&mut self, bufsize: usize) {
            let mut buf = vec![b'\0'; bufsize];
            let mut begin = 0;
            let mut shift = 0;
            loop {
                match self.consume(&mut buf[begin..]) {
                    Consumption::Consumed(amount) => {
                        for ch in buf[begin..(begin + amount)].iter() {
                            begin += 1;
                            match self.validate(*ch) {
                                Validation::Ready => {
                                    match self.process(&buf[shift..begin]) {
                                        Process::Continue => {}
                                        Process::Stop => return,
                                    }
                                    shift = begin;
                                }
                                Validation::Unready => {}
                                Validation::Discard => shift = begin,
                            }
                        }
                    }
                    Consumption::Stop => return,
                }
            }
        }
    }
}

use self::proc::*;

struct GshTcp<'a, 'b> {
    pub gsh: &'a mut Gsh<'b>,
    pub stream: TcpStream,
    pub parser: PartialParse,
}

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
                    Log::Static(
                        "Message parsing succeeded and evaluated, sending response to client",
                    ),
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
                                .warn("gsh", Log::Static("Sending empty help message"));
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
                    .error("gsh", Log::Static("Message parsing failed"));
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
                Log::Static(
                    "Malformed UTF-8 received, this should never happen. Ending connection",
                ),
            );
            Process::Stop
        }
    }
}

// ---

fn connection_loop(s: &mut Gsh, stream: TcpStream) -> io::Result<()> {
    s.gshctx
        .logger
        .debug("gsh", Log::Static("Acquired new stream"));
    let mut gshtcp = GshTcp {
        gsh: s,
        stream,
        parser: PartialParse::default(),
    };
    gshtcp.run(2048);
    Ok(())
}

fn game_shell_thread(mut s: Gsh, listener: TcpListener) {
    s.gshctx
        .logger
        .info("gsh", Log::Static("Started GameShell server"));
    'outer_loop: loop {
        for stream in listener.incoming() {
            if !s.gshctx.keep_running.load(Ordering::Acquire) {
                s.gshctx
                    .logger
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

// ---

type Gsh<'a> = GameShell<Arc<cmdmat::Mapping<'a, Input, GshDecision, GameShellContext>>>;
#[derive(Clone)]
pub enum Input {
    U8(u8),
    I32(i32),
    Atom(String),
    String(String),
    Command(String),
}

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

    fn bind_to_any_tcp_port() -> (TcpListener, u16) {
        for i in 10000..=u16::max_value() {
            if let Ok(listener) =
                TcpListener::bind("127.0.0.1:".to_string() + i.to_string().as_ref())
            {
                return (listener, i);
            }
        }
        panic!["Unable to find an available port"];
    }

    // ---

    #[test]
    fn change_log_level() -> io::Result<()> {
        let (logger, logger_handle) = logger::Logger::spawn();
        assert_ne![123, logger.get_log_level()];
        let (listener, port) = bind_to_any_tcp_port();
        let (_gsh, keep_running) = spawn_with_listener(logger.clone(), listener);
        let mut listener =
            TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref()).unwrap();
        {
            writeln![listener, "log global level 123"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 256])?;
        }
        assert_eq![123, logger.get_log_level()];
        keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        std::mem::drop(logger);
        let _ = TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;
        logger_handle.join().unwrap();
        Ok(())
    }

    #[test]
    fn fuzzing_result_does_not_crash() -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
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
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[test]
    fn check_variable_statements() -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
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

            // then
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[test]
    fn check_integer_overflow() -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
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
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[bench]
    fn speed_of_adding_a_bunch_of_numbers(b: &mut Bencher) -> io::Result<()> {
        let logger_handle = {
            // given
            let (mut logger, logger_handle) = logger::Logger::spawn();
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
            logger_handle
        };
        logger_handle.join().unwrap();

        // cleanup
        Ok(())
    }

    #[bench]
    fn message_bandwidth_over_tcp(b: &mut Bencher) -> io::Result<()> {
        let (mut logger, logger_handle) = logger::Logger::spawn();
        let (listener, port) = bind_to_any_tcp_port();
        let (mut _gsh, keep_running) = spawn_with_listener(logger.clone(), listener);
        logger.set_log_level(0);
        let mut listener =
            TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;
        b.iter(|| -> io::Result<()> {
            writeln![listener, "log global level 0"]?;
            listener.flush()?;
            listener.read(&mut [0u8; 1024])?;
            Ok(())
        });
        keep_running.store(false, Ordering::Release);
        std::mem::drop(listener);
        std::mem::drop(logger);
        let _ = TcpStream::connect("127.0.0.1:".to_string() + port.to_string().as_ref())?;
        let _ = logger_handle.join().unwrap();
        Ok(())
    }
}
