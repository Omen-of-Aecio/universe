use glocals::{EntryPointLogger, LogMessage, Threads};
use std::{collections::HashMap, sync::mpsc::{RecvError, TrySendError}};

pub fn log<T: Into<String>>(
    threads: &mut Threads,
    level: u8,
    context: T,
    message: T,
    key_value_map: &[(String, String)],
) {
    match threads.log_channel.as_mut().map(move |x| {
        x.try_send(LogMessage {
            loglevel: level,
            context: context.into(),
            message: message.into(),
            kvpairs: key_value_map.iter().cloned().collect(),
        })
    }) {
        Some(Ok(())) => {}
        Some(Err(TrySendError::Full(LogMessage { .. }))) => {
            match threads.log_channel_full_count.lock() {
                Ok(mut guard) => {
                    *guard += 1;
                }
                Err(error @ std::sync::PoisonError { .. }) => {
                    println!["Lock is poisoned: {:#?}", error];
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

pub fn entry_point_logger(s: EntryPointLogger) {
    loop {
        match s.receiver.recv() {
            Ok(LogMessage {
                loglevel,
                context,
                message,
                kvpairs,
            }) => {
                println!["{}: {:?}: {:?}, {:#?}", loglevel, context, message, kvpairs];
            }
            Err(RecvError {}) => {
                break;
            }
        }
    }
    println!["128: \"LGGR\": \"Thread exiting\", {:#?}", HashMap::<String, String>::new()];
}
