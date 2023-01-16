//! Wave generation

use std::f32::consts::TAU;

use rand::prelude::*;

use crate::source::*;

/// Defines a waveform
pub trait Waveform {
    /// The perceptual loudness of this waveform compared to a sine wave
    const LOUDNESS: f32;
    /// Get the amplitude of a 1 Hz wave at the given time
    ///
    /// This should be in the range [-1.0, 1.0]
    fn one_hz(&self, time: f32) -> f32;
}

/// A [`Source`] implementation that outputs a simple wave
#[derive(Debug, Clone, Copy)]
pub struct Wave<W> {
    waveform: W,
    freq: f32,
    sample_rate: f32,
    time: f32,
}

impl<W> Wave<W> {
    /// Create a new wave with the given waveform, frequency, and sample rate
    pub fn with(waveform: W, freq: f32, sample_rate: f32) -> Self {
        Wave {
            waveform,
            freq,
            sample_rate,
            time: 0.0,
        }
    }
}

impl<W> Wave<W>
where
    W: Default,
{
    /// Create a new wave with the given frequency and sample rate
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        Wave {
            waveform: W::default(),
            freq,
            sample_rate,
            time: 0.0,
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
        let res = 1.0 / W::LOUDNESS * self.waveform.one_hz(self.time);
        self.time += self.freq / self.sample_rate;
        Some(res)
    }
}

/// A sine waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Sine;
impl Waveform for Sine {
    const LOUDNESS: f32 = 1.0;
    fn one_hz(&self, time: f32) -> f32 {
        (time * TAU).sin()
    }
}

/// A square waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Square;
impl Waveform for Square {
    const LOUDNESS: f32 = 3.0;
    fn one_hz(&self, time: f32) -> f32 {
        if (time * 2.0) as u64 % 2 == 0 {
            -1.0
        } else {
            1.0
        }
    }
}

/// A saw waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Saw;
impl Waveform for Saw {
    const LOUDNESS: f32 = 3.0;
    fn one_hz(&self, time: f32) -> f32 {
        2.0 * (time - (time + 0.5).floor())
    }
}

/// A triangle waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Triangle;
impl Waveform for Triangle {
    const LOUDNESS: f32 = 1.1;
    fn one_hz(&self, time: f32) -> f32 {
        2.0 * Saw.one_hz(time).abs() - 1.0
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
        Some(self.rng.gen_range(-1.0..=1.0) as f32)
    }
}
