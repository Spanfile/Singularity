// this is a rough reimplementation of the Estimate struct from indicatif
// https://github.com/console-rs/indicatif/blob/main/src/state.rs
// indicatif is Copyright (c) 2017 Armin Ronacher <armin.ronacher@active-4.com>

use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Estimate<const N: usize> {
    buf: [f64; N],
    last_index: usize,
    length: usize,
    start_time: Instant,
}

impl<const N: usize> Default for Estimate<N> {
    fn default() -> Self {
        Self {
            buf: [0.0; N],
            last_index: 0,
            length: 0,
            start_time: Instant::now(),
        }
    }
}

impl<const N: usize> Estimate<N> {
    pub fn step(&mut self, value: u64) {
        let divisor = value as f64;
        let item = if divisor == 0.0 {
            0.0
        } else {
            self.elapsed().as_secs_f64() / divisor
        };

        self.push(item);
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn steps_per_second(&self) -> f64 {
        let per_sec = 1.0 / (self.buf[0..self.length].iter().sum::<f64>() / self.length as f64);
        if per_sec.is_nan() { 0.0 } else { per_sec }
    }

    fn push(&mut self, value: f64) {
        if self.length < N {
            self.length += 1;
            self.buf[self.last_index] = value;
        } else {
            self.buf[(self.last_index % self.length)] = value;
        }

        self.last_index = (self.last_index + 1) % self.length;
    }
}
