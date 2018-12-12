//! A threaded logger
//!
//! The logger described in this file uses no global state, it all goes through
//! the glocal `universe::glocals::Threads`.
//!
//! When we create a logger, a thread is spawned and a bounded buffer is allocated
//! for messages to be sent to the logger. This is also accompanied by a counter for
//! failed messages - when the buffer is full, the message is discarded and the counter
//! incremented. Such a counter is useful so the logger can report how many messages
//! were dropped.
//!
//! For using the logger, all you need to use is the `log` function.
//! It takes a logging level and log context, which are used to limit messages
//! if needed. The log levels range from 0 to 255, where 0 will _always_ be logged
//! (unless the queue is full). The context ought to be a short descriptor
//! of where this log message came from the semantic sense.
//!
//! ```
//! use universe::{glocals::Threads, mediators::logger::{create_logger, log}};
//! fn main() {
//!     // Allocate the structure for storing threads from `universe`
//!     let mut threads = Threads::default();
//!
//!     // Start the logger thread, storing the context and queue inside the `threads` structure
//!     create_logger(&mut threads);
//!
//!     // Log a message by pushing it to the logger's queue
//!     // Returning true if there was an active queue with sufficient space
//!     // and false if the message could not be sent.
//!     assert![
//!         log(
//!             // Threads variable so we can communicate to the logger thread
//!             &mut threads,
//!
//!             // The logging level
//!             128,
//!
//!             // The logging context
//!             "TEST",
//!
//!             // An arbitrary message describing the event
//!             "This message does not arrive, and the failed count will _not_ be incremented",
//!
//!             // A key-value map of items, also printed by the logger
//!             // Mainly useful when reporting state
//!             &[("key", "value")]
//!         )
//!     ];
//!
//!     // Close the logging channel, thus signalling to the logger thread that
//!     // we are finished
//!     threads.log_channel = None;
//!
//!     // Join the logger thread with this one
//!     threads.logger.unwrap().join();
//! }
//! ```
use crate::glocals::{EntryPointLogger, LogMessage, Threads};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{RecvError, TrySendError},
        Arc, Mutex,
    },
};

static CHANNEL_SIZE: usize = 1000;

/// Log a message
pub fn log<T: Clone + Into<String>>(
    threads: &mut Threads,
    level: u8,
    context: T,
    message: T,
    key_value_map: &[(T, T)],
) -> bool {
    match threads.log_channel.as_mut().map(move |x| {
        x.try_send(LogMessage {
            loglevel: level,
            context: context.into(),
            message: message.into(),
            kvpairs: key_value_map
                .iter()
                .map(|(k, v)| (k.clone().into(), v.clone().into()))
                .collect(),
        })
    }) {
        Some(Ok(())) => true,
        Some(Err(TrySendError::Full(LogMessage { .. }))) => {
            threads
                .log_channel_full_count
                .fetch_add(1, Ordering::Relaxed);
            false
        }
        Some(Err(TrySendError::Disconnected(LogMessage { .. }))) => {
            std::mem::replace(&mut threads.log_channel, None);
            std::mem::replace(&mut threads.logger, None);
            false
        }
        None => false,
    }
}

// ---

fn write_message_out(out: &mut dyn std::io::Write, msg: LogMessage) {
    let LogMessage {
        loglevel,
        context,
        message,
        kvpairs,
    } = msg;
    let _ = writeln![
        out,
        "{:03}: {:?}: {:?}, {:#?}",
        loglevel, context, message, kvpairs
    ];
}

// ---

fn check_if_messages_were_lost(s: &mut EntryPointLogger) {
    let overfilled_buffer_count = s.log_channel_full_count.swap(0, Ordering::Relaxed);
    if overfilled_buffer_count > 0 {
        write_message_out(
            s.writer,
            LogMessage {
                loglevel: 0,
                context: "LGGR".into(),
                message: "Messages lost due to filled buffer".into(),
                kvpairs: {
                    let mut map = HashMap::new();
                    map.insert("messages_lost".into(), overfilled_buffer_count.to_string());
                    map
                },
            },
        );
    }
}

// ---

fn entry_point_logger(mut s: EntryPointLogger) {
    write_message_out(
        s.writer,
        LogMessage {
            loglevel: 128,
            context: "LGGR".into(),
            message: "Logger thread spawned".into(),
            kvpairs: HashMap::new(),
        },
    );
    loop {
        match s.receiver.recv() {
            Ok(msg @ LogMessage { .. }) => {
                write_message_out(s.writer, msg);
            }
            Err(RecvError) => {
                break;
            }
        }
        check_if_messages_were_lost(&mut s);
    }
    write_message_out(
        s.writer,
        LogMessage {
            loglevel: 128,
            context: "LGGR".into(),
            message: "Logger thread exited".into(),
            kvpairs: HashMap::new(),
        },
    );
}

/// Spawn a logger thread with a writer to stdout
pub fn create_logger(s: &mut Threads) {
    let (tx, rx) = std::sync::mpsc::sync_channel(CHANNEL_SIZE);
    let buffer_full_count = Arc::new(AtomicUsize::new(0));
    s.log_channel = Some(tx);
    s.log_channel_full_count = buffer_full_count.clone();
    s.logger = Some(std::thread::spawn(move || {
        entry_point_logger(EntryPointLogger {
            log_channel_full_count: buffer_full_count.clone(),
            receiver: rx,
            writer: &mut std::io::stdout(),
        });
    }));
}

