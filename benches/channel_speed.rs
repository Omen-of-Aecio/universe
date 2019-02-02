#![feature(test)]
extern crate test; // Required for testing, even though extern crate is no longer needed in the 2018 version, this is a special case

use test::{black_box, Bencher};
use std::{
    collections::HashMap,
    fmt::Display,
    marker::Send,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, RecvError, TrySendError},
        Arc, Mutex,
    },
    thread,
};

#[bench]
fn channel_sending_enum(b: &mut Bencher) {
    enum Test {
        A, B, C
    }
    let (tx, rx) = mpsc::sync_channel(1);
    b.iter(|| {
        tx.send(Test::A).unwrap();
        rx.recv().unwrap();
    });
}

#[bench]
fn channel_sending_fn(b: &mut Bencher) {
    let (tx, rx) = mpsc::sync_channel(1);
    b.iter(|| {
        tx.send(|| {}).unwrap();
        rx.recv().unwrap();
    });
}
