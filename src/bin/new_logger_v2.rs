#![feature(test)]
extern crate test; // Required for testing, even though extern crate is no longer needed in the 2018 version, this is a special case
#[macro_use]
extern crate slog;

use chrono::prelude::*;
use std::{
    fmt,
    fmt::Display,
    marker::{Send, Sync},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, RecvError, TrySendError},
        Arc, Mutex,
    },
    thread,
};

type Logger<C> = LoggerV2Async<C>;

/// The fastest logger in the west
///
/// A very simple logger that uses a custom structure and
/// a logging level. No further assumptions are made about
/// the structure or nature of the logging message, thus
/// making it extremely cheap to send messages to the logger
/// thread.
struct LoggerV2Async<C: Display + Send> {
    log_channel: mpsc::SyncSender<(u8, C)>,
    log_channel_full_count: Arc<AtomicUsize>,
    level: Arc<AtomicUsize>,
}

fn logger_thread<C: Display + Send, W: std::io::Write>(rx: mpsc::Receiver<(u8, C)>, dropped: Arc<AtomicUsize>, mut writer: W) {
    loop {
        match rx.recv() {
            Ok(msg) => {
                writeln![writer, "{}: {:03}: {}", Local::now(), msg.0, msg.1];
            }
            Err(RecvError { .. }) => {
                break;
            }
        }
        let dropped_messages = dropped.swap(0, Ordering::Relaxed);
        if dropped_messages > 0 {
            println![
                "{}: {:03}: {}, {}={}",
                Local::now(),
                0,
                "logger dropped messages due to channel overflow",
                "count",
                dropped_messages
            ];
        }
    }
}

impl<C: 'static + Display + Send> LoggerV2Async<C> {
    /// Create a logger object and spawn a logging thread
    ///
    /// The logger object is the interface to write stuff to
    /// the logger. The logger thread is in the background,
    /// waiting for messages to print out. Once all logger objects
    /// are dropped, the thread will die.
    pub fn spawn() -> (Logger<C>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::sync_channel(30_000);
        let full_count = Arc::new(AtomicUsize::new(0));
        let level = Arc::new(AtomicUsize::new(128));
        let ex = std::io::stdout();
        (
            Logger {
                log_channel: tx,
                log_channel_full_count: full_count.clone(),
                level,
            },
            thread::spawn(move || logger_thread(rx, full_count, ex)),
        )
    }

    pub fn set_log_level(&mut self, level: u8) {
        self.level.store(level as usize, Ordering::Relaxed);
    }

    pub fn log(&mut self, level: u8, message: C) -> bool {
        if level as usize <= self.level.load(Ordering::Relaxed) {
            match self.log_channel.try_send((level, message)) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    self.log_channel_full_count.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Err(TrySendError::Disconnected(_)) => false,
            }
        } else {
            false
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn trace(&mut self, message: C) -> bool {
        false
    }

    #[cfg(debug_assertions)]
    pub fn trace(&mut self, message: C) -> bool {
        self.log(255, message)
    }

    pub fn debug(&mut self, message: C) -> bool {
        self.log(192, message)
    }

    pub fn info(&mut self, message: C) -> bool {
        self.log(128, message)
    }

    pub fn warn(&mut self, message: C) -> bool {
        self.log(64, message)
    }

    pub fn error(&mut self, message: C) -> bool {
        self.log(0, message)
    }
}

// ---

struct LoggerAsyncClientDoesNotCheckLogLevel<C: Display + Send> {
    log_channel: std::sync::mpsc::SyncSender<(u8, C)>,
    log_channel_full_count: Arc<AtomicUsize>,
    level: Arc<AtomicUsize>,
}

fn logger_thread_check_loglevel<C: Display + Send>(
    rx: mpsc::Receiver<(u8, C)>,
    dropped: Arc<AtomicUsize>,
    level: Arc<AtomicUsize>,
) {
    loop {
        match rx.recv() {
            Ok(msg) => {
                if msg.0 as usize <= level.load(Ordering::Relaxed) {
                    println!["{}: {:03}: {}", Local::now(), msg.0, msg.1];
                }
            }
            Err(RecvError { .. }) => {
                break;
            }
        }
        let dropped_messages = dropped.swap(0, Ordering::Relaxed);
        if dropped_messages > 0 {
            println![
                "{}: {:03}: {}, {}: {}",
                Local::now(),
                0,
                "logger dropped messages due to channel overflow",
                "count",
                dropped_messages
            ];
        }
    }
}

impl<C: 'static + Display + Send> LoggerAsyncClientDoesNotCheckLogLevel<C> {
    /// Create a logger object and spawn a logging thread
    ///
    /// The logger object is the interface to write stuff to
    /// the logger. The logger thread is in the background,
    /// waiting for messages to print out. Once all logger objects
    /// are dropped, the thread will die.
    pub fn spawn() -> (Self, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::sync_channel(30_000);
        let full_count = Arc::new(AtomicUsize::new(0));
        let level = Arc::new(AtomicUsize::new(128));
        (
            Self {
                log_channel: tx,
                log_channel_full_count: full_count.clone(),
                level: level.clone(),
            },
            thread::spawn(move || logger_thread_check_loglevel(rx, full_count, level)),
        )
    }

    pub fn set_log_level(&mut self, level: u8) {
        self.level.store(level as usize, Ordering::Relaxed);
    }

    pub fn log(&mut self, level: u8, message: C) -> bool {
        match self.log_channel.try_send((level, message)) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.log_channel_full_count.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn trace(&mut self, message: C) -> bool {
        false
    }

    #[cfg(debug_assertions)]
    pub fn trace(&mut self, message: C) -> bool {
        self.log(255, message)
    }

    pub fn debug(&mut self, message: C) -> bool {
        self.log(192, message)
    }

    pub fn info(&mut self, message: C) -> bool {
        self.log(128, message)
    }

    pub fn warn(&mut self, message: C) -> bool {
        self.log(64, message)
    }

    pub fn error(&mut self, message: C) -> bool {
        self.log(0, message)
    }
}

enum MyMessages {
    Kek,
    Rek { x: i32 },
}

impl Display for MyMessages {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MyMessages::Kek => write![f, "{}", "Kek"],
            MyMessages::Rek { x } => write![f, "{}", x],
        }
    }
}

fn main() {
    let (mut log, thr) = Logger::<MyMessages>::spawn();
    log.log(123, MyMessages::Kek);
    thr.join().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[bench]
    fn message_not_sent(b: &mut Bencher) {
        let (mut log, thread) = Logger::<MyMessages>::spawn();
        log.set_log_level(0);
        b.iter(|| {
            black_box(log.log(black_box(223), black_box(MyMessages::Kek)));
        });
        std::mem::drop(log);
        thread.join().unwrap();
    }

    #[bench]
    fn message_sent_hittema(b: &mut Bencher) {
        let (mut log, thread) = Logger::<MyMessages>::spawn();
        log.set_log_level(black_box(230));
        b.iter(|| {
            black_box(log.log(black_box(123), black_box(MyMessages::Rek { x: 312 })));
        });
        std::mem::drop(log);
        thread.join().unwrap();
    }

    #[bench]
    fn message_urself(b: &mut Bencher) {
        b.iter(|| {
            black_box(println!["{} {}", black_box("Something may be done"), black_box(1230.123)]);
        });
    }

    use slog::{Drain, Level};
    #[bench]
    fn message_slog(b: &mut Bencher) {
        let decorator = slog_term::PlainDecorator::new(std::io::stdout());
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, o!("version" => "0.5"));
        b.iter(|| {
            black_box(trace![log, "{}", "Something may be done"]);
        });
    }
}
