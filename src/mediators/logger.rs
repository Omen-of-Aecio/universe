use glocals::{EntryPointLogger, LogMessage, Threads};
use std::{
    collections::HashMap,
    sync::mpsc::{RecvError, TrySendError},
};

pub fn log<T: Clone + Into<String>>(
    threads: &mut Threads,
    level: u8,
    context: T,
    message: T,
    key_value_map: &[(T, T)],
) {
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
        Some(Ok(())) => {}
        Some(Err(TrySendError::Full(LogMessage { .. }))) => {
            match threads.log_channel_full_count.lock() {
                Ok(mut guard) => {
                    *guard += 1;
                }
                Err(error @ std::sync::PoisonError { .. }) => {
                    println!["Logger lock is poisoned: {:#?}", error];
                }
            }
        }
        Some(Err(TrySendError::Disconnected(LogMessage { .. }))) => {
            std::mem::replace(&mut threads.log_channel, None);
            std::mem::replace(&mut threads.logger, None);
        }
        None => {}
    }
}

// ---

fn write_message_out(msg: LogMessage) {
    let LogMessage {
        loglevel,
        context,
        message,
        kvpairs,
    } = msg;
    println![
        "{:03}: {:?}: {:?}, {:#?}",
        loglevel, context, message, kvpairs
    ];
}

// ---

fn check_if_messages_were_lost(s: &mut EntryPointLogger) {
    match s.log_channel_full_count.lock() {
        Ok(mut overfilled_buffer_count) => {
            if *overfilled_buffer_count > 0 {
                write_message_out(LogMessage {
                    loglevel: 0,
                    context: "LGGR".into(),
                    message: "Messages lost due to filled buffer".into(),
                    kvpairs: {
                        let mut map = HashMap::new();
                        map.insert("messages_lost".into(), overfilled_buffer_count.to_string());
                        map
                    },
                });
                *overfilled_buffer_count = 0;
            }
        }
        Err(error @ std::sync::PoisonError { .. }) => {
            write_message_out(LogMessage {
                loglevel: 0,
                context: "LGGR".into(),
                message: "Logger unable to acquire failed counter".into(),
                kvpairs: {
                    let mut map = HashMap::new();
                    map.insert("reason".into(), error.to_string());
                    map
                },
            });
        }
    }
}

// ---

pub fn entry_point_logger(mut s: EntryPointLogger) {
    loop {
        match s.receiver.recv() {
            Ok(msg @ LogMessage { .. }) => {
                write_message_out(msg);
            }
            Err(RecvError {}) => {
                break;
            }
        }
        check_if_messages_were_lost(&mut s);
    }
    write_message_out(LogMessage {
        loglevel: 128,
        context: "LGGR".into(),
        message: "Logger exiting".into(),
        kvpairs: HashMap::new(),
    });
}
