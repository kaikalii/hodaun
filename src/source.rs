//! Audio sources

use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use parking_lot::Mutex;

use crate::{lerp, Automation, Frame, Shared, Stereo, ToDuration};

/// An audio source with a dynamic frame size
///
/// This is usually only used for audio sources whose channel count
/// is only known at runtime, like audio input.
///
/// It can be converted to a [`Source`] with [`UnrolledSource::resample`].
pub trait UnrolledSource: Iterator<Item = f64> {
    /// Get the number of audio channels
    fn channels(&self) -> usize;
    /// Get the sample rate
    fn sample_rate(&self) -> f64;
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
        F: Frame,
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
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame>;
    /// Amplify the source by some multiplier
    fn amplify<A>(self, amp: A) -> Amplify<Self, A>
    where
        Self: Sized,
        A: Automation,
    {
        Amplify { source: self, amp }
    }
    /// End the source after some duration
    fn take(self, dur: impl ToDuration) -> Take<Self, f64>
    where
        Self: Sized,
    {
        Take {
            source: self,
            duration: dur.to_duration().as_secs_f64(),
            elapsed: 0.0,
            release: 0.0,
        }
    }
    /// End the source after some duration and apply a release envelope
    fn take_release<R>(self, dur: impl ToDuration, release: R) -> Take<Self, R>
    where
        Self: Sized,
        R: Automation,
    {
        Take {
            source: self,
            duration: dur.to_duration().as_secs_f64(),
            elapsed: 0.0,
            release,
        }
    }
    /// Chain the source with another
    fn chain<B>(self, next: B) -> Chain<Self, B>
    where
        Self: Sized,
        B: Source<Frame = Self::Frame>,
    {
        Chain {
            a: self,
            b: next,
            b_start: None,
            time: 0.0,
        }
    }
    /// Apply a low-pass filter with the given cut-off frequency
    fn low_pass<F>(self, freq: F) -> LowPass<Self, F>
    where
        Self: Sized,
        F: Automation,
    {
        LowPass {
            source: self,
            freq,
            acc: None,
        }
    }
    /// Transform each frame with the given function
    fn map<F, B>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Frame) -> B,
    {
        Map { source: self, f }
    }
    /// Combine this source with another using the given frame-combining function
    fn zip<F, B>(self, other: B, f: F) -> Zip<Self, B, F>
    where
        Self: Sized,
        B: Source,
        F: Fn(Self::Frame, B::Frame) -> Self::Frame,
    {
        Zip {
            a: self,
            b: other,
            f,
        }
    }
    /// Combine this source with another by adding their frames
    #[allow(clippy::type_complexity)]
    fn mix<B>(self, other: B) -> Zip<Self, B, fn(Self::Frame, Self::Frame) -> Self::Frame>
    where
        Self: Sized,
        B: Source<Frame = Self::Frame>,
    {
        self.zip(other, Frame::add)
    }
    /// Apply a pan to the source
    ///
    /// Non-mono sources will be averaged before panning
    fn pan<P>(self, pan: P) -> Pan<Self, P>
    where
        Self: Sized,
        P: Automation,
    {
        Pan { source: self, pan }
    }
    /// Map the source's amplitude's range from [-1, 1] to [0, 1]
    ///
    /// This is useful for sources that are used as automation, since
    /// many values that can be automated are in the range [0, 1].
    fn positive(self) -> Positive<Self>
    where
        Self: Sized,
    {
        Positive { source: self }
    }
    /// Repeat a source `n` times
    fn repeat(self, n: usize) -> Repeat<Self, f64>
    where
        Self: Sized,
    {
        Repeat {
            source: self,
            count_left: Some(n),
            curr: Vec::new(),
            period: None,
            time: 0.0,
            started: false,
        }
    }
    /// Repeat a source indefinitely
    fn repeat_indefinitely(self) -> Repeat<Self, f64>
    where
        Self: Sized,
    {
        Repeat {
            source: self,
            count_left: None,
            curr: Vec::new(),
            period: None,
            time: 0.0,
            started: false,
        }
    }
    /// When repeated, make the source continue where it left off instead of starting over
    ///
    /// This is useful for automation sources that need to not reset when the thing they
    /// are automating is repeated.
    fn no_repeat(self) -> NoRepeat<Self>
    where
        Self: Sized,
    {
        NoRepeat {
            source: Arc::new(Mutex::new(self)),
        }
    }
    /// Apply an attack-decay-sustain envelope to the source
    ///
    /// To apply a release as well, use [`Source::take_release`] or [`Source::maintained`] after this
    fn ads<A, D, S>(self, envelope: AdsEnvelope<A, D, S>) -> Ads<Self, A, D, S>
    where
        Self: Sized,
        A: Automation,
        D: Automation,
        S: Automation,
    {
        Ads {
            source: self,
            time: 0.0,
            envelope,
        }
    }
    /// Keep playing this source as long as the given [`Maintainer`] is not dropped
    fn maintained<R>(self, maintainer: &Maintainer<R>) -> Maintained<Self, R>
    where
        Self: Sized,
        R: Automation + Clone,
    {
        Maintained {
            source: self,
            arc: Arc::downgrade(&maintainer.arc),
            release_dur: maintainer.release_dur.clone(),
            release_curr: 0.0,
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
    /// Buffer the source
    ///
    /// The source returned by this function can be cloned, and while each clone will
    /// track its time seperately, the underlying source will only be read once.
    ///
    /// Useful when reading from sound files or with other sources that cannot be cloned.
    fn buffer(self) -> Buffered<Self>
    where
        Self: Sized,
    {
        Buffered {
            inner: Arc::new(Mutex::new(BufferedInner {
                source: self,
                buffer: Vec::new(),
            })),
            time: 0.0,
        }
    }
    /// Unroll the source so that its samples are flat
    fn unroll(self, sample_rate: f64) -> Unroll<Self>
    where
        Self: Sized,
    {
        Unroll {
            source: self,
            curr: None,
            i: 0,
            sample_rate,
        }
    }
}

pub(crate) type DynamicSource<F> = Box<dyn Source<Frame = F> + Send + 'static>;

/// A source that returns a constant value
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Constant(pub f64);

impl Source for Constant {
    type Frame = f64;
    fn next(&mut self, _sample_rate: f64) -> Option<Self::Frame> {
        Some(self.0)
    }
}

/// Source returned from [`Source::amplify`]
#[derive(Debug, Clone, Copy)]
pub struct Amplify<S, A> {
    source: S,
    amp: A,
}

impl<S, A> Source for Amplify<S, A>
where
    S: Source,
    A: Automation,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let frame = self.source.next(sample_rate)?;
        let amp = self.amp.next_value(sample_rate)?;
        Some(frame.map(|a| a * amp))
    }
}

