use std::sync::Arc;

use parking_lot::Mutex;

use crate::{source::*, Frame};

/// Trait for combining [`Source`]s
pub trait MixerInterface {
    /// The frame type
    type Frame: Frame;
    /// Add a source to the mixer to be played immediately
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = Self::Frame> + Send + 'static;
}

/// The [`Source`] used to play sources combined in a [`Mixer`]
#[derive(Clone)]
pub struct MixerSource<F> {
    sources: Arc<Mutex<Vec<DynamicSource<F>>>>,
}

/// An interface for combining [`Source`]s into a new [`Source`]
///
/// [`Mixer`] is the interface, which implements [`MixerInterface`].
/// [`MixerSource`] is the actual [`Source`].
#[derive(Clone)]
pub struct Mixer<F> {
    pub(crate) sources: Arc<Mutex<Vec<DynamicSource<F>>>>,
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
        self.sources.lock().push(Box::new(source));
    }
}

impl<F> Mixer<F> {
    /// Create a new mixer interface and corresponing [`Source`]
    pub fn new() -> (Mixer<F>, MixerSource<F>) {
        let sources = Arc::new(Mutex::new(Vec::new()));
        (
            Mixer {
                sources: Arc::clone(&sources),
            },
            MixerSource { sources },
        )
    }
}

impl<F> Source for MixerSource<F>
where
    F: Frame,
{
    type Frame = F;
    fn next(&mut self, sample_rate: f32) -> Option<Self::Frame> {
        let mut sources = self.sources.lock();
        let mut frame = F::uniform(0.0);
        sources.retain_mut(|source| {
            if let Some(this_frame) = source.next(sample_rate) {
                frame = frame.clone().merge(this_frame, |a, b| a + b);
                true
            } else {
                false
            }
        });
        Some(frame)
    }
}
