use std::sync::Arc;

use parking_lot::Mutex;

use crate::{source::*, Frame};

/// An [`Source`] that mixes multiple [`Source`]s together
#[derive(Clone)]
pub struct Mixer<F> {
    pub(crate) sources: Arc<Mutex<Vec<DynamicSource<F>>>>,
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
    /// Add a source to the mixer to be played immediately
    pub fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        self.sources.lock().push(Box::new(source));
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
