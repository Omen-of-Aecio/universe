//! Simple, stupid benchmarker
//!
//! Allows simple sums of repeated benchmarks. Does no statistical processing,
//! just adds N benchmarks together before returning the sum from `stop`.
//!
//! ```
//! use benchmarker::Benchmarker;
//!
//! fn main() {
//!     // Setup, with 0 buffered benchmarks
//!     let mut bench = Benchmarker::new(0);
//!
//!     // Start the benchmark
//!     bench.start();
//!
//!     // --- Do some stuff
//!
//!     // Stop the benchmark, if this is the end of summing, this call
//!     // will return a Some(Duration), otherwise None.
//!     assert![bench.stop().is_some()];
//! }
//! ```
//!
//! We may also be interested in multiple samples just because.
//!
//! ```
//! use benchmarker::Benchmarker;
//!
//! fn main() {
//!     // Setup, with 99 buffered benchmarks
//!     let mut bench = Benchmarker::new(99);
//!
//!     for _ in 0..99 {
//!         // Start the benchmark
//!         bench.start();
//!
//!         // --- Do some stuff
//!
//!         // Stop the benchmark, if this is the end of summing, this call
//!         // will return a Some(Duration), otherwise None.
//!         assert![bench.stop().is_none()];
//!     }
//!
//!     // Do the final benchmark
//!     bench.start();
//!     assert![bench.stop().is_some()];
//! }
//! ```
//!
//! Note that with 99 buffers, the final call to `stop` will return a duration
//! sum representing 99+1 samples. This is to allow the 0-case, where we have
//! 0+1 samples.
#![feature(test)]
use time::{Duration, PreciseTime};

extern crate test;

pub struct Benchmarker {
    last: PreciseTime,
    count: usize,
    window: usize,
    sum: time::Duration,
}

impl Benchmarker {
    pub fn new(window: usize) -> Benchmarker {
        Benchmarker {
            last: PreciseTime::now(),
            count: 0,
            window,
            sum: Duration::zero(),
        }
    }

    pub fn start(&mut self) {
        self.last = PreciseTime::now();
    }

    pub fn stop(&mut self) -> Option<Duration> {
        let now = PreciseTime::now();
        self.sum = self.sum + self.last.to(now);
        if self.count < self.window {
            self.count += 1;
            None
        } else {
            self.count = 0;
            let ret = Some(self.sum);
            self.sum = Duration::zero();
            ret
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn zero_length() {
        let mut ben = Benchmarker::new(0);
        ben.start();
        assert![ben.stop().is_some()];
    }

    #[test]
    fn basic_length() {
        let mut ben = Benchmarker::new(1);
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_some()];
    }

    #[test]
    fn ten_length() {
        let mut ben = Benchmarker::new(5);
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_none()];
        ben.start();
        assert![ben.stop().is_some()];
    }

    #[bench]
    fn zero_usage(b: &mut Bencher) {
        let mut ben = Benchmarker::new(0);
        b.iter(|| {
            ben.start();
            black_box(ben.stop());
        });
    }

    #[bench]
    fn casual_usage(b: &mut Bencher) {
        let mut ben = Benchmarker::new(100);
        b.iter(|| {
            ben.start();
            black_box(ben.stop());
        });
    }

    #[bench]
    fn large_usage(b: &mut Bencher) {
        let mut ben = Benchmarker::new(1_000_000_000);
        b.iter(|| {
            ben.start();
            black_box(ben.stop());
        });
    }
}
