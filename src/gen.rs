use std::{f32::consts::TAU, marker::PhantomData};

use rand::prelude::*;

use crate::source::*;

pub trait Waveform {
    fn one_hz(time: f32) -> f32;
}

#[derive(Debug, Clone, Copy)]
pub struct Wave<W> {
    freq: f32,
    sample_rate: f32,
    time: f32,
    pd: PhantomData<W>,
}

impl<W> Wave<W> {
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        Wave {
            freq,
            sample_rate,
            time: 0.0,
            pd: PhantomData,
        }
    }
}

impl<W> Source for Wave<W>
where
    W: Waveform,
{
    type Frame = Mono;
    fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let res = W::one_hz(self.time);
        self.time += self.freq / self.sample_rate;
        // Some([dbg!(res)])
        Some([res])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sine;
impl Waveform for Sine {
    fn one_hz(time: f32) -> f32 {
        (time * TAU).sin()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Square;
impl Waveform for Square {
    fn one_hz(time: f32) -> f32 {
        if time as u64 % 2 == 0 {
            -1.0
        } else {
            1.0
        }
    }
}

pub type SineWave = Wave<Sine>;
pub type SquareWave = Wave<Square>;

#[derive(Debug, Clone)]
pub struct Noise {
    rng: SmallRng,
    sample_rate: f32,
}

impl Noise {
    pub fn new(sample_rate: f32) -> Self {
        Noise {
            rng: SmallRng::from_entropy(),
            sample_rate,
        }
    }
}

impl Source for Noise {
    type Frame = Mono;
    fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    fn next(&mut self) -> Option<Self::Frame> {
        Some([self.rng.gen_range(-1.0..=1.0) as f32])
    }
}
