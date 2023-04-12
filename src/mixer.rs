use std::sync::Arc;

use parking_lot::Mutex;

use crate::{source::*, Frame};

/// Trait for mixing [`Source`]s
pub trait Mix {
    /// The frame type
    type Frame: Frame;
    /// Add a source to the mixer to be played immediately
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = Self::Frame> + Send + 'static;
}

/// An [`Source`] that mixes multiple [`Source`]s together
#[derive(Clone)]
pub struct Mixer<F> {
    pub(crate) sources: Arc<Mutex<Vec<DynamicSource<F>>>>,
}

impl<F> Mix for Mixer<F>
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

impl<F> Default for Mixer<F> {
    fn default() -> Self {
        Mixer {
            sources: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<F> Mixer<F> {
    /// Create a new mixer
    pub fn new() -> Mixer<F> {
        Self::default()
    }
}

impl<F> Source for Mixer<F>
where
    F: Frame,
{
    type Frame = F;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let mut sources = self.sources.lock();
        let mut frame = F::uniform(0.0);
        sources.retain_mut(|source| {
            if let Some(this_frame) = source.next(sample_rate) {
                frame.merge(this_frame, |a, b| a + b);
                true
            } else {
                false
            }
        });
        Some(frame)
    }
}
