use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;

fn main() -> io::Result<()> {
    let mut listener = TcpStream::connect("127.0.0.1:32931").unwrap();
    let mut buffer = String::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle = stdin.lock();
    let mut output = stdout.lock();

    writeln![output, "gsh: GameShell v0.1.0 at your service (? for help)"];
    write![output, "> "];
    output.flush()?;
    for line in handle.lines() {
        let line = line?;
        listener.write(line.as_bytes())?;
        listener.flush()?;
        buffer = String::new();
        let mut buffer = [0; 128];
        listener.read(&mut buffer)?;
        // output.write(buffer.as_bytes())?;
        if let Ok(buffer) = std::str::from_utf8(&buffer) {
            writeln![output, "{} ", buffer];
            output.flush()?;
        }
        write![output, "> "];
        output.flush()?;
    }
    writeln![output];
    Ok(())
}
