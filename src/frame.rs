use std::ops::Add;

/// Mono [`Frame`] type
pub type Mono = f64;
/// Stereo [`Frame`] type
pub type Stereo = [f64; 2];

/// Convert a [`Frame`] to mono
pub fn mono(frame: impl AsRef<[f64]>) -> Mono {
    frame.as_ref().iter().sum::<f64>() / frame.as_ref().len() as f64
}

/// Convert a mono [`Frame`] to stereo
pub fn stereo(frame: Mono) -> Stereo {
    [frame; 2]
}

/// A single multi-channel frame in an audio source
pub trait Frame: Clone {
    /// The number of audio channels
    const CHANNELS: usize;
    /// Create a frame with a uniform amplitude across all channels
    fn uniform(amplitude: f64) -> Self;
    /// Get the amplitude of a channel
    fn get_channel(&self, index: usize) -> f64;
    /// Set the amplitude of a channel
    fn set_channel(&mut self, index: usize, amplitude: f64);
    /// Apply a function to each channels
    fn map(self, f: impl Fn(f64) -> f64) -> Self;
    /// Combine two frames by applying a function
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64);
    /// Get the average amplitude
    fn avg(&self) -> f64 {
        let channels = Self::CHANNELS;
        (0..Self::CHANNELS)
            .map(|i| self.get_channel(i))
            .sum::<f64>()
            / channels as f64
    }
    /// Add two frames
    fn add(mut self, other: Self) -> Self {
        self.merge(other, Add::add);
        self
    }
    /// Write the frame to a channel slice
    ///
    /// The channel counts of the frame and slice need not match.
    fn write_slice(self, slice: &mut [f64]) {
        match (Self::CHANNELS, slice.len()) {
            (1, _) => slice.fill(self.get_channel(0)),
            (_, 1) => slice[0] = self.avg(),
            #[allow(clippy::needless_range_loop)]
            (a, b) => {
                for i in 0..a.min(b) {
                    slice[i] = self.get_channel(i);
                }
            }
        }
    }
}

impl Frame for f64 {
    const CHANNELS: usize = 1;
    fn uniform(amplitude: f64) -> Self {
        amplitude
    }
    fn get_channel(&self, _index: usize) -> f64 {
        *self
    }
    fn set_channel(&mut self, _index: usize, amplitude: f64) {
        *self = amplitude;
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        f(self)
    }
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64) {
        *self = f(*self, other);
    }
    fn avg(&self) -> f64 {
        *self
    }
    fn add(self, other: Self) -> Self {
        self + other
    }
}

impl<const N: usize> Frame for [f64; N]
where
    Self: Default,
{
    const CHANNELS: usize = N;
    fn uniform(amplitude: f64) -> Self {
        [amplitude; N]
    }
    fn get_channel(&self, index: usize) -> f64 {
        self[index]
    }
    fn set_channel(&mut self, index: usize, amplitude: f64) {
        self[index] = amplitude;
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        self.map(f)
    }
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64) {
        for (a, b) in self.iter_mut().zip(other) {
            *a = f(*a, b);
        }
    }
}