/// Source returned from [`Source::take`]
#[derive(Debug, Clone, Copy)]
pub struct Take<S, R> {
    source: S,
    duration: f64,
    elapsed: f64,
    release: R,
}

impl<S, R> Source for Take<S, R>
where
    S: Source,
    R: Automation,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        if self.elapsed >= self.duration {
            return None;
        }
        let frame = self.source.next(sample_rate)?;
        let release = self.release.next_value(sample_rate)?;
        let amp = if release == 0.0 {
            1.0
        } else {
            let time_left = self.duration - self.elapsed;
            (time_left / release).min(1.0)
        };
        self.elapsed += 1.0 / sample_rate;
        Some(frame.map(|a| a * amp))
    }
}

/// Source return from [`Source::chain`]
#[derive(Debug, Clone, Copy)]
pub struct Chain<A, B> {
    a: A,
    b: B,
    b_start: Option<f64>,
    time: f64,
}

impl<A, B> Source for Chain<A, B>
where
    A: Source,
    B: Source<Frame = A::Frame>,
{
    type Frame = A::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let frame = if let Some(a) = self.a.next(sample_rate) {
            match self.b_start {
                Some(b_start) if self.time >= b_start => {
                    let b = self
                        .b
                        .next(sample_rate)
                        .unwrap_or_else(|| Self::Frame::uniform(0.0));
                    a.add(b)
                }
                _ => a,
            }
        } else if let Some(b) = self.b.next(sample_rate) {
            b
        } else {
            return None;
        };
        self.time += 1.0 / sample_rate;
        Some(frame)
    }
}

/// Source returned from [`Source::low_pass`]
#[derive(Debug, Clone, Copy)]
pub struct LowPass<S, F>
where
    S: Source,
{
    source: S,
    acc: Option<S::Frame>,
    freq: F,
}

impl<S, F> Source for LowPass<S, F>
where
    S: Source,
    F: Automation,
    S::Frame: std::fmt::Debug,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let freq = self.freq.next_value(sample_rate)?;
        let frame = self.source.next(sample_rate)?;
        Some(if let Some(acc) = &mut self.acc {
            let t = (freq / sample_rate).min(1.0);
            acc.merge(frame, |a, b| lerp(a, b, t));
            acc.clone()
        } else {
            self.acc = Some(frame.clone());
            frame
        })
    }
}

