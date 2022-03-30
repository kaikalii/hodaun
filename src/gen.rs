//! Wave generation

use std::{
    f32::consts::{PI, TAU},
    marker::PhantomData,
};

use rand::prelude::*;

use crate::source::*;

/// Defines a waveform
pub trait Waveform {
    /// Get the amplitude of a 1 Hz wave at the given time
    fn one_hz(time: f32) -> f32;
}

/// A [`Source`] implementation that outputs a simple wave
#[derive(Debug, Clone, Copy)]
pub struct Wave<W> {
    freq: f32,
    sample_rate: f32,
    time: f32,
    pd: PhantomData<W>,
}

impl<W> Wave<W> {
    /// Create a new wave with the given frequency and sample rate
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

/// A sine waveform
#[derive(Debug, Clone, Copy)]
pub struct Sine;
impl Waveform for Sine {
    fn one_hz(time: f32) -> f32 {
        (time * TAU).sin()
    }
}

/// A square waveform
#[derive(Debug, Clone, Copy)]
pub struct Square;
impl Waveform for Square {
    fn one_hz(time: f32) -> f32 {
        const SINE_ENERGY: f32 = 1.0 / PI;
        if (time * 2.0) as u64 % 2 == 0 {
            -SINE_ENERGY
        } else {
            SINE_ENERGY
        }
    }
}

/// A saw waveform
#[derive(Debug, Clone, Copy)]
pub struct Saw;
impl Waveform for Saw {
    fn one_hz(time: f32) -> f32 {
        time - (time + 0.5).floor()
    }
}

/// A triangle waveform
#[derive(Debug, Clone, Copy)]
pub struct Triangle;
impl Waveform for Triangle {
    fn one_hz(time: f32) -> f32 {
        4.0 * (time - (time + 0.5).floor()).abs() - 1.0
    }
}

/// A sine wave source
pub type SineWave = Wave<Sine>;
/// A square wave source
pub type SquareWave = Wave<Square>;
/// A saw wave source
pub type SawWave = Wave<Saw>;
/// A triangle wave source
pub type TriangleWave = Wave<Triangle>;

/// Simple random noise source
#[derive(Debug, Clone)]
pub struct Noise {
    rng: SmallRng,
    sample_rate: f32,
}

impl Noise {
    /// Create new noise with the given sample rate
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
