use std::marker::PhantomData;

pub type Mono = [f32; 1];
pub type Stereo = [f32; 2];

pub trait Frame: Default + Clone {
    fn channels(&self) -> usize;
    fn get_channel(&self, index: usize) -> f32;
    fn map<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32;
    fn join<F>(self, other: Self, f: F) -> Self
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
    fn join<F>(mut self, other: Self, f: F) -> Self
    where
        F: Fn(f32, f32) -> f32,
    {
        for (a, b) in self.as_mut().iter_mut().zip(other.as_ref()) {
            *a = f(*a, *b);
        }
        self
    }
}

pub trait Source {
    type Frame: Frame;
    fn sample_rate(&self) -> f32;
    fn next(&mut self) -> Option<Self::Frame>;
    fn amplify(self, amp: f32) -> Amplify<Self>
    where
        Self: Sized,
    {
        Amplify { source: self, amp }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct Silence<F = [f32; 1]> {
    sample_rate: f32,
    pd: PhantomData<F>,
}

impl Silence {
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
