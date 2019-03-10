use self::command_handlers::*;
use self::predicates::*;
use crate::glocals::{GameShell, GameShellContext, Log};
use cmdmat;
use either::Either;
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

// ---

#[rustfmt::skip]
const SPEC: &[cmdmat::Spec<Input, String, GameShellContext>] = &[
    (&[("log", None), ("global", None), ("level", ANY_U8)], log),
    (&[("set", TWO_STRINGS)], do_set),
    (&[("get", ANY_STRING)], do_get),
    (&[("str", ANY_STRING)], create_string),
    (&[("void", IGNORE_ALL)], void),
    (&[("+", MANY_I32)], add),
    (&[("-", MANY_I32)], sub),
    (&[("*", MANY_I32)], mul),
    (&[("/", MANY_I32)], div),
    (&[("%", MANY_I32)], modulo),
    (&[("^", MANY_I32)], xor),
    (&[("&", MANY_I32)], band),
    (&[("|", MANY_I32)], bor),
    (&[("log", None), ("trace", ANY_STRING)], log_trace),
    (&[("log", None), ("context", ANY_ATOM), ("level", ANY_U8)], log_context),
];

// ---

mod command_handlers {
    use super::*;

    pub fn void(_: &mut GameShellContext, _: &[Input]) -> Result<String, String> {
        Ok("".into())
    }

    pub fn add(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 0;
        for cmd in commands {
            match cmd {
                Input::I32(x) => {
                    sum += x;
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
                    sum = if commands.len() == 1 { -*x } else { *x };
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        for cmd in commands.iter().skip(1) {
            match cmd {
                Input::I32(x) => {
                    sum -= x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn mul(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
        let mut sum = 1;
        for cmd in commands {
            match cmd {
                Input::I32(x) => {
                    sum *= x;
                }
                _ => {
                    return Err("Expected i32".into());
                }
            }
        }
        Ok(sum.to_string())
    }

    pub fn div(_: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
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
                    sum /= x;
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
                    sum %= x;
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
            Err("Does not exist".into())
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
        Ok("OK".into())
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
                Ok("OK: Changed log level".into())
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
        Ok("OK".into())
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
                Ok("OK: Changed log level".into())
            }
            _ => Err("Usage: log context <atom> level <u8>".into()),
        }
    }
}

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
        if out.is_empty() {
            return Decision::Deny("No space in output".into());
        }
        out[0] = Input::String(input[0].to_string());
        Decision::Accept(1)
    }

    fn two_string_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        if out.len() < 2 {
            return Decision::Deny("No space in output".into());
        }
        if input.len() < 2 {
            return Decision::Deny("Not enough arguments provided".into());
        }
        out[0] = Input::String(input[0].to_string());
        out[1] = Input::String(input[1].to_string());
        Decision::Accept(2)
    }

    fn any_u8_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        out[0] = input[0].parse::<u8>().ok().map(Input::U8).unwrap();
        Decision::Accept(1)
    }

    fn any_i32_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        out[0] = input[0].parse::<i32>().ok().map(Input::I32).unwrap();
        Decision::Accept(1)
    }

    fn many_i32_function(input: &[&str], out: &mut [Input]) -> Decision<String> {
        let mut cnt = 0;
        for i in input.iter() {
            if let Some(input) = i.parse::<i32>().ok().map(Input::I32) {
                out[cnt] = input;
                cnt += 1;
            } else {
                break;
            }
        }
        Decision::Accept(cnt)
    }

    fn ignore_all(_: &[&str], _: &mut [Input]) -> Decision<String> {
        Decision::Accept(0)
    }

    type SomeDec = Option<&'static Decider<Input, String>>;
    pub const ANY_ATOM: SomeDec = Some(&Decider {
        description: "<atom>",
        decider: any_atom_function,
    });

    pub const ANY_STRING: SomeDec = Some(&Decider {
        description: "<string>",
        decider: any_string_function,
    });

    pub const TWO_STRINGS: SomeDec = Some(&Decider {
        description: "<string> <string>",
        decider: two_string_function,
    });

    pub const ANY_U8: SomeDec = Some(&Decider {
        description: "<u8>",
        decider: any_u8_function,
    });

