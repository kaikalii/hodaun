use std::f32::consts::TAU;

use crate::source::*;

pub struct Sine {
    freq: f32,
    sample_rate: f32,
    time: f32,
}

impl Sine {
    pub fn new(freq: f32, sample_rate: f32) -> Sine {
        Sine {
            freq,
            sample_rate,
            time: 0.0,
        }
    }
}

impl Source for Sine {
    type Frame = Mono;
    fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let res = self.time.sin();
        self.time += self.freq * TAU / self.sample_rate;
        // Some([dbg!(res)])
        Some([res])
    }
}
