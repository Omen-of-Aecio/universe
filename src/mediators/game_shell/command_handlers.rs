use super::*;
use crate::game::Client;
use gameshell::types::Type;

use fast_logger::{debug, InDebug};

pub fn void(_: &mut GameShellContext, _: &[Type]) -> Result<String, String> {
    Ok("".into())
}

pub fn add(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 0;
    for cmd in commands {
        match cmd {
            Type::I32(x) => {
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

pub fn sub(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
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
            Type::I32(x) => {
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

pub fn mul(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 1;
    for cmd in commands {
        match cmd {
            Type::I32(x) => {
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

pub fn div(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for cmd in commands.iter().skip(1) {
        match cmd {
            Type::I32(x) => {
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

pub fn modulo(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for cmd in commands.iter().skip(1) {
        match cmd {
            Type::I32(x) => {
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

pub fn xor(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for cmd in commands.iter().skip(1) {
        match cmd {
            Type::I32(x) => {
                sum ^= x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    Ok(sum.to_string())
}

pub fn band(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for cmd in commands.iter().skip(1) {
        match cmd {
            Type::I32(x) => {
                sum &= x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    Ok(sum.to_string())
}

pub fn bor(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(cmd) = commands.iter().next() {
        match cmd {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for cmd in commands.iter().skip(1) {
        match cmd {
            Type::I32(x) => {
                sum |= x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    Ok(sum.to_string())
}

pub fn cat(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut string = String::new();
    for cmd in commands {
        match cmd {
            Type::String(res) => {
                string += res;
            }
            _ => {
                return Err("Expected string".into());
            }
        }
    }
    Ok(string)
}

pub fn do_get(gsh: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let [Type::String(key)] = commands {
        if let Some(string) = gsh.variables.get(key) {
            Ok(string.clone())
        } else {
            Err(format!["Variable not exist: {}", key])
        }
    } else {
        Err("Expected string".into())
    }
}

pub fn do_set(gsh: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let [Type::String(key), Type::String(value)] = commands {
        gsh.variables.insert(key.clone(), value.clone());
        Ok("Ok".into())
    } else {
        Err("Expected String String".into())
    }
}

pub fn create_string(_: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let [Type::String(command)] = commands {
        Ok(command.clone())
    } else {
        return Err("Did not get command".into());
    }
}

pub fn log(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let [Type::U8(level)] = commands {
        s.logger.set_log_level(*level);
        Ok("Ok: Changed log level".into())
    } else {
        Err("Usage: log level <u8>".into())
    }
}

pub fn log_trace(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    let mut sum = String::new();
    for (idx, cmd) in commands.iter().enumerate() {
        match cmd {
            Type::String(ref string) => {
                if idx + 1 < commands.len() && idx != 0 {
                    sum.push(' ');
                }
                sum += string;
            }
            _ => return Err("Error".into()),
        }
    }
    s.logger.trace(Log::Dynamic(sum));
    Ok("Ok".into())
}

pub fn set_gravity(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Type::F32(value) = commands[0] {
            match chan.send(Box::new(move |main: &mut Client| {
                main.logic.config.gravity = value;
            })) {
                Ok(()) => Ok("Set gravity value".into()),
                Err(_) => Err("Unable to send message to main".into()),
            }
        } else {
            Err("Did not get an f32".into())
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn get_fps(s: &mut GameShellContext, _: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        let (tx, rx) = mpsc::sync_channel(0);
        let result = chan.send(Box::new(move |main: &mut Client| {
            let send_status = tx.send(main.config.fps);
            debug![main.logger, "Message reply"; "status" => InDebug(&send_status)];
        }));
        let fps = rx.recv().unwrap();
        match result {
            Ok(()) => Ok(fps.to_string()),
            _ => Err("Unable to send message to main".into()),
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn set_fps(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let [Type::F32(fps)] = commands {
            let fps = *fps;
            match chan.send(Box::new(move |main: &mut Client| {
                main.config.fps = fps;
            })) {
                Ok(()) => Ok("Changed fps".into()),
                _ => Err("Unable to send message to main".into()),
            }
        } else {
            return Err("Fps value is negative".into());
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn enable_gravity(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Type::Bool(value) = commands[0] {
            match chan.send(Box::new(move |main: &mut Client| {
                main.logic.config.gravity_on = value;
            })) {
                Ok(()) => Ok("Enabled/disabled gravity".into()),
                _ => Err("Unable to send message to main".into()),
            }
        } else {
            Err("Did not get a boolean".into())
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn log_context(s: &mut GameShellContext, commands: &[Type]) -> Result<String, String> {
    if let [Type::Atom(context), Type::U8(level)] = commands {
        let ctx = match &context[..] {
            "cli" => "cli",
            "trace" => "trace",
            "gsh" => "gsh",
            "benchmark" => "benchmark",
            "logger" => "logger",
            _ => return Err("Invalid logging context".into()),
        };
        s.logger.set_context_specific_log_level(ctx, *level);
        Ok("Ok: Changed log level".into())
    } else {
        Err("Usage: log context <atom> level <u8>".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameshell::types::*;

    #[quickcheck_macros::quickcheck]
    fn log_context_quickcheck(commands: Vec<Type>) {
        let _ = log_context(&mut GameShellContext::default(), &commands[..]);
        let _ = set_fps(&mut GameShellContext::default(), &commands[..]);

        let _ = add(&mut GameShellContext::default(), &commands[..]);
        let _ = sub(&mut GameShellContext::default(), &commands[..]);
        let _ = mul(&mut GameShellContext::default(), &commands[..]);
        let _ = div(&mut GameShellContext::default(), &commands[..]);
        let _ = modulo(&mut GameShellContext::default(), &commands[..]);
        let _ = xor(&mut GameShellContext::default(), &commands[..]);
        let _ = band(&mut GameShellContext::default(), &commands[..]);
        let _ = bor(&mut GameShellContext::default(), &commands[..]);

        let _ = cat(&mut GameShellContext::default(), &commands[..]);
        let _ = do_get(&mut GameShellContext::default(), &commands[..]);
        let _ = do_set(&mut GameShellContext::default(), &commands[..]);
        let _ = create_string(&mut GameShellContext::default(), &commands[..]);
        let _ = log(&mut GameShellContext::default(), &commands[..]);
        let _ = log_trace(&mut GameShellContext::default(), &commands[..]);
        let _ = set_gravity(&mut GameShellContext::default(), &commands[..]);
        let _ = enable_gravity(&mut GameShellContext::default(), &commands[..]);
    }
}
