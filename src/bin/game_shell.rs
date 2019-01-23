use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;

fn main() -> io::Result<()> {
    let mut listener = TcpStream::connect("127.0.0.1:32931").unwrap();
    let mut buffer = String::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle = stdin.lock();
    let mut output = stdout.lock();

    writeln![output, "gsh: GameShell v1.0.0"];
    for line in handle.lines() {
        let line = line?;
        writeln![output, "writing line to remote"];
        listener.write(line.as_bytes())?;
        writeln![output, "flushing to remote"];
        listener.flush()?;
        writeln![output, "reading from remote"];
        buffer = String::new();
        listener.read_to_string(&mut buffer)?;
        // output.write(buffer.as_bytes())?;
        writeln![output, "{}\n> ", buffer];
        output.flush()?;
    }
    Ok(())
}
