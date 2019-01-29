use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::{Editor, Helper};
use std::borrow::Cow::{self, Borrowed, Owned};
use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::net::TcpStream;

static HISTORY_FILE: &str = ".gsh-history.txt";

static COLORED_PROMPT: &'static str = "\x1b[1;32m>>\x1b[0m ";

static PROMPT: &'static str = ">> ";

struct AutoComplete(RefCell<TcpStream>);

impl Completer for AutoComplete {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        Ok((pos, vec![]))
    }
}

impl Hinter for AutoComplete {
    fn hint(&self, line: &str, _: usize) -> Option<String> {
        if line.chars().last() == Some(' ') {
            // ---
            self.0
                .borrow_mut()
                .write(format!["autocomplete {}\n", line].as_bytes());
            self.0.borrow_mut().flush();
            // ---
            let mut buffer = [0; 512];
            self.0.borrow_mut().read(&mut buffer);
            if let Ok(buffer) = std::str::from_utf8(&buffer) {
                return Some(String::from(" ") + buffer.into());
            }
        }
        None
    }
}

impl Helper for AutoComplete {}
impl Highlighter for AutoComplete {}

// ---

fn main() -> io::Result<()> {
    let listener = TcpStream::connect("127.0.0.1:32931")?;
    let stdout = io::stdout();
    let mut output = stdout.lock();

    writeln![output, "gsh: GameShell v0.1.0 at your service (? for help)"]?;
    let mut rl = Editor::<AutoComplete>::new();
    rl.set_helper(Some(AutoComplete(RefCell::new(listener))));
    if rl.load_history(HISTORY_FILE).is_err() {
        writeln![
            output,
            "No previous history, using `{}` in current directory",
            HISTORY_FILE
        ]?;
    }

    write![output, "> "]?;
    output.flush()?;
    loop {
        let line = rl.readline("> ");
        match line {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());
                let mut listener = rl.helper_mut().unwrap().0.borrow_mut();
                // ---
                listener.write(line.as_bytes())?;
                listener.write(b"\n")?;
                listener.flush()?;
                // ---
                let mut buffer = [0; 512];
                listener.read(&mut buffer)?;
                if let Ok(buffer) = std::str::from_utf8(&buffer) {
                    writeln![output, "{} ", buffer]?;
                    output.flush()?;
                }
                // ---
                write![output, "> "]?;
                output.flush()?;
            }
            Err(ReadlineError::Interrupted) => {
                writeln![output, "^C"]?;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                writeln![output, "Error: {:?}", err]?;
                break;
            }
        }
    }
    rl.save_history(HISTORY_FILE).unwrap();
    Ok(())
}
