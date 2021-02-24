use std::sync::mpsc::*;

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
    pub fn frame(&mut self) -> Option<F> {
        if self.t <= 0.0 {
            if let Some(frame) = self.source.next() {
                self.t += 1.0 / self.source.sample_rate();
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

pub struct MixerSource<F> {
    sources: Vec<MixedSource<F>>,
    recv: Receiver<Box<dyn Source<Frame = F> + Send + 'static>>,
}

#[derive(Clone)]
pub struct Mixer<F> {
    send: Sender<Box<dyn Source<Frame = F> + Send + 'static>>,
}

impl<F> Mixer<F> {
    pub fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        let _ = self.send.send(Box::new(source));
    }
}

impl<F> Mixer<F> {
    pub fn new() -> (Mixer<F>, MixerSource<F>) {
        let (send, recv) = channel();
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
    fn sample_rate(&self) -> f32 {
        self.sources
            .iter()
            .map(|ms| ms.source.sample_rate())
            .min_by(|a, b| a.partial_cmp(b).expect("source has NaN sample rate"))
            .unwrap_or(1.0)
    }
    fn next(&mut self) -> Option<Self::Frame> {
        self.sources
            .extend(self.recv.try_iter().map(|source| MixedSource {
                source,
                t: 0.0,
                curr: None,
                finished: false,
            }));
        let mut frame = F::default();
        let sample_rate = self.sample_rate();
        for ms in &mut self.sources {
            if let Some(this_frame) = ms.frame() {
                frame = frame.join(this_frame, |a, b| a + b);
            }
            ms.advance(sample_rate);
        }
        self.sources.retain(|ms| !ms.finished());
        Some(frame)
    }
}
