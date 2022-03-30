//! Audio sources

use std::{
    collections::VecDeque,
    marker::PhantomData,
    sync::{Arc, Weak},
    time::Duration,
};

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
    /// Chain the source with another
    fn chain<B>(self, next: B) -> Chain<Self::Frame>
    where
        Self: Sized + Send + 'static,
        B: Sized + Send + 'static + Source<Frame = Self::Frame>,
    {
        let initial_sample_rate = self.sample_rate();
        let mut chain = Chain {
            initial_sample_rate,
            queue: VecDeque::new(),
        };
        chain.queue.push_back(Box::new(self));
        chain.queue.push_back(Box::new(next));
        chain
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
    /// Apply an attack envelope to the source
    fn attack(self, dur: Duration) -> Attack<Self>
    where
        Self: Sized,
    {
        Attack {
            source: self,
            attack_curr: Duration::ZERO,
            attack_dur: dur,
        }
    }
    /// Keep playing this source as long as the given [`Maintainer`] is not dropped
    fn maintained(self, maintainer: &Maintainer) -> Maintained<Self>
    where
        Self: Sized,
    {
        Maintained {
            source: self,
            arc: Arc::downgrade(&maintainer.arc),
            decay_dur: maintainer.decay_dur,
            decay_curr: Duration::ZERO,
        }
    }
    /// Allow the current frame of the source to be inspected
    fn inspect(self) -> (SourceInspector<Self::Frame>, InspectedSource<Self>)
    where
        Self: Sized,
    {
        let curr = Shared::new(None);
        (
            SourceInspector { curr: curr.clone() },
            InspectedSource { source: self, curr },
        )
    }
}

/// A dynamic source type
pub type DynSource<F> = Box<dyn Source<Frame = F> + Send>;

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

impl<F> Silence<F> {
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
        self.elapsed += Duration::from_secs_f32(1.0 / self.source.sample_rate());
        Some(frame)
    }
}

/// Source return from [`Source::chain`]
pub struct Chain<F> {
    initial_sample_rate: f32,
    queue: VecDeque<DynSource<F>>,
}

impl<F> Source for Chain<F>
where
    F: Frame,
{
    type Frame = F;
    fn sample_rate(&self) -> f32 {
        self.queue
            .front()
            .map(|source| source.sample_rate())
            .unwrap_or(self.initial_sample_rate)
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let front = self.queue.front_mut()?;
        if let Some(frame) = front.next() {
            Some(frame)
        } else {
            self.queue.pop_front();
            self.next()
        }
    }
    fn chain<B>(mut self, next: B) -> Chain<Self::Frame>
    where
        Self: Sized + Send + 'static,
        B: Sized + Send + 'static + Source<Frame = Self::Frame>,
    {
        self.queue.push_back(Box::new(next));
        self
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
pub struct Maintainer {
    arc: Arc<()>,
    decay_dur: Duration,
}

impl Maintainer {
    /// Create a new maintainer
    pub fn new() -> Self {
        Self::default()
    }
    /// Create a new maintainer with the given decay duration
    pub fn with_decay(dur: Duration) -> Self {
        Maintainer {
            arc: Arc::new(()),
            decay_dur: dur,
        }
    }
}

impl Drop for Maintainer {
    fn drop(&mut self) {}
}

/// Source returned from [`Source::attack`]
pub struct Attack<S> {
    source: S,
    attack_curr: Duration,
    attack_dur: Duration,
}

impl<S> Source for Attack<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let frame = self.source.next()?;
        Some(if self.attack_curr < self.attack_dur {
            let amp = self.attack_curr.as_secs_f32() / self.attack_dur.as_secs_f32();
            self.attack_curr += Duration::from_secs_f32(1.0 / self.source.sample_rate());
            frame.map(|s| s * amp)
        } else {
            frame
        })
    }
}

/// Source returned from [`Source::maintained`]
pub struct Maintained<S> {
    source: S,
    arc: Weak<()>,
    decay_dur: Duration,
    decay_curr: Duration,
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
        if Weak::strong_count(&self.arc) == 0 {
            if self.decay_curr < self.decay_dur {
                let amp = 1.0 - self.decay_curr.as_secs_f32() / self.decay_dur.as_secs_f32();
                let frame = self.source.next()?.map(|s| s * amp);
                self.decay_curr += Duration::from_secs_f32(1.0 / self.source.sample_rate());
                Some(frame)
            } else {
                None
            }
        } else {
            self.source.next()
        }
    }
}

/// A source that is being inspected by a [`SourceInspector`]
pub struct InspectedSource<S: Source> {
    source: S,
    curr: Shared<Option<S::Frame>>,
}

/// Allows the inspection of a [`Source`]'s current frame
pub struct SourceInspector<F> {
    curr: Shared<Option<F>>,
}

impl<S: Source> Source for InspectedSource<S> {
    type Frame = S::Frame;
    fn sample_rate(&self) -> f32 {
        self.source.sample_rate()
    }
    fn next(&mut self) -> Option<Self::Frame> {
        let frame = self.source.next();
        self.curr.set(frame.clone());
        frame
    }
}

impl<F> SourceInspector<F>
where
    F: Frame,
{
    /// Read the inspected [`Source`]'s current frame
    pub fn read(&mut self) -> Option<F> {
        self.curr.cloned()
    }
}