/// Source returned from [`Source::map`]
#[derive(Debug, Clone, Copy)]
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
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        self.source.next(sample_rate).map(&self.f)
    }
}

/// Source returned from [`Source::zip`]
#[derive(Debug, Clone, Copy)]
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
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        Some((self.f)(
            self.a.next(sample_rate)?,
            self.b.next(sample_rate)?,
        ))
    }
}

/// Source returned from [`Source::pan`]
#[derive(Debug, Clone, Copy)]
pub struct Pan<S, P> {
    source: S,
    pan: P,
}

impl<S, P> Source for Pan<S, P>
where
    S: Source,
    P: Automation,
{
    type Frame = Stereo;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let pan = self.pan.next_value(sample_rate)?;
        let pan = (pan + 1.0) / 2.0;
        self.source.next(sample_rate).map(|frame| {
            let frame = frame.avg();
            let left = frame * (1.0 - pan);
            let right = frame * pan;
            [left, right]
        })
    }
}

/// Source returned from [`Source::positive`]
#[derive(Debug, Clone, Copy)]
pub struct Positive<S> {
    source: S,
}

impl<S> Source for Positive<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        self.source
            .next(sample_rate)
            .map(|frame| frame.map(|a| (a + 1.0) / 2.0))
    }
}

/// Source returned from [`Source::repeat`]
#[derive(Debug, Clone)]
pub struct Repeat<S, P> {
    source: S,
    count_left: Option<usize>,
    curr: Vec<S>,
    period: Option<P>,
    time: f64,
    started: bool,
}

impl<S, P> Repeat<S, P> {
    /// Repeat every `period` seconds
    pub fn every<Q>(self, period: Q) -> Repeat<S, Q>
    where
        Self: Sized,
        Q: Automation,
    {
        Repeat {
            source: self.source,
            count_left: self.count_left,
            curr: self.curr,
            period: Some(period),
            time: self.time,
            started: self.started,
        }
    }
}

impl<S, P> Source for Repeat<S, P>
where
    S: Source + Clone,
    P: Automation,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let count_left = &mut self.count_left;
        let mut add_new = || match count_left {
            Some(count_left) => {
                if *count_left > 0 {
                    *count_left -= 1;
                    true
                } else {
                    false
                }
            }
            None => true,
        };
        if let Some(period) = &mut self.period {
            let period = period.next_value(sample_rate)?;
            if self.time >= period {
                if add_new() {
                    self.curr.push(self.source.clone());
                    self.time = 0.0;
                } else {
                    return None;
                }
            }
        }
        if self.curr.is_empty() && (self.period.is_none() || !self.started) {
            if add_new() {
                self.curr.push(self.source.clone());
                self.time = 0.0;
            } else {
                return None;
            }
        }
        let mut frame = Self::Frame::uniform(0.0);
        self.curr.retain_mut(|source| {
            if let Some(next) = source.next(sample_rate) {
                frame.merge(next, Frame::add);
                true
            } else {
                false
            }
        });
        self.time += 1.0 / sample_rate;
        self.started = true;
        Some(frame)
    }
}

/// Source returned from [`Source::no_repeat`]
#[derive(Debug, Clone)]
pub struct NoRepeat<S> {
    source: Arc<Mutex<S>>,
}

impl<S> Source for NoRepeat<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        self.source.lock().next(sample_rate)
    }
}

/// Used to coordinate the dropping of [`Source`]s
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Maintainer<R = f64> {
    arc: Arc<()>,
    release_dur: R,
}

