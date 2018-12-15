#![feature(test)]
extern crate test; // Required for testing, even though extern crate is no longer needed in the 2018 version, this is a special case

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    marker::{Send, Sync},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{RecvError, TrySendError},
        Arc, Mutex,
    },
};

trait Logo: Clone + Debug + Eq + Hash + PartialEq + Send + Sync { }

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Context {
    Main,
    Lggr,
}

impl Logo for Context { }

#[derive(Default, Debug)]
struct LogMessage<T: Logo> {
    pub loglevel: u8,
    pub context: T,
    pub message: &'static str,
    pub kvpairs: BTreeMap<String, String>,
}

#[derive(Clone)]
struct Logger<C: Logo> {
    log_channel: Option<std::sync::mpsc::SyncSender<LogMessage<C>>>,
    log_channel_full_count: Arc<AtomicUsize>,
    log_contexts: Arc<Mutex<HashMap<C, u8>>>,
    current_context: C,
}

impl<C: 'static + Logo> Logger<C> {

    pub fn set_context(&mut self, context: C, value: u8) {
        let mut contexts = self.log_contexts.lock().unwrap();
        contexts.insert(context, value);
    }

    pub fn with_context(&self, context: C) -> Logger<C> {
        Logger {
            current_context: context,
            .. self.clone()
        }
    }

    pub fn new(context: C) -> (Logger<C>, std::thread::JoinHandle<()>) {
        let (tx, rx) = std::sync::mpsc::sync_channel(100);
        let full_count = Arc::new(AtomicUsize::new(0));
        let contexts = Arc::new(Mutex::new(HashMap::new()));
        (
            Logger {
                log_channel: Some(tx),
                log_channel_full_count: full_count.clone(),
                log_contexts: contexts.clone(),
                current_context: context,
            },
            std::thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(msg @ LogMessage { .. }) => {
                            let lock = contexts.lock().unwrap();
                            let value = lock.get(&msg.context).unwrap_or(&128);
                            if *value >= msg.loglevel {
                                write_message_out(&mut std::io::stdout(), msg);
                            }
                        }
                        Err(RecvError { .. }) => {
                            break;
                        }
                    }
                }
            })
        )
    }

    /// Log a message
    pub fn log<T: Clone + Into<String>>(
        &mut self,
        level: u8,
        message: &'static str,
        key_value_map: &[(&'static str, T)],
    ) -> bool {
        let this_context = self.current_context.clone();
        {
            let contexts = self.log_contexts.lock().unwrap();
            if level > *contexts.get(&this_context).unwrap_or(&128) {
                return false;
            }
        }
        match self.log_channel.as_mut().map(move |x| {
            x.try_send(LogMessage {
                loglevel: level,
                context: this_context,
                message: message,
                kvpairs: key_value_map
                    .iter()
                    .map(|(k, v)| (k.clone().into(), v.clone().into()))
                    .collect(),
            })
        }) {
            Some(Ok(())) => true,
            Some(Err(TrySendError::Full(LogMessage { .. }))) => {
               self 
                    .log_channel_full_count
                    .fetch_add(1, Ordering::Relaxed);
                false
            }
            Some(Err(TrySendError::Disconnected(LogMessage { .. }))) => {
                std::mem::replace(&mut self.log_channel, None);
                false
            }
            None => false,
        }
    }

}

fn write_message_out<C: Logo> (out: &mut dyn std::io::Write, msg: LogMessage<C>) {
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

fn main() {
    let (mut log, thread) = Logger::new(Context::Main);
    let mut l = log.clone();
    let kek = std::thread::spawn(move || {
        l.log::<String>(0, "Ok", &[]);
    });
    log.set_context(Context::Main, 190);
    log.log::<String>(123, "Hen", &[]);
    let mut newlog = log.with_context(Context::Lggr);
    log.log::<String>(129, "Hend", &[]);
    log.set_context(Context::Lggr, 0);
    newlog.log::<String>(0, "neato", &[]);
    thread.join().unwrap();
}


#[cfg(test)]
mod tests {
    use rand::prelude::*;
    use super::*;
    use test::{black_box, Bencher};

    #[bench]
    fn message_not_sent(b: &mut Bencher) {
        let (mut log, thread) = Logger::new(Context::Main);
        b.iter(|| {
            black_box(log.log::<String>(255, "Not sent", &[]));
        });
        std::mem::drop(log);
        thread.join();
    }

    #[bench]
    fn message_sent(b: &mut Bencher) {
        let (mut log, thread) = Logger::new(Context::Main);
        b.iter(|| {
            black_box(log.log::<String>(128, "Sent", &[]));
        });
        std::mem::drop(log);
        thread.join();
    }

    #[bench]
    fn raw_print(b: &mut Bencher) {
        b.iter(|| {
            black_box(println!["Message"]);
        });
    }
}
