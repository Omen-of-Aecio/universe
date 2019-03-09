#![feature(test)]
extern crate test;

use std::collections::HashMap;

// ---

/// Spec: The command specification format
pub type Spec<'b, 'a, A, D, C> = (
    &'b [(&'static str, Option<&'a Decider<A, D>>)],
    fn(&mut C, &[A]),
);

/// A decision contains information about token consumption by the decider
///
/// If the decider has accepted the tokens, it will return an `Accept(usize)`, if it failed to
/// parse interpret the tokens, it will return a deny value.
#[derive(Debug, PartialEq)]
pub enum Decision<D> {
    Accept(usize),
    Deny(D),
}

/// A decider is a function taking in a sequence of tokens and an output array
///
/// It puts tokens into the output array according to interal logic and returns how many elements
/// it has consumed. If it could not process the input tokens it will return a `Deny`, containing
/// the reason for denying. Calling a decider with &[] should always yield its deny value.
pub struct Decider<A, D> {
    description: &'static str,
    decider: fn(&[&str], &mut [A]) -> Decision<D>,
}

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

pub struct Mapping<'a, A, D, C> {
    map: HashMap<&'a str, Mapping<'a, A, D, C>>,
    decider: Option<&'a Decider<A, D>>,
    finalizer: Option<fn(&mut C, &[A])>,
}

