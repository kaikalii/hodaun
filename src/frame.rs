use std::ops::Add;

/// Mono [`Frame`] type
pub type Mono = f32;
/// Stereo [`Frame`] type
pub type Stereo = [f32; 2];

/// Convert a [`Frame`] to mono
pub fn mono(frame: impl AsRef<[f32]>) -> Mono {
    frame.as_ref().iter().sum::<f32>() / frame.as_ref().len() as f32
}

/// Convert a mono frame to stereo
pub fn stereo(frame: Mono) -> Stereo {
    [frame; 2]
}

/// A single multi-channel frame in an audio source
pub trait Frame: Clone {
    /// The number of audio channels
    const CHANNELS: usize;
    /// Create a frame with a uniform amplitude across all channels
    fn uniform(amplitude: f32) -> Self;
    /// Get the amplitude of a channel
    fn get_channel(&self, index: usize) -> f32;
    /// Set the amplitude of a channel
    fn set_channel(&mut self, index: usize, amplitude: f32);
    /// Apply a function to each channels
    fn map(self, f: impl Fn(f32) -> f32) -> Self;
    /// Combine two frames by applying a function
    fn merge(self, other: Self, f: impl Fn(f32, f32) -> f32) -> Self;
    /// Get the average amplitude
    fn avg(&self) -> f32 {
        let channels = Self::CHANNELS;
        (0..Self::CHANNELS)
            .map(|i| self.get_channel(i))
            .sum::<f32>()
            / channels as f32
    }
    /// Add two frames
    fn add(self, other: Self) -> Self {
        self.merge(other, Add::add)
    }
}

impl Frame for f32 {
    const CHANNELS: usize = 1;
    fn uniform(amplitude: f32) -> Self {
        amplitude
    }
    fn get_channel(&self, _index: usize) -> f32 {
        *self
    }
    fn set_channel(&mut self, _index: usize, amplitude: f32) {
        *self = amplitude;
    }
    fn map(self, f: impl Fn(f32) -> f32) -> Self {
        f(self)
    }
    fn merge(self, other: Self, f: impl Fn(f32, f32) -> f32) -> Self {
        f(self, other)
    }
    fn avg(&self) -> f32 {
        *self
    }
    fn add(self, other: Self) -> Self {
        self + other
    }
}

impl<const N: usize> Frame for [f32; N]
where
    Self: Default,
{
    const CHANNELS: usize = N;
    fn uniform(amplitude: f32) -> Self {
        [amplitude; N]
    }
    fn get_channel(&self, index: usize) -> f32 {
        self[index]
    }
    fn set_channel(&mut self, index: usize, amplitude: f32) {
        self[index] = amplitude;
    }
    fn map(self, f: impl Fn(f32) -> f32) -> Self {
        self.map(f)
    }
    fn merge(mut self, other: Self, f: impl Fn(f32, f32) -> f32) -> Self {
        for (a, b) in self.iter_mut().zip(other) {
            *a = f(*a, b);
        }
        self
    }
}
