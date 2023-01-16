//! Audio sources

use std::{
    collections::VecDeque,
    marker::PhantomData,
    ops::Add,
    sync::{Arc, Weak},
    time::Duration,
};

use crate::{lerp, Shared, ToDuration};

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

/// An audio source with a dynamic frame size
///
/// This is usually only used for audio sources whose channel count
/// is only known at runtime, like audio input.
///
/// It can be converted to a [`Source`] by with [`UnrolledSource::resample`].
pub trait UnrolledSource: Iterator<Item = f32> {
    /// Get the number of audio channels
    fn channels(&self) -> usize;
    /// Get the sample rate
    fn sample_rate(&self) -> f32;
    /// Resample this source to have a static frame size
    ///
    /// For a frame size of 1, the source samples are averaged.
    /// If there is only one source channel, then that channel's
    /// amplitude is duplicated to all frame channels. In all other
    /// cases, the amplitudes of source channels that excede the
    /// frame's channel count are discarded.
    fn resample<F>(self) -> Resample<Self, F>
    where
        Self: Sized,
    {
        Resample {
            source: self,
            time: 0.0,
            frame: None,
            pd: PhantomData,
        }
    }
}

/// An audio source with a static frame size
pub trait Source {
    /// The [`Frame`] type
    type Frame: Frame;
    /// Get the next frame
    ///
    /// Returning [`None`] indicates the source has no samples left
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame>;
    /// Amplify the source by some multiplier
    fn amplify(self, amp: impl Into<Shared<f32>>) -> Amplify<Self>
    where
        Self: Sized,
    {
        Amplify {
            source: self,
            amp: amp.into(),
        }
    }
    /// Normalize the amplitude of the source
    ///
    /// The source will be amplified based on the average amplitude of
    /// of previous frames
    fn normalize(
        self,
        target_amp: impl Into<Shared<f32>>,
        running_average_dur: impl ToDuration,
    ) -> Normalize<Self>
    where
        Self: Sized,
    {
        Normalize {
            source: self,
            target_amp: target_amp.into(),
            amp_mul: 1.0,
            running_avg_dur: running_average_dur.to_duration().as_secs_f32(),
        }
    }
    /// End the source after some duration
    fn take(self, dur: impl ToDuration) -> Take<Self>
    where
        Self: Sized,
    {
        Take {
            source: self,
            duration: dur.to_duration(),
            elapsed: Duration::ZERO,
            release: Duration::ZERO,
        }
    }
    /// End the source after some duration and apply a release envelope
    fn take_release(self, dur: impl ToDuration, release: impl ToDuration) -> Take<Self>
    where
        Self: Sized,
    {
        Take {
            source: self,
            duration: dur.to_duration(),
            elapsed: Duration::ZERO,
            release: release.to_duration(),
        }
    }
    /// Chain the source with another
    fn chain<B>(self, next: B) -> Chain<Self::Frame>
    where
        Self: Sized + Send + 'static,
        B: Sized + Send + 'static + Source<Frame = Self::Frame>,
    {
        let mut chain = Chain {
            queue: VecDeque::new(),
        };
        chain.queue.push_back(Box::new(self));
        chain.queue.push_back(Box::new(next));
        chain
    }
    /// Apply a low-pass filter with the given cut-off frequency
    fn low_pass(self, freq: impl Into<Shared<f32>>) -> LowPass<Self>
    where
        Self: Sized,
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
            b: other,
            f,
        }
    }
    /// Apply a pan to the source
    ///
    /// Non-mono sources will be averaged before panning
    fn pan(self, pan: impl Into<Shared<f32>>) -> Pan<Self>
    where
        Self: Sized,
    {
        Pan {
            source: self,
            pan: pan.into(),
        }
    }
    /// Apply an attack-decay-sustain envelope to the source
    ///
    /// To apply a release as well, use [`Source::take_release`] or [`Source::maintained`] after this
    fn ads(self, envelope: impl Into<Shared<AdsEnvelope>>) -> Ads<Self>
    where
        Self: Sized,
    {
        Ads {
            source: self,
            curr: Duration::ZERO,
            envelope: envelope.into(),
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
            release_dur: maintainer.release_dur.clone(),
            release_curr: Duration::ZERO,
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

/// A source that returns a constant value
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Constant(pub f32);

impl Source for Constant {
    type Frame = f32;
    fn next(&mut self, _sample_rate: f32) -> Option<Self::Frame> {
        Some(self.0)
    }
}

/// Source returned from [`Source::amplify`]
pub struct Amplify<S> {
    source: S,
    amp: Shared<f32>,
}

impl<S> Source for Amplify<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        self.source
            .next(sample_rate)
            .map(|frame| frame.map(|a| a * self.amp.get()))
    }
}

