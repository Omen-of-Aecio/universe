use crate::{decision::Decision, feedback::Feedback, types::Type};
use cmdmat::{self, LookError, Mapping, RegError, Spec};
use either::Either;
use metac::{Data, Evaluate, ParseError};

/// The virtual machine that runs commands
///
/// The virtual machine interprets strings and provides an output. It operates on the strings
/// according to the specified mapping table, which can be manipulated via [register] and
/// [register_many].
pub struct Evaluator<'a, C> {
    mapping: Mapping<'a, Type, Decision, C>,
    context: C,
}

impl<'a, C> Evaluator<'a, C> {
    pub fn new(context: C) -> Self {
        Self {
            mapping: Mapping::default(),
            context,
        }
    }

    pub fn context(&self) -> &C {
        &self.context
    }

    pub fn register(&mut self, spec: Spec<'_, 'a, Type, Decision, C>) -> Result<(), RegError> {
        self.mapping.register(spec)
    }

    pub fn register_many(
        &mut self,
        spec: &[Spec<'_, 'a, Type, Decision, C>],
    ) -> Result<(), RegError> {
        self.mapping.register_many(spec)
    }

    // Parse subcommands recursively into a vector of strings, fail with feedback otherwise
    fn parse_subcommands(&mut self, cmds: &[Data]) -> Result<Vec<String>, Feedback> {
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
                            Ok(Feedback::Ok(string)) => {
                                content.push(string);
                            }
                            Ok(ref res @ Feedback::Help(_)) => {
                                return Err(res.clone());
                            }
                            Ok(ref res @ Feedback::Err(_)) => {
                                return Err(res.clone());
                            }
                            Err(ParseError::DanglingLeftParenthesis) => {
                                return Err(Feedback::Err("Dangling left parenthesis".into()));
                            }
                            Err(ParseError::PrematureRightParenthesis) => {
                                return Err(Feedback::Err("Right parenthesis encountered with no matching left parenthesis".into()));
                            }
                        }
                    }
                }
            }
        }
        Ok(content)
    }
}

impl<'a, C> Evaluate<Feedback> for Evaluator<'a, C> {
    fn evaluate(&mut self, commands: &[Data]) -> Feedback {
        let content = match self.parse_subcommands(commands) {
            Ok(content) => content,
            Err(err) => return err,
        };
        let content_ref = content.iter().map(|s| &s[..]).collect::<Vec<_>>();

        if let Some(front) = content_ref.first() {
            if *front == "autocomplete" {
                match self.mapping.partial_lookup(&content_ref[1..]) {
                    Ok(Either::Left(mapping)) => {
                        let mut col = mapping
                            .get_direct_keys()
                            .map(|k| {
                                let mut s = String::new() + k.literal;
                                if k.decider.is_some() {
                                    s += " ";
                                }
                                s += if k.decider.is_some() {
                                    k.decider.unwrap().description
                                } else {
                                    ""
                                };
                                s += if k.finalizer.is_some() {
                                    " (final)"
                                } else {
                                    ""
                                };
                                s
                            })
                            .collect::<Vec<_>>();
                        if col.is_empty() {
                            return Feedback::Ok("No more handlers".into());
                        } else {
                            col.sort();
                            return Feedback::Ok(col.join(", "));
                        }
                    }
                    Ok(Either::Right(name)) => {
                        return Feedback::Ok(name.into());
                    }
                    Err(err) => {
                        return lookerr_to_evalres(err, true);
                    }
                }
            }
        }

        let res = self.mapping.lookup(&content_ref[..]);
        match res {
            Ok(fin) => {
                let res = fin.0(&mut self.context, &fin.1);
                match res {
                    Ok(res) => Feedback::Ok(res),
                    Err(res) => Feedback::Err(res),
                }
            }
            Err(err) => lookerr_to_evalres(err, false),
        }
    }
}

fn lookerr_to_evalres(err: LookError<Decision>, allow_help: bool) -> Feedback {
    match err {
        LookError::DeciderAdvancedTooFar => Feedback::Err("Decider advanced too far".into()),
        LookError::DeciderDenied(desc, Decision::Err(decider)) => {
            Feedback::Err(format!["Expected {} but got: {}", desc, decider])
        }
        LookError::DeciderDenied(desc, Decision::Help(help)) => {
            if allow_help {
                Feedback::Help(help)
            } else {
                Feedback::Err(format!["Expected {} but got denied: {}", desc, help])
            }
        }
        LookError::FinalizerDoesNotExist => Feedback::Err("Finalizer does not exist".into()),
        LookError::UnknownMapping(token) => {
            Feedback::Err(format!["Unrecognized mapping: {}", token])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::*;
    use crossbeam_channel::{bounded, Sender};

    #[test]
    fn evaluate_simple() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("".into())
        }

        eval.register((&[("call", None)], handler)).unwrap();
        assert_eq![
            Feedback::Ok("".into()),
            eval.interpret_single("call").unwrap()
        ];
    }

    #[test]
    fn evaluate_any_f32() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();
        assert_eq![
            Feedback::Err("Expected <f32> but got: Too few elements: []".into()),
            eval.interpret_single("call").unwrap()
        ];
        assert_eq![
            Feedback::Ok("".into()),
            eval.interpret_single("call 3.14159").unwrap()
        ];
        assert_eq![
            Feedback::Ok("".into()),
            eval.interpret_single("call 3").unwrap()
        ];
        assert_eq![
            Feedback::Err("Expected <f32> but got: alpha".into()),
            eval.interpret_single("call alpha").unwrap()
        ];
    }

    #[test]
    fn a_command_triggers_a_channel() {
        let (tx, rx) = bounded(1);

        let mut eval = Evaluator::new(tx);

        fn handler(context: &mut Sender<f32>, args: &[Type]) -> Result<String, String> {
            match args[0] {
                Type::F32(number) => context.send(number).unwrap(),
                _ => panic!["Input was not an f32"],
            }
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();

        eval.interpret_single("call 3.14159").unwrap();

        assert_eq![3.14159, rx.recv().unwrap()]
    }
}