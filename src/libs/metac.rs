//! A zero-allocation, no_std command language parser for custom interpreters.
//!
//! Here's an example:
//! ```
//! use universe::libs::metac::{Data, Evaluate};
//! fn main() {
//!     struct Eval { }
//!     impl Evaluate<()> for Eval {
//!         fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> () {
//!             for command in commands {
//!                 match command {
//!                     Data::Atom(string) => {}
//!                     Data::Command(command_string) => {}
//!                 }
//!                 println!["{:?}", command];
//!             }
//!         }
//!     }
//!
//!     let mut eval = Eval { };
//!     eval.interpret("Hello (World 1 2) 3").unwrap();
//! }
//! ```
//!
//! All you need to do is implement trait `Evaluate` on a structure, then, you call `interpret`
//! on said struct.
//!
//! This allows you to put anything in your struct, you decide how the interpreter is going to
//! work. What this library does for you is parse the input into two things:
//!
//! 1. Atoms - Basically strings
//! 2. Commands - `()`-enclosed text.
//!
//! Note that commands are not expanded by metac, you have to do this yourself.
//! A nested command like `something (alpha (beta gamma))` will be parsed as `[Atom("something"),
//! Command("alpha (beta gamma)")]`.
//! Your evaluator decides whether it will parse the contents or use it for something different.
//!
//! # More interesting example #
//! ```
//! use universe::libs::metac::{Data, Evaluate};
//! use std::collections::HashMap;
//! fn main() {
//!     struct Eval {
//!         hashmap: HashMap<String, String>,
//!     }
//!     impl Eval {
//!         fn register(&mut self, key: &str, value: &str) {
//!             self.hashmap.insert(key.into(), value.into());
//!         }
//!     }
//!     impl Evaluate<String> for Eval {
//!         fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> String {
//!             if commands.len() == 2 {
//!                 if let Data::Atom("Get") = commands[0] {
//!                     if let Data::Atom(key) = commands[1] {
//!                         return self.hashmap.get(key).unwrap().clone();
//!                     }
//!                 }
//!             }
//!             "".into()
//!         }
//!     }
//!
//!     let mut eval = Eval { hashmap: HashMap::new() };
//!     eval.register("my-variable", "my-value");
//!     assert_eq!["my-value", eval.interpret("Get my-variable").unwrap()];
//! }
//! ```
//! From here we can set up a more complex environment, callbacks, etc. It's all up to the
//! implementer.
#![feature(test)]
#![no_std]
extern crate test;

/// Size of the buffer used during parsing
const BUFFER_SIZE: usize = 32;

/// Distinguishes atoms from commands
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Data<'a> {
    Atom(&'a str),
    Command(&'a str),
}

/// Parsing error struct
///
/// The errors represented here are single-line oriented. For instance, a
/// `DanglingLeftParenthesis` refers to an unclosed `(` in the line.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    DanglingLeftParenthesis,
    InputLineTooLong,
    PrematureRightParenthesis,
}

/// Interpreter trait
///
/// Central trait to add the interpreter to your custom evaluator
pub trait Evaluate<T: Default> {
    /// Evaluate a single command
    ///
    /// Commands are line-separated pieces of code turned into fixed data
    /// segments.
    fn evaluate<'a>(&mut self, commands: &[Data<'a>]) -> T;
    /// Repeatedly calls evaluate for each line in the input
    ///
    /// Also performs some setup to be able to call evaluate.
    fn interpret(&mut self, input: &str) -> Result<T, ParseError> {
        let mut data = [Data::Atom(&input[0..]); BUFFER_SIZE]; // TODO Use MaybeUninit here to prevent default initialization, currently only on nightly
        let mut result = T::default();
        for line in input.lines() {
            let size = parse(line, &mut data)?;
            result = self.evaluate(&data[0..size]);
        }
        Ok(result)
    }
}

