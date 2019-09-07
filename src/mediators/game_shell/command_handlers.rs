use super::*;
use crate::game::Client;
use gameshell::types::Type;

use fast_logger::{debug, InDebug};

pub fn void(_: &mut GameShellContext, _: &[Type]) -> Result<String, String> {
    Ok("".into())
}

pub fn add(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 0;
    for arg in args {
        match arg {
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

pub fn sub(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = if args.len() == 1 {
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
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn mul(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 1;
    for arg in args {
        match arg {
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

pub fn div(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum: i32 = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn modulo(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn xor(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn band(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn bor(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = 0;
    if let Some(arg) = args.iter().next() {
        match arg {
            Type::I32(x) => {
                sum = *x;
            }
            _ => {
                return Err("Expected i32".into());
            }
        }
    }
    for arg in args.iter().skip(1) {
        match arg {
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

pub fn cat(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut string = String::new();
    for arg in args {
        match arg {
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

pub fn do_get(gsh: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let [Type::String(key)] = args {
        if let Some(string) = gsh.variables.get(key) {
            Ok(string.clone())
        } else {
            Err(format!["Variable not exist: {}", key])
        }
    } else {
        Err("Expected string".into())
    }
}

pub fn do_set(gsh: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let [Type::String(key), Type::String(value)] = args {
        gsh.variables.insert(key.clone(), value.clone());
        Ok("Ok".into())
    } else {
        Err("Expected String String".into())
    }
}

pub fn create_string(_: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let [Type::String(command)] = args {
        Ok(command.clone())
    } else {
        Err("Did not get command".into())
    }
}

pub fn log(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let [Type::U8(level)] = args {
        s.logger.set_log_level(*level);
        Ok("Ok: Changed log level".into())
    } else {
        Err("Usage: log level <u8>".into())
    }
}

pub fn log_trace(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    let mut sum = String::new();
    for (idx, arg) in args.iter().enumerate() {
        match arg {
            Type::String(ref string) => {
                if idx + 1 < args.len() && idx != 0 {
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

pub fn set_gravity(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Type::F32(value) = args[0] {
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

pub fn set_fps(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let [Type::F32(fps)] = args {
            let fps = *fps;
            match chan.send(Box::new(move |main: &mut Client| {
                main.config.fps = fps;
            })) {
                Ok(()) => Ok("Changed fps".into()),
                _ => Err("Unable to send message to main".into()),
            }
        } else {
            Err("Fps value is negative".into())
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn enable_gravity(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Type::Bool(value) = args[0] {
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

pub fn log_context(s: &mut GameShellContext, args: &[Type]) -> Result<String, String> {
    if let [Type::Atom(context), Type::U8(level)] = args {
        if s.logger.set_context_specific_log_level(context, *level) {
            Ok("Ok: Changed log level".into())
        } else {
            Err("Invalid logging context".into())
        }
    } else {
        Err("Usage: log context <atom> level <u8>".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameshell::types::*;

    #[quickcheck_macros::quickcheck]
    fn log_context_quickcheck(args: Vec<Type>) {
        let _ = log_context(&mut GameShellContext::default(), &args[..]);
        let _ = set_fps(&mut GameShellContext::default(), &args[..]);

        let _ = add(&mut GameShellContext::default(), &args[..]);
        let _ = sub(&mut GameShellContext::default(), &args[..]);
        let _ = mul(&mut GameShellContext::default(), &args[..]);
        let _ = div(&mut GameShellContext::default(), &args[..]);
        let _ = modulo(&mut GameShellContext::default(), &args[..]);
        let _ = xor(&mut GameShellContext::default(), &args[..]);
        let _ = band(&mut GameShellContext::default(), &args[..]);
        let _ = bor(&mut GameShellContext::default(), &args[..]);

        let _ = cat(&mut GameShellContext::default(), &args[..]);
        let _ = do_get(&mut GameShellContext::default(), &args[..]);
        let _ = do_set(&mut GameShellContext::default(), &args[..]);
        let _ = create_string(&mut GameShellContext::default(), &args[..]);
        let _ = log(&mut GameShellContext::default(), &args[..]);
        let _ = log_trace(&mut GameShellContext::default(), &args[..]);
        let _ = set_gravity(&mut GameShellContext::default(), &args[..]);
        let _ = enable_gravity(&mut GameShellContext::default(), &args[..]);
    }
}
