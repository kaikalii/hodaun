pub type Mono = [f32; 1];
pub type Stereo = [f32; 2];

pub trait Frame: Send + 'static {
    fn channels(&self) -> usize;
    fn get_channel(&self, index: usize) -> f32;
    fn map<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32;
}

impl<T> Frame for T
where
    T: AsRef<[f32]> + AsMut<[f32]> + Send + 'static,
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
}

pub trait Source: Send + 'static {
    type Frame: Frame;
    fn sample_rate(&self) -> f32;
    fn next(&mut self) -> Option<Self::Frame>;
}