/// Source returned from [`Source::normalize`]
pub struct Normalize<S> {
    source: S,
    target_amp: Shared<f32>,
    amp_mul: f32,
    running_avg_dur: f32,
}

impl<S> Source for Normalize<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let frame = self.source.next(sample_rate)?;
        let t = 1.0 / (sample_rate * self.running_avg_dur);
        let target = self.target_amp.get();
        let amp = target / self.amp_mul;
        let new_amp = (1.0 - t) * amp + t * frame.avg().abs();
        self.amp_mul = target / new_amp;
        Some(frame.map(|a| a * self.amp_mul))
    }
}

/// Source returned from [`Source::take`]
pub struct Take<S> {
    source: S,
    duration: Duration,
    elapsed: Duration,
    release: Duration,
}

impl<S> Source for Take<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        if self.elapsed >= self.duration {
            return None;
        }
        let frame = self.source.next(sample_rate)?;
        let amp = if self.release.is_zero() {
            1.0
        } else {
            let time_left = (self.duration - self.elapsed).as_secs_f32();
            (time_left / self.release.as_secs_f32()).min(1.0)
        };
        self.elapsed += Duration::from_secs_f32(1.0 / sample_rate);
        Some(frame.map(|a| a * amp))
    }
}

/// Source return from [`Source::chain`]
pub struct Chain<F> {
    queue: VecDeque<DynSource<F>>,
}

impl<F> Source for Chain<F>
where
    F: Frame,
{
    type Frame = F;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let front = self.queue.front_mut()?;
        if let Some(frame) = front.next(sample_rate) {
            Some(frame)
        } else {
            self.queue.pop_front();
            self.next(sample_rate)
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
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        if let Some(frame) = self.source.next(sample_rate) {
            Some(if let Some(acc) = &mut self.acc {
                let t = (self.freq.get() / sample_rate).min(1.0);
                acc.clone().merge(frame, |a, b| lerp(a, b, t))
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
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        self.source.next(sample_rate).map(&self.f)
    }
}

/// Source returned from [`Source::zip`]
pub struct Zip<A, B, F>
where
    A: Source,
    B: Source,
{
    a: A,
    b: B,
    f: F,
}

impl<A, B, F, C> Source for Zip<A, B, F>
where
    A: Source,
    B: Source,
    F: Fn(A::Frame, B::Frame) -> C,
    C: Frame,
{
    type Frame = C;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        Some((self.f)(
            self.a.next(sample_rate)?,
            self.b.next(sample_rate)?,
        ))
    }
}

/// Source returned from [`Source::pan`]
pub struct Pan<S> {
    source: S,
    pan: Shared<f32>,
}

impl<S> Source for Pan<S>
where
    S: Source,
{
    type Frame = Stereo;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        self.source.next(sample_rate).map(|frame| {
            let frame = frame.avg();
            let pan = self.pan.get();
            let left = frame * (1.0 - pan);
            let right = frame * pan;
            [left, right]
        })
    }
}

/// Used to coordinate the dropping of [`Source`]s
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Maintainer {
    arc: Arc<()>,
    release_dur: Shared<Duration>,
}

impl Maintainer {
    /// Create a new maintainer
    pub fn new() -> Self {
        Self::default()
    }
    /// Create a new maintainer with the given release duration
    pub fn with_release(dur: impl Into<Shared<Duration>>) -> Self {
        Maintainer {
            arc: Arc::new(()),
            release_dur: dur.into(),
        }
    }
}

impl Drop for Maintainer {
    fn drop(&mut self) {}
}

/// An attack-decay-sustain evenlope
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsEnvelope {
    /// The time after the sound starts before it is at its maximum volume
    pub attack: Duration,
    /// The time between the maximum amplitude and the sustain amplitude
    pub decay: Duration,
    /// The sustain amplitude
    pub sustain: f32,
}

impl Default for AdsEnvelope {
    fn default() -> Self {
        AdsEnvelope {
            attack: Duration::ZERO,
            decay: Duration::ZERO,
            sustain: 1.0,
        }
    }
}

impl AdsEnvelope {
    /// Create a new ADS envelope
    pub fn new(attack: impl ToDuration, decay: impl ToDuration, sustain: f32) -> Self {
        Self {
            attack: attack.to_duration(),
            decay: decay.to_duration(),
            sustain,
        }
    }
}

/// Source returned from [`Source::ads`]
pub struct Ads<S> {
    source: S,
    curr: Duration,
    envelope: Shared<AdsEnvelope>,
}

impl<S> Source for Ads<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let frame = self.source.next(sample_rate)?;
        let envelope = self.envelope.get();
        let amp = if self.curr < envelope.attack {
            self.curr.as_secs_f32() / envelope.attack.as_secs_f32()
        } else {
            let after_attack = self.curr - envelope.attack;
            if after_attack < envelope.decay {
                (1.0 - after_attack.as_secs_f32() / envelope.decay.as_secs_f32())
                    * (1.0 - envelope.sustain)
                    + envelope.sustain
            } else {
                envelope.sustain
            }
        };
        self.curr += Duration::from_secs_f32(1.0 / sample_rate);
        Some(frame.map(|s| s * amp))
    }
}