/// Spawn a logger thread with a custom writer
pub fn create_logger_with_writer<T: 'static + std::io::Write + Send>(
    s: &mut Threads,
    mut writer: T,
) {
    let (tx, rx) = std::sync::mpsc::sync_channel(CHANNEL_SIZE);
    let buffer_full_count = Arc::new(AtomicUsize::new(0));
    s.log_channel = Some(tx);
    s.log_channel_full_count = buffer_full_count.clone();
    s.logger = Some(std::thread::spawn(move || {
        entry_point_logger(EntryPointLogger {
            log_channel_full_count: buffer_full_count.clone(),
            receiver: rx,
            writer: &mut writer,
        });
    }));
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_logger_thread_fails_sending_message() {
        let mut threads = Threads::default();
        assert_eq![
            false,
            log(
                &mut threads,
                128,
                "TEST",
                "This message does not arrive, and the failed count will _not_ be incremented",
                &[]
            )
        ];
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
    }

    #[test]
    fn single_message_arrives() {
        let mut threads = Threads::default();
        create_logger(&mut threads);
        assert_eq![
            true,
            log(&mut threads, 128, "TEST", "This message will arrive", &[])
        ];
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
    }

    // ---

    struct Veclog {
        pub data: Arc<Mutex<Vec<u8>>>,
    }

    impl Veclog {
        fn new() -> Veclog {
            Veclog {
                data: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl std::io::Write for Veclog {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            match self.data.lock() {
                Ok(ref mut g) => {
                    g.append(&mut Vec::from(data));
                    Ok(data.len())
                }
                Err(_) => {
                    panic!["Unable to lock writer"];
                }
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // ---

    #[test]
    fn single_message_arrives_confirm() {
        let mut threads = Threads::default();
        let veclog = Veclog::new();
        let arc = veclog.data.clone();
        create_logger_with_writer(&mut threads, veclog);
        assert_eq![
            true,
            log(&mut threads, 128, "TEST", "This message will arrive", &[])
        ];
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
        threads.log_channel = None;
        threads.logger.map(|x| x.join());
        assert_eq![
            r#"128: "LGGR": "Logger thread spawned", {}
128: "TEST": "This message will arrive", {}
128: "LGGR": "Logger thread exited", {}
"#
            .as_bytes(),
            arc.lock().unwrap().as_slice()
        ];
    }

    #[test]
    fn single_message_arrives_confirm_with_key_values() {
        let mut threads = Threads::default();
        let veclog = Veclog::new();
        let arc = veclog.data.clone();
        create_logger_with_writer(&mut threads, veclog);
        assert![log(
            &mut threads,
            128,
            "TEST",
            "This message will arrive",
            &[("key", "value")]
        )];
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
        threads.log_channel = None;
        threads.logger.map(|x| x.join());
        assert_eq![
            r#"128: "LGGR": "Logger thread spawned", {}
128: "TEST": "This message will arrive", {
    "key": "value"
}
128: "LGGR": "Logger thread exited", {}
"#
            .as_bytes(),
            arc.lock().unwrap().as_slice()
        ];
    }

    #[test]
    fn single_message_arrives_confirm_with_duplicate_key_values() {
        let mut threads = Threads::default();
        let veclog = Veclog::new();
        let arc = veclog.data.clone();
        create_logger_with_writer(&mut threads, veclog);
        assert![log(
            &mut threads,
            128,
            "TEST",
            "This message will arrive",
            &[("key", "value"), ("key", "value"), ("key", "value")]
        )];
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
        threads.log_channel = None;
        threads.logger.map(|x| x.join());
        assert_eq![
            r#"128: "LGGR": "Logger thread spawned", {}
128: "TEST": "This message will arrive", {
    "key": "value"
}
128: "LGGR": "Logger thread exited", {}
"#
            .as_bytes(),
            arc.lock().unwrap().as_slice()
        ];
    }

    #[test]
    fn dropped_messages() {
        let mut threads = Threads::default();
        let veclog = Veclog::new();
        let arc = veclog.data.clone();
        create_logger_with_writer(&mut threads, veclog);
        {
            let _guard = arc.lock();
            for _ in 0..CHANNEL_SIZE {
                assert![log(
                    &mut threads,
                    128,
                    "TEST",
                    "This message will arrive",
                    &[]
                )];
            }
            assert_eq![
                0usize,
                threads.log_channel_full_count.load(Ordering::Relaxed)
            ];
            log(
                &mut threads,
                128,
                "TEST",
                "This message will not arrive",
                &[],
            );
            assert_eq![
                1usize,
                threads.log_channel_full_count.load(Ordering::Relaxed)
            ];
            log(
                &mut threads,
                128,
                "TEST",
                "This message will not arrive",
                &[],
            );
            assert_eq![
                2usize,
                threads.log_channel_full_count.load(Ordering::Relaxed)
            ];
        }
        threads.log_channel = None;
        threads.logger.map(|x| x.join());
        assert_eq![
            0usize,
            threads.log_channel_full_count.load(Ordering::Relaxed)
        ];
    }
}
