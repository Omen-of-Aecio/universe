use std::time::{Duration, Instant};

/// A timer that runs after a given interval
///
/// This timer will run at max once per update, meaning that
/// if `N*interval` has elapsed, it will only call the callback once.
pub struct WeakTimer<T> {
    callback: fn(&mut T, Instant),
    interval: Duration,
    prev: Instant,
}

impl<T> WeakTimer<T> {
    /// Create a new [WeakTimer]
    pub fn new(cb: fn(&mut T, Instant), interval: Duration, start_time: Instant) -> Self {
        Self {
            callback: cb,
            interval,
            prev: start_time,
        }
    }

    /// Check if the time instant has passed the interval, then run the callback on the argument
    ///
    /// The [WeakTimer] never runs the callback more than once per update.
    pub fn update(&mut self, now: Instant, arg: &mut T) {
        if now - self.prev >= self.interval {
            (self.callback)(arg, now);
            self.prev = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weaktimer() {
        let now = Instant::now();
        let mut timer = WeakTimer::new(|x, _| { *x += 1; }, Duration::new(1, 0), now);
        let mut arg = 0i32;

        // when
        timer.update(now, &mut arg);

        // then
        assert_eq![0, arg];

        // when
        timer.update(now + Duration::new(1, 0), &mut arg);

        // then
        assert_eq![1, arg];

        // when
        timer.update(now + Duration::new(10, 0), &mut arg);

        // then
        assert_eq![2, arg];

        // when
        timer.update(now + Duration::new(10, 999_999_999), &mut arg);

        // then
        assert_eq![2, arg];

        // when
        timer.update(now + Duration::new(11, 0), &mut arg);

        // then
        assert_eq![3, arg];
    }

    #[test]
    fn summing_weaktime() {
        let now = Instant::now();
        let mut timer = WeakTimer::new(|x, i| { x.0 += i - x.1; }, Duration::new(1, 0), now);
        let mut arg = (Duration::new(0, 0), now);

        // when
        timer.update(now, &mut arg);

        // then
        assert_eq![0, arg];

        // when
        timer.update(now + Duration::new(1, 0), &mut arg);

        // then
        assert_eq![1, arg];

        // when
        timer.update(now + Duration::new(10, 0), &mut arg);

        // then
        assert_eq![2, arg];

        // when
        timer.update(now + Duration::new(10, 999_999_999), &mut arg);

        // then
        assert_eq![2, arg];

        // when
        timer.update(now + Duration::new(11, 0), &mut arg);

        // then
        assert_eq![3, arg];
    }
}
