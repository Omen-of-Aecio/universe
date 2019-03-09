#![feature(test)]
extern crate test;

use std::borrow::Borrow;
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

// ---

/// A decision contains information about token consumption by the decider
///
/// If the decider has accepted the tokens, it will return an `Accept(usize)`, if it failed to
/// parse interpret the tokens, it will return a deny value.
pub enum Decision<D> {
    Accept(usize),
    Deny(D),
}

/// A decider is a function taking in a sequence of tokens and an output array
///
/// It puts tokens into the output array according to interal logic and returns how many elements
/// it has consumed. If it could not process the input tokens it will return a `Deny`, containing
/// the reason for denying. Calling a decider with &[] should always yield its deny value.
pub type Decider<A, D> = fn(&[&str], &mut [A]) -> Decision<D>;

#[derive(Debug, PartialEq)]
pub enum RegError {
    FinalizerAlreadyExists,
    DeciderAlreadyExists,
}

#[derive(Debug, PartialEq)]
pub enum LookError<D> {
    FinalizerDoesNotExist,
    DeciderDenied(D),
    DeciderAdvancedTooFar,
    UnknownMapping,
}

// ---

struct Mapping<'a, A, D, C> {
    map: HashMap<&'a str, Mapping<'a, A, D, C>>,
    decider: Option<Decider<A, D>>,
    finalizer: Option<fn(&mut C, &[A])>,
}