    pub const ANY_I32: SomeDec = Some(&Decider {
        description: "<i32>",
        decider: any_i32_function,
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
    let mut cmdmat = cmdmat::Mapping::new();
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

pub fn spawn(logger: Logger<Log>) -> (JoinHandle<()>, Arc<AtomicBool>) {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_clone = keep_running.clone();
    (
        thread::Builder::new()
            .name("gsh/server".to_string())
            .spawn(move || {
                let mut cmdmat = cmdmat::Mapping::new();
                cmdmat.register_many(SPEC).unwrap();
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
                    Log::StaticDynamic("Connection errored out", "reason", format!["{:?}", error]),
                );
            }
        }
    })
}

fn connection_loop(s: &mut Gsh, mut stream: TcpStream) -> io::Result<()> {
    s.gshctx
        .logger
        .debug("gsh", Log::Static("Acquired new stream"));
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
        s.gshctx
            .logger
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
        s.gshctx
            .logger
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
                            match result {
                                EvalRes::Ok(res) => {
                                    stream.write_all(format!["Ok: {}", res].as_bytes())?;
                                }
                                EvalRes::Err(res) => {
                                    stream.write_all(format!["Err: {}", res].as_bytes())?;
                                }
                            }
                            stream.flush()?;
                        } else {
                            s.gshctx
                                .logger
                                .error("gsh", Log::Static("Message parsing failed"));
                            stream.write_all(b"Unable to complete query (parse error)")?;
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

type Gsh<'a> = GameShell<Arc<cmdmat::Mapping<'a, Input, String, GameShellContext>>>;
#[derive(Clone)]
pub enum Input {
    U8(u8),
    I32(i32),
    Atom(String),
    String(String),
    Command(String),
}

impl Default for Input {
    fn default() -> Input {
        Input::U8(0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvalRes {
    Ok(String),
    Err(String),
}

impl Default for EvalRes {
    fn default() -> Self {
        EvalRes::Ok(String::default())
    }
}

// ---

impl<'a> Evaluate<EvalRes> for Gsh<'a> {
    fn evaluate(&mut self, commands: &[Data]) -> EvalRes {
        use cmdmat::LookError;
        let mut stack = [
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
            Input::default(),
        ];
        let mut content: Vec<String> = Vec::new();
        for cmd in commands {
            match cmd {
                Data::Atom(string) => {
                    content.push((*string).into());
                }
                Data::Command(string) => {
                    if &(*string)[0..1] == "#" {
                        content.push((string[1..]).into());
                    } else {
                        let res = self.interpret_single(string);
                        match res {
                            Ok(EvalRes::Ok(string)) => {
                                content.push(string);
                            }
                            Ok(ref res @ EvalRes::Err(_)) => {
                                return res.clone();
                            }
                            Err(_) => {
                                panic![];
                            }
                        }
                    }
                }
            }
        }

        let content_ref = content.iter().map(|s| &s[..]).collect::<Vec<_>>();

        if let Some(front) = content_ref.first() {
            if *front == "autocomplete" {
                match self.commands.partial_lookup(&content_ref[1..], &mut stack) {
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
                        col.sort();
                        return EvalRes::Ok(col.join(", "));
                    }
                    Ok(Either::Right(name)) => {
                        return EvalRes::Ok(name.into());
                    }
                    Err(LookError::DeciderAdvancedTooFar) => {
                        return EvalRes::Err("Decider advanced too far".into());
                    }
                    Err(LookError::DeciderDenied(decider)) => {
                        return EvalRes::Err(decider.into());
                    }
                    Err(LookError::FinalizerDoesNotExist) => {
                        return EvalRes::Err("Finalizer does not exist".into());
                    }
                    Err(LookError::UnknownMapping) => {
                        return EvalRes::Err("Unrecognized command".into());
                    }
                }
            }
        }

        let res = self.commands.lookup(&content_ref[..], &mut stack);
        match res {
            Ok(fin) => {
                let res = fin.0(&mut self.gshctx, &stack[..fin.1]);
                return match res {
                    Ok(res) => EvalRes::Ok(res.into()),
                    Err(res) => EvalRes::Err(res.into()),
                };
            }
            Err(LookError::DeciderAdvancedTooFar) => {
                return EvalRes::Err("Decider advanced too far".into());
            }
            Err(LookError::DeciderDenied(decider)) => {
                return EvalRes::Err(decider.into());
            }
            Err(LookError::FinalizerDoesNotExist) => {
                return EvalRes::Err("Finalizer does not exist".into());
            }
            Err(LookError::UnknownMapping) => {
                return EvalRes::Err("Unrecognized command".into());
            }
        }
    }
}

// ---

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
                EvalRes::Ok("OK".into()),
                gsh.interpret_single("set key some-value").unwrap()
            ];
            assert_eq![
                EvalRes::Ok("some-value".into()),
                gsh.interpret_single("get key").unwrap()
            ];

            assert_eq![
                EvalRes::Err("Unrecognized command".into()),
                gsh.interpret_single("set key some-value extra").unwrap()
            ];

            assert_eq![
                EvalRes::Ok("OK".into()),
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
            let mut cmdmat = cmdmat::Mapping::new();
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
                EvalRes::Err("Unrecognized command".into()),
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
                EvalRes::Err("Unrecognized command".into()),
                gsh.interpret_multiple("+ 1 (+ 8 (+ 1) 0.6 9) (+ 3\n1\n)")
                    .unwrap()
            ];
            assert_eq![
                EvalRes::Err("Unrecognized command".into()),
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
            let mut cmdmat = cmdmat::Mapping::new();
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
