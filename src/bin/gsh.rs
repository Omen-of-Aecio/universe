use metac::PartialParse;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::{Editor, Helper};
use std::cell::{Cell, RefCell};
use std::io::{self, Read, Write};
use std::net::TcpStream;

static HISTORY_FILE: &str = ".gsh-history.txt";

struct AutoComplete(RefCell<TcpStream>, Cell<bool>);

impl Completer for AutoComplete {
    type Candidate = Pair;

    fn complete(&self, _: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        Ok((pos, vec![]))
    }
}

impl Hinter for AutoComplete {
    fn hint(&self, line: &str, _: usize) -> Option<String> {
        if !self.1.get() || line.find('(').is_some() {
            return None;
        }
        if line.ends_with(' ') {
            // ---
            self.0
                .borrow_mut()
                .write_all(format!["autocomplete {}\n", line].as_bytes())
                .unwrap();
            self.0.borrow_mut().flush().unwrap();
            // ---
            let mut buffer = [0; 512];
            let _ = self.0.borrow_mut().read(&mut buffer).unwrap();
            if let Ok(buffer) = std::str::from_utf8(&buffer) {
                return Some(String::from(" ") + buffer);
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
    let mut parse = PartialParse::default();

    writeln![output, "gsh: GameShell v0.1.0 at your service (? for help)"]?;
    let mut rl = Editor::<AutoComplete>::new();
    rl.set_helper(Some(AutoComplete(RefCell::new(listener), Cell::new(true))));
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
        let probe = rl.helper_mut().unwrap().1.get();
        let line = rl.readline(if probe { "> " } else { "| " });
        match line {
            Ok(line) => {
                for ch in line.bytes() {
                    match parse.parse_increment(ch) {
                        Some(x) => {
                            rl.helper_mut().unwrap().1.set(x);
                        }
                        None => {
                            rl.helper_mut().unwrap().1.set(false);
                        }
                    }
                }
                match parse.parse_increment(b'\n') {
                    Some(x) => {
                        rl.helper_mut().unwrap().1.set(x);
                    }
                    None => {
                        rl.helper_mut().unwrap().1.set(false);
                    }
                }
                rl.add_history_entry(line.as_ref());
                {
                    let mut listener = rl.helper_mut().unwrap().0.borrow_mut();
                    // ---
                    listener.write_all(line.as_bytes())?;
                    listener.write_all(b"\n")?;
                    listener.flush()?;
                    // ---
                }
                if !rl.helper_mut().unwrap().1.get() {
                    continue;
                }
                // ---
                let mut buffer = [0; 512];
                let mut listener = rl.helper_mut().unwrap().0.borrow_mut();
                let _ = listener.read(&mut buffer)?;
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
