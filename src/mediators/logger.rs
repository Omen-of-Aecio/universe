use crate::glocals::{EntryPointLogger, LogMessage, Threads};
use std::{
    collections::HashMap,
    sync::{
        mpsc::{RecvError, TrySendError},
        Arc, Mutex,
    },
};

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
            match threads.log_channel_full_count.lock() {
                Ok(mut guard) => {
                    *guard += 1;
                }
                Err(error @ std::sync::PoisonError { .. }) => {
                    println!["Logger lock is poisoned: {:#?}", error];
                }
            }
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
    writeln![
        out,
        "{:03}: {:?}: {:?}, {:#?}",
        loglevel, context, message, kvpairs
    ];
}

// ---

fn check_if_messages_were_lost(s: &mut EntryPointLogger) {
    match s.log_channel_full_count.lock() {
        Ok(mut overfilled_buffer_count) => {
            if *overfilled_buffer_count > 0 {
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
                *overfilled_buffer_count = 0;
            }
        }
        Err(error @ std::sync::PoisonError { .. }) => {
            write_message_out(
                s.writer,
                LogMessage {
                    loglevel: 0,
                    context: "LGGR".into(),
                    message: "Logger unable to acquire failed counter".into(),
                    kvpairs: {
                        let mut map = HashMap::new();
                        map.insert("reason".into(), error.to_string());
                        map
                    },
                },
            );
        }
    }
}

// ---

pub fn entry_point_logger(mut s: EntryPointLogger) {
    loop {
        match s.receiver.recv() {
            Ok(msg @ LogMessage { .. }) => {
                write_message_out(s.writer, msg);
            }
            Err(RecvError {}) => {
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
            message: "Logger exiting".into(),
            kvpairs: HashMap::new(),
        },
    );
}

pub fn create_logger(s: &mut Threads) {
    let (tx, rx) = std::sync::mpsc::sync_channel(1000);
    let buffer_full_count = Arc::new(Mutex::new(0));
    s.log_channel = Some(tx);
    s.log_channel_full_count = buffer_full_count.clone();
    s.logger = Some(std::thread::spawn(move || {
        entry_point_logger(EntryPointLogger {
            log_channel_full_count: buffer_full_count,
            receiver: rx,
            writer: &mut std::io::stdout(),
        });
    }));
    log(s, 128, "MAIN", "Logger thread created", &[]);
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
                127,
                "TEST",
                "This message does not arrive, and the failed count will _not_ be incremented",
                &[]
            )
        ];
        match threads.log_channel_full_count.lock() {
            Ok(ref guard) => {
                assert_eq![0usize, **guard];
            }
            Err(error @ std::sync::PoisonError { .. }) => {
                assert![false, "The lock should not be poisoned"];
            }
        };
    }

    #[test]
    fn single_message_arrives() {
        let mut threads = Threads::default();
        create_logger(&mut threads);
        assert_eq![
            true,
            log(&mut threads, 127, "TEST", "This message will arrive", &[])
        ];
        match threads.log_channel_full_count.lock() {
            Ok(ref guard) => {
                assert_eq![0usize, **guard];
            }
            Err(error @ std::sync::PoisonError { .. }) => {
                assert![false, "The lock should not be poisoned"];
            }
        };
    }
}
