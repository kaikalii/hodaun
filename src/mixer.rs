use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::source::*;

pub(crate) struct MixedSource<F> {
    source: Box<dyn Source<Frame = F> + Send + 'static>,
    t: f32,
    curr: Option<F>,
    finished: bool,
}

impl<F> MixedSource<F>
where
    F: Frame,
{
    pub fn new<S>(source: S) -> Self
    where
        S: Source<Frame = F> + Send + 'static,
    {
        MixedSource {
            source: Box::new(source),
            t: 0.0,
            curr: None,
            finished: false,
        }
    }
    pub fn frame(&mut self, sample_rate: f32) -> Option<F> {
        if self.t <= 0.0 {
            if let Some(frame) = self.source.next(sample_rate) {
                self.t += 1.0 / sample_rate;
                self.curr = Some(frame.clone());
                Some(frame)
            } else {
                self.finished = true;
                None
            }
        } else {
            self.curr.clone()
        }
    }
    pub fn advance(&mut self, sample_rate: f32) {
        self.t -= 1.0 / sample_rate;
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
}

/// Trait for combining [`Source`]s
pub trait MixerInterface {
    /// The frame type
    type Frame: Frame;
    /// Add a source to the mixer
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = Self::Frame> + Send + 'static;
}

/// The [`Source`] used to play sources combined in a [`Mixer`]
pub struct MixerSource<F> {
    sources: Vec<MixedSource<F>>,
    recv: Receiver<Box<dyn Source<Frame = F> + Send + 'static>>,
}

/// An interface for combining [`Source`]s into a new [`Source`]
///
/// [`Mixer`] is the interface, which implements [`MixerInterface`].
/// [`MixerSource`] is the actual [`Source`].
#[derive(Clone)]
pub struct Mixer<F> {
    send: Sender<Box<dyn Source<Frame = F> + Send + 'static>>,
}

impl<F> MixerInterface for Mixer<F>
where
    F: Frame,
{
    type Frame = F;
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = Self::Frame> + Send + 'static,
    {
        let _ = self.send.send(Box::new(source));
    }
}

impl<F> Mixer<F> {
    /// Create a new mixer interface and corresponing [`Source`]
    pub fn new() -> (Mixer<F>, MixerSource<F>) {
        let (send, recv) = unbounded();
        (
            Mixer { send },
            MixerSource {
                sources: Vec::new(),
                recv,
            },
        )
    }
}

impl<F> Source for MixerSource<F>
where
    F: Frame,
{
    type Frame = F;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        self.sources
            .extend(self.recv.try_iter().map(|source| MixedSource {
                source,
                t: 0.0,
                curr: None,
                finished: false,
            }));
        let mut frame = F::uniform(0.0);
        for ms in &mut self.sources {
            if let Some(this_frame) = ms.frame(sample_rate) {
                frame = frame.merge(this_frame, |a, b| a + b);
            }
            ms.advance(sample_rate);
        }
        self.sources.retain(|ms| !ms.finished());
        Some(frame)
    }
}