/// Parse an input line into a classified output buffer
pub fn parse<'a>(line: &'a str, output: &mut [Data<'a>; BUFFER_SIZE]) -> Result<usize, ParseError> {
    let mut lparen_stack = 0;
    let mut buff_idx = 0;
    let (mut start, mut stop) = (0, 0);
    for i in line.chars() {
        if lparen_stack > 0 {
            if i == '(' {
                lparen_stack += 1;
                stop += 1;
            } else if i == ')' {
                lparen_stack -= 1;
                if lparen_stack == 0 {
                    output[buff_idx] = Data::Command(&line[start..stop]);
                    buff_idx += 1;
                    if buff_idx >= BUFFER_SIZE {
                        return Err(ParseError::InputLineTooLong);
                    }
                    stop += 1;
                    start = stop;
                } else {
                    stop += 1;
                }
            } else {
                stop += 1;
            }
        } else {
            if i.is_whitespace() {
                if start != stop {
                    output[buff_idx] = Data::Atom(&line[start..stop]);
                    buff_idx += 1;
                    if buff_idx >= BUFFER_SIZE {
                        return Err(ParseError::InputLineTooLong);
                    }
                }
                stop += 1;
                start = stop;
            } else if i == '(' {
                lparen_stack += 1;
                stop += 1;
                start = stop;
            } else if i == ')' {
                return Err(ParseError::PrematureRightParenthesis);
            } else {
                stop += 1;
            }
        }
    }
    if lparen_stack > 0 {
        return Err(ParseError::DanglingLeftParenthesis);
    }
    if start != stop {
        output[buff_idx] = Data::Atom(&line[start..stop]);
        buff_idx += 1;
    }
    Ok(buff_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn empty_parse() {
        let line = "";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        let count = parse(line, &mut data).unwrap();

        assert_eq![0, count];
    }

    #[test]
    fn basic_parse() {
        let line = "Set Log Level 0";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        let count = parse(line, &mut data).unwrap();

        assert_eq![4, count];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Atom("0"), data[3]];
    }

    #[test]
    fn subcommand_parse() {
        let line = "Set Log Level (Get Log Level)";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        let count = parse(line, &mut data).unwrap();

        assert_eq![4, count];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Command("Get Log Level"), data[3]];
    }

    // ---

    #[test]
    fn fail_parse_too_long() {
        let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Suspendisse viverra porta lacus, quis pretium nibh lacinia at. Mauris convallis sed lectus nec dapibus. Interdum et malesuada fames ac ante ipsum primis in faucibus. Nulla vulputate sapien dui. Aliquam finibus ante ut purus facilisis, in sagittis tortor varius. Nunc interdum fermentum libero, et egestas arcu convallis sed. Maecenas nec diam a libero vulputate suscipit. Phasellus ac dolor ut nunc ultricies fringilla. Maecenas sed feugiat nunc. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae. Quisque tincidunt metus ut ante dapibus, et molestie massa varius. Sed ultrices sapien sed mauris congue pretium. Pellentesque bibendum hendrerit sagittis. Vestibulum dignissim egestas feugiat. Ut porttitor et massa a posuere. Ut euismod metus a sem facilisis ullamcorper. Proin pharetra placerat enim";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        assert_eq![
            ParseError::InputLineTooLong,
            parse(line, &mut data).unwrap_err()
        ];
    }

    #[test]
    fn fail_parse_closing_parenthesis() {
        let line = "command ) will not work";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        assert_eq![
            ParseError::PrematureRightParenthesis,
            parse(line, &mut data).unwrap_err()
        ];
    }

    #[test]
    fn fail_parse_dangling_open_parenthesis() {
        let line = "command ( will not work";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        assert_eq![
            ParseError::DanglingLeftParenthesis,
            parse(line, &mut data).unwrap_err()
        ];
    }

    // ---

    #[test]
    fn evaluator() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };
        eval.interpret("Hello World").unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret("This is an example (command)").unwrap();
        assert_eq![2, eval.invoked];
    }

    #[test]
    fn recursive_evaluator() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, commands: &[Data<'a>]) {
                self.invoked += 1;
                for command in commands {
                    match command {
                        Data::Atom(_) => {}
                        Data::Command(string) => {
                            self.interpret(string).unwrap();
                        }
                    }
                }
            }
        }
        let mut eval = Eval { invoked: 0 };
        eval.interpret("Hello World").unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret("This is an example of substitution: (command)")
            .unwrap();
        assert_eq![3, eval.invoked];
        eval.interpret(
            "We can substitute more than once: (my command), anywhere: (another command here)",
        )
        .unwrap();
        assert_eq![6, eval.invoked];
        eval.interpret("We can also nest substitutions: (my (recursive (command) here))")
            .unwrap();
        assert_eq![10, eval.invoked];
    }

    // ---

    #[bench]
    fn empty_evaluate(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret(black_box("unknown reasonably long command"))
                .unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_very_short(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret(black_box("x")).unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_very_long(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret(black_box("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris tristique massa magna, eget consectetur dui posuere congue. Etiam rhoncus porttitor enim, eget malesuada ante dapibus eget. Duis neque dui, tincidunt ut varius")).unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_with_subsistution(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret(black_box("unknown (some) (long command 1)"))
                .unwrap();
        });
    }

    #[bench]
    fn increment_evaluate(b: &mut Bencher) {
        struct Eval {
            pub invoke: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoke += 1;
            }
        }
        let mut eval = Eval { invoke: 0 };
        b.iter(|| {
            eval.interpret(black_box("unknown reasonably long command"))
                .unwrap();
        });
    }

    #[bench]
    fn parse_very_long(b: &mut Bencher) {
        let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris tristique massa magna, eget consectetur dui posuere congue. Etiam rhoncus porttitor enim, eget malesuada ante dapibus eget. Duis neque dui, tincidunt ut varius";
        let mut data = [Data::Atom(&line[0..]); BUFFER_SIZE];
        b.iter(|| {
            parse(black_box(line), &mut data).unwrap();
        });
    }
}
