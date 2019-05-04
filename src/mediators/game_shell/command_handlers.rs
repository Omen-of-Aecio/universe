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

pub fn set_gravity(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Input::F32(value) = commands[0] {
            match chan.send(Box::new(move |main: &mut Main| {
                main.logic.config.world.gravity = value;
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

pub fn get_fps(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        let (tx, rx) = mpsc::sync_channel(0);
        let result = chan.send(Box::new(move |main: &mut Main| {
            tx.send(main.logic.config.client.fps);
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

pub fn set_fps(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Input::F32(value) = commands[0] {
            if value < 0.0 {
                return Err("Fps value is negative".into());
            }
            match chan.send(Box::new(move |main: &mut Main| {
                main.logic.config.client.fps = value;
            })) {
                Ok(()) => Ok("Changed fps".into()),
                _ => Err("Unable to send message to main".into()),
            }
        } else {
            Err("Did not get f32".into())
        }
    } else {
        Err("Unable to contact main".into())
    }
}

pub fn enable_gravity(s: &mut GameShellContext, commands: &[Input]) -> Result<String, String> {
    if let Some(ref mut chan) = s.config_change {
        if let Input::Bool(value) = commands[0] {
            match chan.send(Box::new(move |main: &mut Main| {
                main.logic.config.world.gravity_on = value;
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
