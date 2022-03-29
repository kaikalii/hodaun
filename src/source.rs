//! Audio sources

use std::{marker::PhantomData, sync::Arc, time::Duration};

use crate::{lerp, Shared};

/// Mono [`Frame`] type
pub type Mono = [f32; 1];
/// Stereo [`Frame`] type
pub type Stereo = [f32; 2];

/// Convert a [`Frame`] to mono
pub fn mono<F>(frame: F) -> Mono
where
    F: AsRef<[f32]>,
{
    [frame.as_ref().iter().sum::<f32>() / frame.as_ref().len() as f32]
}

/// Convert a mono frame to stereo
pub fn stereo(frame: Mono) -> Stereo {
    [frame[0]; 2]
}

/// A single multi-channel frame in an audio stream
pub trait Frame: Default + Clone {
    /// Get the number of audio channels
    fn channels(&self) -> usize;
    /// Get the amplitude of a channel
    fn get_channel(&self, index: usize) -> f32;
    /// Apply a function to each channels
    fn map<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32;
    /// Combine two frames by applying a function
    fn merge<F>(&mut self, other: Self, f: F)
    where
        F: Fn(f32, f32) -> f32;
}

impl<T> Frame for T
where
    T: Default + Clone + AsRef<[f32]> + AsMut<[f32]> + Send + 'static,
{
    fn channels(&self) -> usize {
        self.as_ref().len()
    }
    fn get_channel(&self, index: usize) -> f32 {
        self.as_ref()[index % self.channels()]
    }
    fn map<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        for a in self.as_mut() {
            *a = f(*a);
        }
        self
    }
    fn merge<F>(&mut self, other: Self, f: F)
    where
        F: Fn(f32, f32) -> f32,
    {
        for (a, b) in self.as_mut().iter_mut().zip(other.as_ref()) {
            *a = f(*a, *b);
        }
    }
}

/// An audio source
pub trait Source {
    /// The [`Frame`] type
    type Frame: Frame;
    /// Get the sample rate
    fn sample_rate(&self) -> f32;
    /// Get the next frame
    ///
    /// Returning [`None`] indicates the source has no samples left
    fn next(&mut self) -> Option<Self::Frame>;
    /// Amplify the source by some multiplier
    fn amplify(self, amp: f32) -> Amplify<Self>
    where
        Self: Sized,
    {
        Amplify { source: self, amp }
    }
    /// End the source after some duration
    fn take(self, dur: Duration) -> Take<Self>
    where
        Self: Sized,
    {
        Take {
            source: self,
            duration: dur,
            elapsed: Duration::from_secs(0),
        }
    }
    /// Apply a low-pass filter with the given cut-off frequency
    fn low_pass<F>(self, freq: F) -> LowPass<Self>
    where
        Self: Sized,
        F: Into<Shared<f32>>,
    {
        LowPass {
            source: self,
            freq: freq.into(),
            acc: None,
        }
    }
    /// Transform each frame with the given function
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        Map { source: self, f }
    }
    /// Combine this source with another using the given frame-combining function
    fn zip<F, B>(self, other: B, f: F) -> Zip<Self, B, F>
    where
        Self: Sized,
        B: Source,
    {
        Zip {
            a: self,
            curr_a: None,
            b: other,
            curr_b: None,
            f,
            t: 0.0,
        }
    }
    /// Keep playing this source as long as the given [`Maintainer`] is not dropped
    fn maintained(self, maintainer: &Maintainer) -> Maintained<Self>
    where
        Self: Sized,
    {
        Maintained {
            source: self,
            arc: Arc::clone(&maintainer.0),
        }
    }
}

/// Source returned from [`Source::amplify`]
pub struct Amplify<S> {
    source: S,
    amp: f32,
}

impl<S> Source for Amplify<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        self.source.next().map(|frame| frame.map(|a| a * self.amp))
    }
}

/// Source that plays nothing forever
#[derive(Debug, Clone, Copy)]
pub struct Silence<F = [f32; 1]> {
    sample_rate: f32,
    pd: PhantomData<F>,
}

impl Silence {
    /// Create new silence
    pub fn new(sample_rate: f32) -> Self {
        Silence {
            sample_rate,
            pd: PhantomData,
        }
    }
}

impl<F> Source for Silence<F>
where
    F: Frame,
{
    type Frame = F;
    fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    fn next(&mut self) -> Option<Self::Frame> {
        Some(F::default())
    }
}

/// Source returned from [`Source::take`]
pub struct Take<S> {
    source: S,
    duration: Duration,
    elapsed: Duration,
}

impl<S> Source for Take<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        if self.elapsed >= self.duration {
            return None;
        }
        let frame = self.source.next()?;
        self.duration += Duration::from_secs_f32(1.0 / self.source.sample_rate());
        Some(frame)
    }
}

/// Source returned from [`Source::low_pass`]
pub struct LowPass<S>
where
    S: Source,
{
    source: S,
    acc: Option<S::Frame>,
    freq: Shared<f32>,
}

impl<S> Source for LowPass<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        if let Some(frame) = self.source.next() {
            Some(if let Some(acc) = &mut self.acc {
                let t = (self.freq.get() / self.source.sample_rate()).min(1.0);
                acc.merge(frame, |a, b| lerp(a, b, t));
                acc.clone()
            } else {
                self.acc = Some(frame.clone());
                frame
            })
        } else {
            None
        }
    }
}

/// Source returned from [`Source::map`]
pub struct Map<S, F> {
    source: S,
    f: F,
}

impl<S, F, B> Source for Map<S, F>
where
    S: Source,
    F: Fn(S::Frame) -> B,
    B: Frame,
{
    type Frame = B;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        self.source.next().map(&self.f)
    }
}

/// Source returned from [`Source::zip`]
pub struct Zip<A, B, F>
where
    A: Source,
    B: Source,
{
    a: A,
    curr_a: Option<A::Frame>,
    b: B,
    curr_b: Option<B::Frame>,
    f: F,
    t: f32,
}

impl<A, B, F, C> Source for Zip<A, B, F>
where
    A: Source,
    B: Source,
    F: Fn(A::Frame, B::Frame) -> C,
    C: Frame,
{
    type Frame = C;
    fn sample_rate(&self) -> f32 {
        self.a.sample_rate().max(self.b.sample_rate())
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let a = if self.t >= 0.0 {
            let frame = self.a.next()?;
            self.curr_a = Some(frame.clone());
            self.t -= 1.0 / self.a.sample_rate();
            frame
        } else {
            self.curr_a.clone()?
        };
        let b = if self.t < 0.0 {
            let frame = self.b.next()?;
            self.curr_b = Some(frame.clone());
            self.t += 1.0 / self.b.sample_rate();
            frame
        } else {
            self.curr_b.clone()?
        };
        Some((self.f)(a, b))
    }
}

/// Used to coordinate the dropping of [`Source`]s
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Maintainer(Arc<()>);

impl Maintainer {
    /// Create a new maintainer
    pub fn new() -> Self {
        Self::default()
    }
}

/// Source returned from [`Source::maintained`]
pub struct Maintained<S> {
    source: S,
    arc: Arc<()>,
}

impl<S> Source for Maintained<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        if Arc::strong_count(&self.arc) == 1 {
            None
        } else {
            self.source.next()
        }
    }
}
