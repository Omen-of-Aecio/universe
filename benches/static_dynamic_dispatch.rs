//! Compare static and dynamic dispatch on an object
#![feature(test)]
extern crate test;

use test::{black_box, Bencher};

trait T {
    fn call(&self) -> Option<()>;
}

struct C {}
impl T for C {
    fn call(&self) -> Option<()> {
        Some(())
    }
}

fn static_call(obj: &C) -> Option<()> {
    obj.call()
}

fn dynamic_call(obj: &dyn T) -> Option<()> {
    obj.call()
}

#[bench]
fn static_dispatch(b: &mut Bencher) {
    let c = C {};
    b.iter(|| {
        for _ in 0..100_000 {
            black_box(static_call(black_box(&c))).unwrap();
        }
    });
}

#[bench]
fn dynamic_dispatch(b: &mut Bencher) {
    let c = C {};
    b.iter(|| {
        for _ in 0..100_000 {
            black_box(dynamic_call(black_box(&c))).unwrap();
        }
    });
}