impl<'a, A, D, C> Mapping<'a, A, D, C> {
    pub fn new() -> Mapping<'a, A, D, C> {
        Mapping {
            map: HashMap::new(),
            decider: None,
            finalizer: None,
        }
    }

    pub fn register_many<'b>(&mut self, spec: &[Spec<'b, 'a, A, D, C>]) -> Result<(), RegError> {
        for subspec in spec {
            self.register(subspec.clone())?;
        }
        Ok(())
    }

    fn register<'b>(&mut self, spec: Spec<'b, 'a, A, D, C>) -> Result<(), RegError> {
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
            entry.register((&spec.0[1..], spec.1))?;
        } else {
            let mut mapping = Mapping::<A, D, C> {
                map: HashMap::new(),
                decider: decider,
                finalizer: None,
            };
            mapping.register((&spec.0[1..], spec.1))?;
            self.map.insert(key, mapping);
        }
        Ok(())
    }

    pub fn lookup(
        &self,
        input: &[&str],
        output: &mut [A],
    ) -> Result<fn(&mut C, &[A]), LookError<D>> {
        if input.is_empty() {
            if let Some(finalizer) = self.finalizer {
                return Ok(finalizer);
            } else {
                return Err(LookError::FinalizerDoesNotExist);
            }
        }
        if let Some(handler) = self.map.get(&input[0]) {
            let mut advance_output = 0;
            if let Some(ref decider) = handler.decider {
                match (decider.decider)(&input[1..], output) {
                    Decision::Accept(res) => {
                        advance_output = res;
                    }
                    Decision::Deny(res) => {
                        return Err(LookError::DeciderDenied(res));
                    }
                }
            }
            if input.len() > advance_output && output.len() >= advance_output {
                return handler.lookup(&input[1 + advance_output..], &mut output[advance_output..]);
            } else {
                return Err(LookError::DeciderAdvancedTooFar);
            }
        }
        Err(LookError::UnknownMapping)
    }

    pub fn get_direct_keys(&self) -> impl Iterator<Item = (&&str, Option<&'static str>, bool)> {
        self.map
            .iter()
            .map(|(k, v)| (k, v.decider.map(|d| d.description), v.finalizer.is_some()))
    }

    pub fn partial_lookup<'b>(
        &'b self,
        input: &'b [&str],
        output: &mut [A],
    ) -> Result<&'b Mapping<'a, A, D, C>, LookError<D>> {
        if input.is_empty() {
            return Ok(self);
        }
        if let Some(handler) = self.map.get(&input[0]) {
            let mut advance_output = 0;
            if let Some(ref decider) = handler.decider {
                match (decider.decider)(&input[1..], output) {
                    Decision::Accept(res) => {
                        advance_output = res;
                    }
                    Decision::Deny(res) => {
                        return Err(LookError::DeciderDenied(res));
                    }
                }
            }
            if input.len() > advance_output && output.len() >= advance_output {
                return handler
                    .partial_lookup(&input[1 + advance_output..], &mut output[advance_output..]);
            } else {
                return Err(LookError::DeciderAdvancedTooFar);
            }
        }
        Err(LookError::UnknownMapping)
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    // ---

    type Accept = bool;
    type Context = u32;

    fn add_one(ctx: &mut Context, _: &[Accept]) {
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
        let mapping: Mapping<Accept, (), Context> = Mapping::new();
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

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
        assert_eq![
            Err(RegError::DeciderAlreadyExists),
            mapping.register((&[("add-one", Some(&DECIDE))], add_one))
        ];
    }

    #[test]
    fn sequences_decider_fails() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Deny(())
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();
        if let Err(err) = mapping.register((&[("add-one", None)], add_one)) {
            assert_eq![RegError::FinalizerAlreadyExists, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn decider_of_one() {
        fn decide(_: &[&str], out: &mut [Accept]) -> Decision<()> {
            out[0] = true;
            Decision::Accept(1)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let mut output = [false; 3];
        mapping.lookup(&["add-one", "123"], &mut output).unwrap();
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

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let mut output = [false; 3];
        if let Err(err) = mapping.lookup(&["add-one", "123"], &mut output) {
            assert_eq![LookError::DeciderAdvancedTooFar, err];
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

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let mut output = [false; 3];
        mapping
            .lookup(&["add-one", "123", "456"], &mut output)
            .unwrap();
        assert_eq![true, output[0]];
        assert_eq![true, output[1]];
        assert_eq![false, output[2]];
    }

    #[test]
    fn decider_of_two_short_output() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Accept(2)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

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

        const DECIDE: Decider<i32, ()> = Decider {
            description: "",
            decider: decide,
        };

        fn do_sum(ctx: &mut u32, out: &[i32]) {
            for i in out {
                *ctx += *i as u32;
            }
        }
        let mut mapping: Mapping<i32, (), Context> = Mapping::new();
        mapping
            .register((&[("sum", Some(&DECIDE))], do_sum))
            .unwrap();

        let mut output = [0; 3];
        let handler = mapping
            .lookup(&["sum", "123", "456", "789"], &mut output)
            .unwrap();

        let mut ctx = 0;
        handler(&mut ctx, &output);
        assert_eq![1368, ctx];
    }

    #[test]
    fn nested() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();

        let mut output = [false; 0];
        mapping
            .lookup(&["lorem", "ipsum", "dolor"], &mut output)
            .unwrap();
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
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        mapping
            .register((&[("lorem", None), ("ipsum", None)], add_one))
            .unwrap();

        let mut output = [false; 0];
        mapping
            .lookup(&["lorem", "ipsum", "dolor"], &mut output)
            .unwrap();
        if let Err(err) = mapping.lookup(&["lorem", "ipsum", "dolor", "exceed"], &mut output) {
            assert_eq![LookError::UnknownMapping, err];
        } else {
            assert![false];
        }
        mapping.lookup(&["lorem", "ipsum"], &mut output).unwrap();
    }

    #[test]
    fn partial_lookup() {
        fn decide(_: &[&str], _: &mut [Accept]) -> Decision<()> {
            Decision::Accept(0)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "Do nothing",
            decider: decide,
        };

        fn consume_decide(input: &[&str], _: &mut [Accept]) -> Decision<()> {
            if input.is_empty() {
                Decision::Deny(())
            } else {
                Decision::Accept(1)
            }
        }

        const CONSUME_DECIDE: Decider<Accept, ()> = Decider {
            description: "Consume a single element, regardless of what it is",
            decider: consume_decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        mapping
            .register((&[("lorem", None), ("ipsum", None)], add_one))
            .unwrap();
        mapping
            .register((&[("mirana", None), ("ipsum", Some(&DECIDE))], add_one))
            .unwrap();
        mapping
            .register((
                &[("consume", Some(&CONSUME_DECIDE)), ("dummy", None)],
                add_one,
            ))
            .unwrap();

        let mut output = [false; 0];
        let part = mapping
            .partial_lookup(&["lorem", "ipsum"], &mut output)
            .unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"dolor", None, true), key];

        let part = mapping.partial_lookup(&["lorem"], &mut output).unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"ipsum", None, true), key];

        let part = mapping.partial_lookup(&["mirana"], &mut output).unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"ipsum", Some("Do nothing"), true), key];

        let mut output = [false; 1];
        let part = mapping
            .partial_lookup(&["consume", "123"], &mut output)
            .unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"dummy", None, true), key];

        let part = mapping.partial_lookup(&["consume"], &mut output);
        if let Err(err) = part {
            assert_eq![LookError::DeciderDenied(()), err];
        } else {
            assert![false];
        }
    }

    // ---

    #[bench]
    fn lookup_speed(b: &mut Bencher) {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::new();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        let mut output = [false; 0];
        b.iter(|| {
            mapping
                .lookup(black_box(&["lorem", "ipsum", "dolor"]), &mut output)
                .unwrap();
        });
    }
}