impl<R> Maintainer<R>
where
    R: Default,
{
    /// Create a new maintainer
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R> Maintainer<R>
where
    R: Automation,
{
    /// Create a new maintainer with the given release duration
    pub fn with_release(release_dur: R) -> Self {
        Maintainer {
            arc: Arc::new(()),
            release_dur,
        }
    }
}

/// An attack-decay-sustain evenlope
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsEnvelope<A = f64, D = f64, S = f64> {
    /// The time after the sound starts before it is at its maximum volume
    pub attack: A,
    /// The time between the maximum amplitude and the sustain amplitude
    pub decay: D,
    /// The sustain amplitude
    pub sustain: S,
}

impl<A, D, S> Default for AdsEnvelope<A, D, S>
where
    A: From<f64>,
    D: From<f64>,
    S: From<f64>,
{
    fn default() -> Self {
        AdsEnvelope {
            attack: 0.0.into(),
            decay: 0.0.into(),
            sustain: 1.0.into(),
        }
    }
}

impl<A, D, S> AdsEnvelope<A, D, S>
where
    A: Automation,
    D: Automation,
    S: Automation,
{
    /// Create a new ADS envelope
    pub fn new(attack: A, decay: D, sustain: S) -> Self {
        Self {
            attack,
            decay,
            sustain,
        }
    }
}

/// Source returned from [`Source::ads`]
#[derive(Debug, Clone, Copy)]
pub struct Ads<Src, A, D, S> {
    source: Src,
    time: f64,
    envelope: AdsEnvelope<A, D, S>,
}

impl<Src, A, D, S> Source for Ads<Src, A, D, S>
where
    Src: Source,
    A: Automation,
    D: Automation,
    S: Automation,
{
    type Frame = Src::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let frame = self.source.next(sample_rate)?;
        let attack = self.envelope.attack.next_value(sample_rate)?;
        let decay = self.envelope.decay.next_value(sample_rate)?;
        let sustain = self.envelope.sustain.next_value(sample_rate)?;
        let amp = if self.time < attack {
            self.time / attack
        } else {
            let after_attack = self.time - attack;
            if after_attack < decay {
                (1.0 - after_attack / decay) * (1.0 - sustain) + sustain
            } else {
                sustain
            }
        };
        self.time += 1.0 / sample_rate;
        Some(frame.map(|s| s * amp))
    }
}

/// Source returned from [`Source::maintained`]
#[derive(Debug, Clone)]
pub struct Maintained<S, R> {
    source: S,
    arc: Weak<()>,
    release_dur: R,
    release_curr: f64,
}

impl<S, R> Source for Maintained<S, R>
where
    S: Source,
    R: Automation,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let release_dur = self.release_dur.next_value(sample_rate)?;
        if Weak::strong_count(&self.arc) == 0 {
            if self.release_curr < release_dur {
                let amp = 1.0 - self.release_curr / release_dur;
                let frame = self.source.next(sample_rate)?.map(|s| s * amp);
                self.release_curr += 1.0 / sample_rate;
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
#[derive(Debug, Clone)]
pub struct InspectedSource<S: Source> {
    source: S,
    curr: Shared<Option<S::Frame>>,
}

/// Allows the inspection of a [`Source`]'s current frame
#[derive(Debug, Clone)]
pub struct SourceInspector<F> {
    curr: Shared<Option<F>>,
}

impl<S: Source> Source for InspectedSource<S> {
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
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

/// Source returned from [`Source::buffer`]
pub struct Buffered<S: Source> {
    inner: Arc<Mutex<BufferedInner<S>>>,
    time: f64,
}

struct BufferedInner<S: Source> {
    source: S,
    buffer: Vec<S::Frame>,
}

impl<S> Source for Buffered<S>
where
    S: Source,
{
    type Frame = S::Frame;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let index = (self.time * sample_rate) as usize;
        let mut inner = self.inner.lock();
        while index >= inner.buffer.len() {
            let frame = inner.source.next(sample_rate)?;
            inner.buffer.push(frame);
        }
        self.time += 1.0 / sample_rate;
        Some(inner.buffer[index].clone())
    }
}

impl<S> Clone for Buffered<S>
where
    S: Source,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            time: self.time,
        }
    }
}

/// Source that resamples a dynamic source to have a fixed frame size
#[derive(Debug, Clone, Copy)]
pub struct Resample<S, F> {
    source: S,
    time: f64,
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
                sample.set_channel(0, sum / count as f64);
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
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let target_time = self.time + 1.0 / sample_rate;
        while self.time < target_time {
            self.frame = self.get_frame();
            self.time += 1.0 / self.source.sample_rate();
        }
        self.frame.clone()
    }
}

/// Source returned from [`Source::unroll`]
pub struct Unroll<S: Source> {
    source: S,
    sample_rate: f64,
    curr: Option<S::Frame>,
    i: usize,
}

impl<S> Iterator for Unroll<S>
where
    S: Source,
{
    type Item = f64;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(curr) = &self.curr {
            if self.i < <S::Frame as Frame>::CHANNELS {
                let amplitude = curr.get_channel(self.i);
                self.i += 1;
                Some(amplitude)
            } else {
                self.curr = None;
                self.i = 0;
                self.next()
            }
        } else if let Some(curr) = self.source.next(self.sample_rate) {
            self.curr = Some(curr);
            self.next()
        } else {
            None
        }
    }
}

impl<S> UnrolledSource for Unroll<S>
where
    S: Source,
{
    fn channels(&self) -> usize {
        <S::Frame as Frame>::CHANNELS
    }
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
}
