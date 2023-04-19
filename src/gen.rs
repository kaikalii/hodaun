//! Wave generation

use std::{
    f64::consts::TAU,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "rand")]
use rand::prelude::*;

use crate::{lerp, source::*, Automation, Mono};

/// Defines a waveform
pub trait Waveform {
    /// The perceptual loudness of this waveform compared to a sine wave
    const LOUDNESS: f64;
    /// Get the amplitude of a 1 Hz wave at the given time
    ///
    /// This should be in the range [-1.0, 1.0]
    fn one_hz(&self, time: f64) -> f64;
}

/// A [`Source`] implementation that outputs a simple wave
#[derive(Debug, Clone, Copy)]
pub struct Wave<W, F = f64> {
    waveform: W,
    freq: F,
    time: f64,
}

impl<W, F> Wave<W, F> {
    /// Create a new wave with the given waveform and frequency
    pub fn with(waveform: W, freq: F) -> Self {
        Wave {
            waveform,
            freq,
            time: 0.0,
        }
    }
}

impl<W, F> Wave<W, F>
where
    W: Default,
{
    /// Create a new wave with the given frequency
    pub fn new(freq: F) -> Self {
        Wave {
            waveform: W::default(),
            freq,
            time: 0.0,
        }
    }
}

impl<W, F> Source for Wave<W, F>
where
    W: Waveform,
    F: Automation,
{
    type Frame = Mono;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let res = 1.0 / W::LOUDNESS * self.waveform.one_hz(self.time);
        let freq = self.freq.next_value(sample_rate)?;
        self.time = (self.time + freq / sample_rate) % (1e6 * sample_rate / freq);
        Some(res)
    }
}

/// A sine waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Sine;
impl Waveform for Sine {
    const LOUDNESS: f64 = 1.0;
    fn one_hz(&self, time: f64) -> f64 {
        (time * TAU).sin()
    }
}

/// A square waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Square;
impl Waveform for Square {
    const LOUDNESS: f64 = 3.0;
    fn one_hz(&self, time: f64) -> f64 {
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
    const LOUDNESS: f64 = 3.0;
    fn one_hz(&self, time: f64) -> f64 {
        2.0 * (time - (time + 0.5).floor())
    }
}

/// A triangle waveform
#[derive(Debug, Clone, Copy, Default)]
pub struct Triangle;
impl Waveform for Triangle {
    const LOUDNESS: f64 = 1.1;
    fn one_hz(&self, time: f64) -> f64 {
        2f64.mul_add(Saw.one_hz(time).abs(), -1.0)
    }
}

/// A sine wave source
pub type SineWave<F = f64> = Wave<Sine, F>;
/// A square wave source
pub type SquareWave<F = f64> = Wave<Square, F>;
/// A saw wave source
pub type SawWave<F = f64> = Wave<Saw, F>;
/// A triangle wave source
pub type TriangleWave<F = f64> = Wave<Triangle, F>;

/// Simple random noise source
#[cfg(feature = "noise")]
#[derive(Debug, Clone)]
pub struct Noise {
    rng: SmallRng,
}

impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "noise")]
impl Noise {
    /// Create new noise with the given sample rate
    pub fn new() -> Self {
        Noise {
            rng: SmallRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
            ),
        }
    }
}

#[cfg(feature = "noise")]
impl Source for Noise {
    type Frame = Mono;
    fn next(&mut self, _sample_rate: f64) -> Option<Self::Frame> {
        Some(self.rng.gen_range(-1.0..=1.0))
    }
}

#[derive(Clone, Copy)]
/// A linear interpolation source
pub struct Lerp<A, B, D> {
    start: A,
    end: B,
    duration: D,
    time: f64,
}

impl<A, B, D> Lerp<A, B, D> {
    /// Create a new linear interpolation from `start` to `end` over `duration`
    pub fn new(start: A, end: B, duration: D) -> Self {
        Lerp {
            start,
            end,
            duration,
            time: 0.0,
        }
    }
}

impl<A, B, D> Source for Lerp<A, B, D>
where
    A: Automation,
    B: Automation,
    D: Automation,
{
    type Frame = Mono;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let duration = self.duration.next_value(sample_rate)?;
        if self.time >= duration {
            return None;
        }
        let t = self.time / duration;
        let a = self.start.next_value(sample_rate)?;
        let b = self.end.next_value(sample_rate)?;
        let res = lerp(a, b, t);
        self.time += 1.0 / sample_rate;
        Some(res)
    }
}
