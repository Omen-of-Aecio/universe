//! A command matching engine
//!
//! This library matches a list of input parameters such as:
//! `["example", "input", "123"]`
//! to a handler that is able to handle these inputs.
//!
//! The handlers are registered using the `Spec` (specification) format:
//! ```
//! use cmdmat::{Decider, Decision, Spec, SVec};
//!
//! type Accept = i32;
//! type Deny = String;
//! type Context = i32;
//!
//! fn handler(_ctx: &mut Context, _args: &[Accept]) -> Result<String, String> {
//!     Ok("".into())
//! }
//!
//! fn accept_integer(input: &[&str], out: &mut SVec<Accept>) -> Decision<Deny> {
//!     if input.len() != 1 {
//!         return Decision::Deny("Require exactly 1 input".into());
//!     }
//!     if let Ok(num) = input[0].parse::<Accept>() {
//!         out.push(num);
//!         Decision::Accept(1)
//!     } else {
//!         Decision::Deny("Unable to get number".into())
//!     }
//! }
//!
//! const DEC: Decider<Accept, Deny> = Decider {
//!     description: "<i32>",
//!     decider: accept_integer,
//! };
//!
//! const SPEC: Spec<Accept, Deny, Context> = (&[("example", None), ("input", Some(&DEC))], handler);
//!
//! ```
//!
//! In the above the `SPEC` variable defines a path to get to the `handler`, requiring first
//! `"example"` with no validator `None`, then followed by `"input"` which takes a single
//! integer.
//!
//! If the validator `accept_integer` fails, then the command lookup will also fail.
//!
//! The `Spec`s will be collected inside a `Mapping`, where lookup will happen among a tree of
//! merged `Spec`s.
//!
//! The reason we have a separate literal string and validator is to make it easy and unambiguous
//! to find the next node in the search tree. If we only used validators (which can be completely
//! arbitrary), then we can not sort a tree to make searching `O(log n)`. These fixed literal
//! search points also give us a good way to debug commands if they happen to not match anything.
//!
//! Here is an example with actual lookup where we call a handler:
//! (Unfortunately, a bit of setup is required.)
//!
//! ```
//! use cmdmat::{Decider, Decision, Mapping, Spec, SVec};
//!
//! // The accept type is the type enum containing accepted tokens, parsed into useful formats
//! // the list of accepted input is at last passed to the finalizer
//! #[derive(Debug)]
//! enum Accept {
//!     I32(i32),
//! }
//!
//! // Deny is the type returned by a decider when it denies an input (the input is invalid)
//! type Deny = String;
//!
//! // The context is the type on which "finalizers" (the actual command handler) will run
//! type Context = i32;
//!
//! // This is a `spec` (command specification)
//! const SPEC: Spec<Accept, Deny, Context> = (&[("my-command-name", Some(&DEC))], print_hello);
//!
//! fn print_hello(_ctx: &mut Context, args: &[Accept]) -> Result<String, String> {
//!     println!["Hello world!"];
//!     assert_eq![1, args.len()];
//!     println!["The args I got: {:?}", args];
//!     Ok("".into())
//! }
//!
//! // This decider accepts only a single number
//! fn decider_function(input: &[&str], out: &mut SVec<Accept>) -> Decision<Deny> {
//!     if input.is_empty() {
//!         return Decision::Deny("No argument provided".into());
//!     }
//!     let num = input[0].parse::<i32>();
//!     if let Ok(num) = num {
//!         out.push(Accept::I32(num));
//!         Decision::Accept(1)
//!     } else {
//!         Decision::Deny("Number is not a valid i32".into())
//!     }
//! }
//!
//! const DEC: Decider<Accept, Deny> = Decider {
//!     description: "<i32>",
//!     decider: decider_function,
//! };
//!
//! fn main() {
//!     let mut mapping = Mapping::default();
//!     mapping.register(SPEC).unwrap();
//!
//!     let handler = mapping.lookup(&["my-command-name", "123"]);
//!
//!     match handler {
//!         Ok((func, buf)) => {
//!             let mut ctx = 0i32;
//!             func(&mut ctx, &buf); // prints hello world
//!         }
//!         Err(look_err) => {
//!             println!["{:?}", look_err];
//!             assert![false];
//!         }
//!     }
//! }
//! ```
//!
//! This library also allows partial lookups and iterating over the direct descendants in order
//! to make autocomplete easy to implement for terminal interfaces.
//! ```
//! use cmdmat::{Decider, Decision, Mapping, Spec, SVec};
//!
//! #[derive(Debug)]
//! enum Accept {
//!     I32(i32),
//! }
//! type Deny = String;
//! type Context = i32;
//!
//! const SPEC: Spec<Accept, Deny, Context> =
//!     (&[("my-command-name", Some(&DEC)), ("something", None)], print_hello);
//!
//! fn print_hello(_ctx: &mut Context, args: &[Accept]) -> Result<String, String> {
//!     Ok("".into())
//! }
//!
//! fn decider_function(input: &[&str], out: &mut SVec<Accept>) -> Decision<Deny> {
//!     if input.is_empty() {
//!         return Decision::Deny("No argument provided".into());
//!     }
//!     let num = input[0].parse::<i32>();
//!     if let Ok(num) = num {
//!         out.push(Accept::I32(num));
//!         Decision::Accept(1)
//!     } else {
//!         Decision::Deny("Number is not a valid i32".into())
//!     }
//! }
//!
//! const DEC: Decider<Accept, Deny> = Decider {
//!     description: "<i32>",
//!     decider: decider_function,
//! };
//!
//! fn main() {
//!     let mut mapping = Mapping::default();
//!     mapping.register(SPEC).unwrap();
//!
//!     // When a decider is "next-up", we get its description
//!     // We can't know in advance what the decider will consume because it is arbitrary code,
//!     // so we will have to trust its description to be valuable.
//!     let decider_desc = mapping.partial_lookup(&["my-command-name"]).unwrap().right().unwrap();
//!     assert_eq!["<i32>", decider_desc];
//!
//!     // In this case the decider succeeded during the partial lookup, so the next step in the
//!     // tree is the "something" node.
//!     let mapping = mapping.partial_lookup(&["my-command-name", "123"]).unwrap().left().unwrap();
//!     let (name, decider, has_handler) = mapping.get_direct_keys().next().unwrap();
//!     assert_eq!["something", *name];
//!     assert_eq![None, decider];
//!     assert_eq![true, has_handler];
//! }
//! ```
#![feature(test)]
extern crate test;

