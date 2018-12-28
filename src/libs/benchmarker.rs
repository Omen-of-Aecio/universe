use time::{Duration, PreciseTime};

pub struct Benchmarker {
    last: PreciseTime,
    window: usize,
    storage: Vec<time::Duration>,
}

impl Benchmarker {
    pub fn new(window: usize) -> Benchmarker {
        Benchmarker {
            last: PreciseTime::now(),
            window,
            storage: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.last = PreciseTime::now();
    }

    pub fn stop(&mut self) -> Option<Duration> {
        let now = PreciseTime::now();
        self.storage.push(self.last.to(now));
        if self.storage.len() >= self.window {
            let mut sum = Duration::zero();
            for time in &self.storage {
                sum = sum + *time;
            }
            self.storage = Vec::new();
            Some(sum)
        } else {
            None
        }
    }
}