impl<'a, A, D, C> Mapping<'a, A, D, C> {
    fn new() -> Mapping<'a, A, D, C> {
        Mapping {
            map: HashMap::new(),
            decider: None,
            finalizer: None,
        }
    }

    fn register_many(&mut self, spec: &[(&[(&'static str, Option<Decider<A, D>>)], fn(&mut C, &[A]))]) -> Result<(), RegError> {
        for subspec in spec {
            self.register(subspec.clone())?;
        }
        Ok(())
    }

    fn register(&mut self, spec: (&[(&'static str, Option<Decider<A, D>>)], fn(&mut C, &[A]))) -> Result<(), RegError> {
        if spec.0.is_empty() {
            if self.finalizer.is_some() {
                return Err(RegError::FinalizerAlreadyExists);
            }
            self.finalizer = Some(spec.1);
            return Ok(());
        }
        let key = spec.0[0].0;
        let decider = spec.0[0].1;
        if let Some(ref mut entry) = self.map.get_mut(key) {
            if decider.is_some() {
                return Err(RegError::DeciderAlreadyExists);
            }
            entry.register((&spec.0[1..], spec.1));
        } else {
            let mut mapping = Mapping::<A, D, C> {
                map: HashMap::new(),
                decider: decider,
                finalizer: None,
            };
            mapping.register((&spec.0[1..], spec.1));
            self.map.insert(key, mapping);
        }
        Ok(())
    }

    fn lookup(&self, input: &[&str], output: &mut [A]) -> Result<fn(&mut C, &[A]), LookError<D>> {
        if input.is_empty() {
            if let Some(finalizer) = self.finalizer {
                return Ok(finalizer);
            } else {
                return Err(LookError::FinalizerDoesNotExist);
            }
        }
        if let Some(handler) = self.map.get(&input[0]) {
            let mut advance_output = 0;
            if let Some(decider) = handler.decider {
                match decider(&input[1..], output) {
                    Decision::Accept(res) => {
                        advance_output = res;
                    }
                    Decision::Deny(res) => {
                        return Err(LookError::DeciderDenied(res));
                    }
                }
            }
            if input.len() > advance_output && output.len() >= advance_output {
                return handler.lookup(&input[1+advance_output..], &mut output[advance_output..]);
            } else {
                return Err(LookError::DeciderAdvancedTooFar);
            }
        }
        Err(LookError::UnknownMapping)
    }
}

// ---

fn x() {}
#[rustfmt::skip]
const SPEC: &[(&[(&'static str, Option<fn(&str) -> bool>)], fn())] = &[
    (&[("log", None)], x),
];

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    // ---

    type Accept = bool;
    type Context = u32;

    fn add_one(ctx: &mut Context, s: &[Accept]) {
        *ctx += 1;
    }

    // ---

    #[test]
    fn single_mapping() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
        let mut output = [true; 10];
        let handler = mapping.lookup(&["add-one"], &mut output).unwrap();
        let mut ctx = 123;
        handler(&mut ctx, &output);
        assert_eq![124, ctx];
    }

    #[test]
    fn mapping_does_not_exist() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        let mut output = [true; 0];
        if let Err(err) = mapping.lookup(&["add-one"], &mut output) {
            assert_eq![LookError::UnknownMapping, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn overlapping_decider_fails() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Deny(())
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
        assert_eq![Err(RegError::DeciderAlreadyExists), mapping.register((&[("add-one", Some(decide))], add_one))];
    }

    #[test]
    fn sequences_decider_succeeds() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Deny(())
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", Some(decide))], add_one)).unwrap();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
    }

    #[test]
    fn decider_of_one() {
        fn decide(_: &[&str], out: &mut [Accept]) -> Decision<()> {
            out[0] = true;
            Decision::Accept(1)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", Some(decide))], add_one)).unwrap();

        let mut output = [false; 3];
        let handler = mapping.lookup(&["add-one", "123"], &mut output).unwrap();
        assert_eq![true, output[0]];
        assert_eq![false, output[1]];
        assert_eq![false, output[2]];
    }

    #[test]
    fn decider_of_two_overrun() {
        fn decide(_: &[&str], out: &mut [Accept]) -> Decision<()> {
            out[0] = true;
            out[1] = true;
            Decision::Accept(2)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", Some(decide))], add_one)).unwrap();

        let mut output = [false; 3];
        if let Err(err) = mapping.lookup(&["add-one", "123"], &mut output) {
            assert_eq![LookError::DeciderAdvancedTooFar , err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn decider_of_two() {
        fn decide(_: &[&str], out: &mut [Accept]) -> Decision<()> {
            out[0] = true;
            out[1] = true;
            Decision::Accept(2)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", Some(decide))], add_one)).unwrap();

        let mut output = [false; 3];
        mapping.lookup(&["add-one", "123", "456"], &mut output);
        assert_eq![true, output[0]];
        assert_eq![true, output[1]];
        assert_eq![false, output[2]];
    }

    #[test]
    fn decider_of_two_short_output() {
        fn decide(_: &[&str], out: &mut [Accept]) -> Decision<()> {
            Decision::Accept(2)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", Some(decide))], add_one)).unwrap();

        let mut output = [false; 1];
        if let Err(err) = mapping.lookup(&["add-one", "123", "456"], &mut output) {
            assert_eq![LookError::DeciderAdvancedTooFar, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn decider_of_many() {
        fn decide(input: &[&str], out: &mut [i32]) -> Decision<()> {
            if out.len() >= input.len() {
                for (idx, i) in input.iter().enumerate() {
                    let number = i.parse::<i32>();
                    if let Ok(number) = number {
                        out[idx] = number;
                    } else {
                        return Decision::Deny(());
                    }
                }
            }
            Decision::Accept(input.len())
        }
        fn do_sum(ctx: &mut u32, out: &[i32]) {
            for i in out {
                *ctx += *i as u32;
            }
        }
        let mut mapping: Mapping<i32, (), Context> = Mapping::new();
        mapping.register((&[("sum", Some(decide))], do_sum)).unwrap();

        let mut output = [0; 3];
        let handler = mapping.lookup(&["sum", "123", "456", "789"], &mut output).unwrap();

        let mut ctx = 0;
        handler(&mut ctx, &output);
        assert_eq![1368, ctx];
    }

    #[test]
    fn nested() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Accept(0)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("lorem", None), ("ipsum", None), ("dolor", None)], add_one)).unwrap();

        let mut output = [false; 0];
        mapping.lookup(&["lorem", "ipsum", "dolor"], &mut output).unwrap();
        if let Err(err) = mapping.lookup(&["lorem", "ipsum", "dolor", "exceed"], &mut output) {
            assert_eq![LookError::UnknownMapping, err];
        } else {
            assert![false];
        }
        if let Err(err) = mapping.lookup(&["lorem", "ipsum"], &mut output) {
            assert_eq![LookError::FinalizerDoesNotExist, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn finalizer_at_multiple_levels() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Accept(0)
        }
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("lorem", None), ("ipsum", None), ("dolor", None)], add_one)).unwrap();
        mapping.register((&[("lorem", None), ("ipsum", None)], add_one)).unwrap();

        let mut output = [false; 0];
        mapping.lookup(&["lorem", "ipsum", "dolor"], &mut output).unwrap();
        if let Err(err) = mapping.lookup(&["lorem", "ipsum", "dolor", "exceed"], &mut output) {
            assert_eq![LookError::UnknownMapping, err];
        } else {
            assert![false];
        }
        mapping.lookup(&["lorem", "ipsum"], &mut output).unwrap();
    }

    // ---

    #[bench]
    fn lookup_speed(b: &mut Bencher) {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("lorem", None), ("ipsum", None), ("dolor", None)], add_one)).unwrap();
        let mut output = [false; 0];
        b.iter(|| {
            mapping.lookup(black_box(&["lorem", "ipsum", "dolor"]), &mut output).unwrap();
        });
    }
}