use either::Either;
use smallvec::SmallVec;
use std::collections::HashMap;

// ---

/// A specific-sized small vector
pub type SVec<A> = SmallVec<[A; 8]>;

/// The command specification format
pub type Spec<'b, 'a, A, D, C> = (
    &'b [(&'static str, Option<&'a Decider<A, D>>)],
    Finalizer<A, C>,
);

/// A finalizer is the function that runs to handle the entirety of the command after it has been
/// verified by the deciders.
pub type Finalizer<A, C> = fn(&mut C, &[A]) -> Result<String, String>;

/// Finalizer with associated vector of arguments
pub type FinWithArgs<'o, A, C> = (Finalizer<A, C>, SVec<A>);

/// Either a mapping or a descriptor of a mapping
pub type MapOrDesc<'a, 'b, A, D, C> = Either<&'b Mapping<'a, A, D, C>, &'a str>;

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
/// the reason for denying.
pub struct Decider<A, D> {
    pub description: &'static str,
    pub decider: fn(&[&str], &mut SVec<A>) -> Decision<D>,
}

/// Errors we can get by registering specs.
#[derive(Debug, PartialEq)]
pub enum RegError {
    DeciderAlreadyExists,
    FinalizerAlreadyExists,
}

/// Errors happening during lookup of a command.
#[derive(Debug, PartialEq)]
pub enum LookError<D> {
    DeciderAdvancedTooFar,
    DeciderDenied(String, D),
    FinalizerDoesNotExist,
    UnknownMapping(String),
}

// ---