/// Source returned from [`Source::maintained`]
pub struct Maintained<S> {
    source: S,
    arc: Weak<()>,
    release_dur: Shared<Duration>,
    release_curr: Duration,
}

impl<S> Source for Maintained<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        if Weak::strong_count(&self.arc) == 0 {
            let release_dur = self.release_dur.get();
            if self.release_curr < release_dur {
                let amp = 1.0 - self.release_curr.as_secs_f32() / release_dur.as_secs_f32();
                let frame = self.source.next(sample_rate)?.map(|s| s * amp);
                self.release_curr += Duration::from_secs_f32(1.0 / sample_rate);
                Some(frame)
            } else {
                None
            }
        } else {
            self.source.next(sample_rate)
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
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let frame = self.source.next(sample_rate);
        self.curr.set(frame.clone());
        frame
    }
}

impl<F> SourceInspector<F>
where
    F: Frame,
{
    /// Read the inspected [`Source`]'s current frame
    pub fn read(&self) -> Option<F> {
        self.curr.cloned()
    }
}

/// Source that resamples a dynamic source to have a fixed frame size
pub struct Resample<S, F> {
    source: S,
    time: f32,
    frame: Option<F>,
    pd: PhantomData<F>,
}

impl<S, F> Resample<S, F>
where
    S: UnrolledSource,
    F: Frame,
{
    fn get_frame(&mut self) -> Option<F> {
        let source_channels = self.source.channels();
        let mut sample = F::uniform(0.0);
        match F::CHANNELS {
            // For empty output just take all the source samples
            0 => {
                if self.source.by_ref().take(source_channels).count() < source_channels {
                    return None;
                }
            }
            // For mono output, use the average of all source samples
            1 => {
                let mut sum = 0.0;
                let mut count = 0;
                for s in self.source.by_ref().take(source_channels) {
                    count += 1;
                    sum += s;
                }
                if count < source_channels {
                    return None;
                }
                sample.set_channel(0, sum / count as f32);
            }
            // For mono input and multi output, fill every output channel with the input one
            n if source_channels == 1 => {
                let amplitude = self.source.next()?;
                for i in 0..n {
                    sample.set_channel(i, amplitude);
                }
            }
            // For multi input and output, discard extra input samples
            n => {
                let mut count = 0;
                for (i, amplitude) in self.source.by_ref().take(source_channels).enumerate() {
                    count += 1;
                    if i < n {
                        sample.set_channel(i, amplitude);
                    }
                }
                if count < source_channels {
                    return None;
                }
            }
        }
        Some(sample)
    }
}

impl<S, F> Source for Resample<S, F>
where
    S: UnrolledSource,
    F: Frame,
{
    type Frame = F;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let target_time = self.time + 1.0 / sample_rate;
        while self.time < target_time {
            self.frame = self.get_frame();
            self.time += 1.0 / self.source.sample_rate();
        }
        self.frame.clone()
    }
}
