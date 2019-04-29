//! Contains predicates used by the command matcher for the gameshell.
//!
//! Please only put predicates in this file and predicate helpers.
use super::*;
use cmdmat::{Decider, Decision};

// ---

// Please keep this list sorted

pub const ANY_ATOM: SomeDec = Some(&Decider {
    description: "<atom>",
    decider: any_atom_function,
});
pub const ANY_BOOL: SomeDec = Some(&Decider {
    description: "<true/false>",
    decider: any_bool_function,
});
pub const ANY_F32: SomeDec = Some(&Decider {
    description: "<f32>",
    decider: any_f32_function,
});
pub const ANY_STRING: SomeDec = Some(&Decider {
    description: "<string>",
    decider: any_string_function,
});
pub const ANY_U8: SomeDec = Some(&Decider {
    description: "<u8>",
    decider: any_u8_function,
});
pub const IGNORE_ALL: SomeDec = Some(&Decider {
    description: "<anything> ...",
    decider: ignore_all_function,
});
pub const MANY_I32: SomeDec = Some(&Decider {
    description: "<i32> ...",
    decider: many_i32_function,
});
pub const MANY_STRING: SomeDec = Some(&Decider {
    description: "<string> ...",
    decider: many_string_function,
});
pub const TWO_STRINGS: SomeDec = Some(&Decider {
    description: "<string> <string>",
    decider: two_string_function,
});

// ---

// TODO: Replace usage of this macro with `?'
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

// ---

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

fn any_bool_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<bool>().ok().map(Input::Bool) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny(GshDecision::Err(input[0].into()));
        }
    }
    Decision::Accept(1)
}

fn any_f32_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<f32>().ok().map(Input::F32) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny(GshDecision::Err(input[0].into()));
        }
    }
    Decision::Accept(1)
}

fn any_string_function(input: &[&str], out: &mut SVec<Input>) -> Decision<GshDecision> {
    ret_if_err![aslen(input, 1)];
    out.push(Input::String(input[0].to_string()));
    Decision::Accept(1)
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

fn ignore_all_function(input: &[&str], _: &mut SVec<Input>) -> Decision<GshDecision> {
    Decision::Accept(input.len())
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

// ---

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