/// Node in the matching tree
///
/// A `Mapping` is used to interface with `cmdmat`. Each node defines a point in a command tree,
/// containing subcommands, deciders for argument parsing, and a finalizer if this mapping can be
/// run.
pub struct Mapping<'a, A, D, C> {
    map: HashMap<&'a str, Mapping<'a, A, D, C>>,
    decider: Option<&'a Decider<A, D>>,
    finalizer: Option<Finalizer<A, C>>,
}

impl<'a, A, D, C> Default for Mapping<'a, A, D, C> {
    fn default() -> Self {
        Mapping {
            map: HashMap::new(),
            decider: None,
            finalizer: None,
        }
    }
}

impl<'a, A, D, C> Mapping<'a, A, D, C> {
    /// Register many command specs at once, see `register` for more detail
    pub fn register_many<'b>(&mut self, spec: &[Spec<'b, 'a, A, D, C>]) -> Result<(), RegError> {
        for subspec in spec {
            self.register(subspec.clone())?;
        }
        Ok(())
    }

    /// Register a single command specification into the tree
    ///
    /// The specification will be merged with existing command specifications, and may not
    /// overwrite commands with new deciders or finalizers. The overriding decider must be `None`
    /// to avoid an error.
    pub fn register<'b>(&mut self, spec: Spec<'b, 'a, A, D, C>) -> Result<(), RegError> {
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
                decider,
                finalizer: None,
            };
            mapping.register((&spec.0[1..], spec.1))?;
            self.map.insert(key, mapping);
        }
        Ok(())
    }

    /// Looks up a command and runs deciders to collect all arguments
    pub fn lookup(&self, input: &[&str]) -> Result<FinWithArgs<A, C>, LookError<D>> {
        let mut output = SVec::<A>::new();
        match self.lookup_internal(input, &mut output) {
            Ok((finalizer, _advance)) => Ok((finalizer, output)),
            Err(err) => Err(err),
        }
    }

    /// Looks up a command and runs deciders to collect all arguments
    fn lookup_internal(
        &self,
        input: &[&str],
        output: &mut SVec<A>,
    ) -> Result<(Finalizer<A, C>, usize), LookError<D>> {
        if input.is_empty() {
            if let Some(finalizer) = self.finalizer {
                return Ok((finalizer, 0));
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
                        return Err(LookError::DeciderDenied(decider.description.into(), res));
                    }
                }
            }
            if input.len() > advance_output {
                let res = handler.lookup_internal(&input[1 + advance_output..], output);
                if let Ok(mut res) = res {
                    res.1 += advance_output;
                    return Ok(res);
                } else {
                    return res;
                }
            } else {
                return Err(LookError::DeciderAdvancedTooFar);
            }
        }
        Err(LookError::UnknownMapping(input[0].to_string()))
    }

    /// Iterator over the current `Mapping` keys: containing subcommands
    pub fn get_direct_keys(&self) -> impl Iterator<Item = (&&str, Option<&'static str>, bool)> {
        self.map
            .iter()
            .map(|(k, v)| (k, v.decider.map(|d| d.description), v.finalizer.is_some()))
    }

    pub fn partial_lookup<'b>(
        &'b self,
        input: &'b [&str],
    ) -> Result<MapOrDesc<'a, 'b, A, D, C>, LookError<D>> {
        let mut output = SVec::<A>::new();
        self.partial_lookup_internal(input, &mut output)
    }

    /// Perform a partial lookup, useful for autocompletion
    ///
    /// Due to the node structure of `Mapping`, this function returns either a `Mapping` or a
    /// `&str` describing the active decider.
    fn partial_lookup_internal<'b>(
        &'b self,
        input: &'b [&str],
        output: &mut SVec<A>,
    ) -> Result<MapOrDesc<'a, 'b, A, D, C>, LookError<D>> {
        if input.is_empty() {
            return Ok(Either::Left(self));
        }
        if let Some(handler) = self.map.get(&input[0]) {
            let mut advance_output = 0;
            if let Some(ref decider) = handler.decider {
                if input.len() == 1 {
                    return Ok(Either::Right(decider.description));
                }
                match (decider.decider)(&input[1..], output) {
                    Decision::Accept(res) => {
                        advance_output = res;
                    }
                    Decision::Deny(res) => {
                        return Err(LookError::DeciderDenied(decider.description.into(), res));
                    }
                }
            }
            if input.len() > advance_output {
                return handler.partial_lookup_internal(&input[1 + advance_output..], output);
            } else {
                return Err(LookError::DeciderAdvancedTooFar);
            }
        }
        Err(LookError::UnknownMapping(input[0].to_string()))
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

    fn add_one(ctx: &mut Context, _: &[Accept]) -> Result<String, String> {
        *ctx += 1;
        Ok("".into())
    }

    // ---

    #[test]
    fn single_mapping() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
        let handler = mapping.lookup(&["add-one"]).unwrap();
        assert_eq![0, handler.1.len()];
        let mut ctx = 123;
        handler.0(&mut ctx, &handler.1).unwrap();
        assert_eq![124, ctx];
    }

    #[test]
    fn mapping_does_not_exist() {
        let mapping: Mapping<Accept, (), Context> = Mapping::default();
        if let Err(err) = mapping.lookup(&["add-one"]) {
            assert_eq![LookError::UnknownMapping("add-one".into()), err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn overlapping_decider_fails() {
        fn decide(_: &[&str], _: &mut SVec<Accept>) -> Decision<()> {
            Decision::Deny(())
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping.register((&[("add-one", None)], add_one)).unwrap();
        assert_eq![
            Err(RegError::DeciderAlreadyExists),
            mapping.register((&[("add-one", Some(&DECIDE))], add_one))
        ];
    }

    #[test]
    fn sequences_decider_fails() {
        fn decide(_: &[&str], _: &mut SVec<Accept>) -> Decision<()> {
            Decision::Deny(())
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
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
        fn decide(_: &[&str], out: &mut SVec<Accept>) -> Decision<()> {
            out.push(true);
            Decision::Accept(1)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let out = mapping.lookup(&["add-one", "123"]).unwrap();
        assert_eq![1, out.1.len()];
        assert_eq![true, out.1[0]];
    }

    #[test]
    fn decider_of_two_overrun() {
        fn decide(_: &[&str], out: &mut SVec<Accept>) -> Decision<()> {
            out.push(true);
            out.push(true);
            Decision::Accept(2)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        if let Err(err) = mapping.lookup(&["add-one", "123"]) {
            assert_eq![LookError::DeciderAdvancedTooFar, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn decider_of_two() {
        fn decide(_: &[&str], out: &mut SVec<Accept>) -> Decision<()> {
            out.push(true);
            out.push(false);
            Decision::Accept(2)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let output = mapping.lookup(&["add-one", "123", "456"]).unwrap().1;
        assert_eq![2, output.len()];
        assert_eq![true, output[0]];
        assert_eq![false, output[1]];
    }

    #[test]
    fn decider_of_two_short_output() {
        fn decide(_: &[&str], _: &mut SVec<Accept>) -> Decision<()> {
            Decision::Accept(2)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "",
            decider: decide,
        };

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((&[("add-one", Some(&DECIDE))], add_one))
            .unwrap();

        let output = mapping.lookup(&["add-one", "123", "456"]).unwrap();
        assert_eq![0, output.1.len()];
    }

    #[test]
    fn decider_of_many() {
        fn decide(input: &[&str], out: &mut SVec<i32>) -> Decision<()> {
            for i in input.iter() {
                let number = i.parse::<i32>();
                if let Ok(number) = number {
                    out.push(number);
                } else {
                    return Decision::Deny(());
                }
            }
            Decision::Accept(input.len())
        }

        const DECIDE: Decider<i32, ()> = Decider {
            description: "",
            decider: decide,
        };

        fn do_sum(ctx: &mut u32, out: &[i32]) -> Result<String, String> {
            for i in out {
                *ctx += *i as u32;
            }
            Ok("".into())
        }
        let mut mapping: Mapping<i32, (), Context> = Mapping::default();
        mapping
            .register((&[("sum", Some(&DECIDE))], do_sum))
            .unwrap();

        let handler = mapping.lookup(&["sum", "123", "456", "789"]).unwrap();
        assert_eq![3, handler.1.len()];

        let mut ctx = 0;
        handler.0(&mut ctx, &handler.1).unwrap();
        assert_eq![1368, ctx];
    }

    #[test]
    fn nested() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();

        mapping.lookup(&["lorem", "ipsum", "dolor"]).unwrap();
        if let Err(err) = mapping.lookup(&["lorem", "ipsum", "dolor", "exceed"]) {
            assert_eq![LookError::UnknownMapping("exceed".into()), err];
        } else {
            assert![false];
        }
        if let Err(err) = mapping.lookup(&["lorem", "ipsum"]) {
            assert_eq![LookError::FinalizerDoesNotExist, err];
        } else {
            assert![false];
        }
    }

    #[test]
    fn finalizer_at_multiple_levels() {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        mapping
            .register((&[("lorem", None), ("ipsum", None)], add_one))
            .unwrap();

        mapping.lookup(&["lorem", "ipsum", "dolor"]).unwrap();
        if let Err(err) = mapping.lookup(&["lorem", "ipsum", "dolor", "exceed"]) {
            assert_eq![LookError::UnknownMapping("exceed".into()), err];
        } else {
            assert![false];
        }
        mapping.lookup(&["lorem", "ipsum"]).unwrap();
    }

    #[test]
    fn partial_lookup() {
        fn decide(_: &[&str], _: &mut SVec<Accept>) -> Decision<()> {
            Decision::Accept(0)
        }

        const DECIDE: Decider<Accept, ()> = Decider {
            description: "Do nothing",
            decider: decide,
        };

        fn consume_decide(input: &[&str], _: &mut SVec<Accept>) -> Decision<()> {
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

        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
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

        let part = mapping
            .partial_lookup(&["lorem", "ipsum"])
            .unwrap()
            .left()
            .unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"dolor", None, true), key];

        let part = mapping.partial_lookup(&["lorem"]).unwrap().left().unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"ipsum", None, true), key];

        let part = mapping.partial_lookup(&["mirana"]).unwrap().left().unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"ipsum", Some("Do nothing"), true), key];

        let part = mapping
            .partial_lookup(&["consume", "123"])
            .unwrap()
            .left()
            .unwrap();
        let key = part.get_direct_keys().next().unwrap();
        assert_eq![(&"dummy", None, true), key];

        let part = mapping
            .partial_lookup(&["consume"])
            .unwrap()
            .right()
            .unwrap();
        assert_eq!["Consume a single element, regardless of what it is", part];
    }

    // ---

    #[quickcheck_macros::quickcheck]
    fn default_contains_no_elements(strings: Vec<String>) -> bool {
        type Accept = ();
        type Deny = ();
        type Context = ();
        let strings_ref = strings.iter().map(|s| &s[..]).collect::<Vec<_>>();
        let mapping: Mapping<Accept, Deny, Context> = Mapping::default();
        match mapping.lookup(&strings_ref[..]) {
            Err(LookError::UnknownMapping(string)) => string == strings[0],
            Err(LookError::FinalizerDoesNotExist) => strings.is_empty(),
            _ => false,
        }
    }

    // ---

    #[bench]
    fn lookup_speed(b: &mut Bencher) {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        b.iter(|| {
            black_box(mapping.lookup(black_box(&["lorem", "ipsum", "dolor"]))).unwrap();
        });
    }

    #[bench]
    fn partial_lookup_speed(b: &mut Bencher) {
        let mut mapping: Mapping<Accept, (), Context> = Mapping::default();
        mapping
            .register((
                &[("lorem", None), ("ipsum", None), ("dolor", None)],
                add_one,
            ))
            .unwrap();
        b.iter(|| {
            mapping.partial_lookup(black_box(&["lorem"])).unwrap();
        });
    }
}
