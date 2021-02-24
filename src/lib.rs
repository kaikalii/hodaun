pub mod gen;
pub mod mixer;
pub mod source;

use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use cpal::traits::*;

pub use cpal;
pub use mixer::*;
pub use source::{Frame, Mono, Source, Stereo};

pub fn default_output_device() -> Option<cpal::Device> {
    cpal::default_host().default_output_device()
}

pub struct DeviceMixer<F> {
    pub device: Option<cpal::Device>,
    sources: Arc<Mutex<Vec<MixedSource<F>>>>,
    stream: Option<cpal::Stream>,
}

impl<F> DeviceMixer<F> {
    pub fn new(device: cpal::Device) -> Self {
        DeviceMixer {
            device: Some(device),
            sources: Default::default(),
            stream: None,
        }
    }
}

impl<F> Default for DeviceMixer<F> {
    fn default() -> Self {
        DeviceMixer {
            device: default_output_device(),
            sources: Default::default(),
            stream: None,
        }
    }
}

impl<F> DeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    pub fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        self.sources.lock().unwrap().push(MixedSource::new(source));
    }
    pub fn default_config(&self) -> Option<cpal::SupportedStreamConfig> {
        self.device.as_ref().and_then(|device| {
            device
                .supported_output_configs()
                .ok()
                .and_then(|mut scs| scs.next())
                .map(|sc| sc.with_max_sample_rate())
        })
    }
    pub fn play(&mut self) -> Result<(), cpal::PlayStreamError> {
        if let Some(config) = self.default_config() {
            self.play_with_config(config)
        } else {
            Ok(())
        }
    }
    pub fn play_with_config(
        &mut self,
        config: cpal::SupportedStreamConfig,
    ) -> Result<(), cpal::PlayStreamError> {
        if let Some(device) = self.device.as_ref() {
            let sample_format = config.sample_format();
            let config = cpal::StreamConfig::from(config);
            let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
            let stream = match sample_format {
                cpal::SampleFormat::F32 => {
                    device.build_output_stream(&config, self.write_sources::<f32>(&config), err_fn)
                }
                cpal::SampleFormat::I16 => {
                    device.build_output_stream(&config, self.write_sources::<i16>(&config), err_fn)
                }
                cpal::SampleFormat::U16 => {
                    device.build_output_stream(&config, self.write_sources::<u16>(&config), err_fn)
                }
            }
            .unwrap();
            stream.play()?;
            self.stream = Some(stream);
            Ok(())
        } else {
            Ok(())
        }
    }
    fn write_sources<A>(
        &self,
        config: &cpal::StreamConfig,
    ) -> impl FnMut(&mut [A], &cpal::OutputCallbackInfo)
    where
        A: Amplitude,
    {
        let mut i = 0;
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0 as f32;
        let sources = Arc::clone(&self.sources);
        move |buffer, _| {
            buffer.fill(A::MIDPOINT);
            for ms in &mut *sources.lock().unwrap() {
                let mut b = 0;
                loop {
                    let frame = if let Some(frame) = ms.frame() {
                        frame
                    } else {
                        return;
                    };
                    while i < channels as usize && b < buffer.len() {
                        let c = i % frame.channels();
                        let a = frame.get_channel(c);
                        buffer[b] += A::from_f32(a);
                        i += 1;
                        b += 1;
                    }
                    ms.advance(sample_rate);
                    if i == channels as usize {
                        i = 0;
                    }
                    if b == buffer.len() {
                        break;
                    }
                }
            }
        }
    }
}

trait Amplitude: Clone + std::ops::AddAssign<Self> {
    const MIDPOINT: Self;
    fn from_f32(f: f32) -> Self;
}

impl Amplitude for f32 {
    const MIDPOINT: Self = 0.0;
    fn from_f32(f: f32) -> Self {
        f
    }
}

impl Amplitude for u16 {
    const MIDPOINT: Self = u16::MAX / 2;
    fn from_f32(f: f32) -> Self {
        const HALF_U16_MAX: f32 = u16::MAX as f32 * 0.5;
        (f * HALF_U16_MAX + HALF_U16_MAX) as u16
    }
}

impl Amplitude for i16 {
    const MIDPOINT: Self = 0;
    fn from_f32(f: f32) -> Self {
        const I16_MAX: f32 = i16::MAX as f32;
        (f * I16_MAX) as i16
    }
}

#[test]
fn test() {
    use std::{thread::sleep, time::Duration};
    let mut mixer = DeviceMixer::default();
    let (mx, mx_source) = Mixer::new();
    mixer.add(mx_source);
    mx.add(gen::SineWave::new(220.0, 32000.0));
    // mixer.add(gen::SquareWave::new(440.0, 32000.0));
    mixer.play().unwrap();
    sleep(Duration::from_secs(1));
}

#[derive(Clone, Default)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Shared<T> {
    pub fn new(val: T) -> Self {
        Shared(Arc::new(Mutex::new(val)))
    }
    pub fn set(&self, val: T) {
        *self.0.lock().unwrap() = val;
    }
}

impl<T> Shared<T>
where
    T: Copy,
{
    pub fn get(&self) -> T {
        *self.0.lock().unwrap()
    }
}

impl<T> Shared<T>
where
    T: Clone,
{
    pub fn cloned(&self) -> T {
        self.0.lock().unwrap().clone()
    }
}

impl<T> PartialEq for Shared<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        *self.0.lock().unwrap() == *other.0.lock().unwrap()
    }
}

impl<T> Eq for Shared<T> where T: Eq {}

impl<T> PartialOrd for Shared<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0
            .lock()
            .unwrap()
            .partial_cmp(&*other.0.lock().unwrap())
    }
}

impl<T> Ord for Shared<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.lock().unwrap().cmp(&*other.0.lock().unwrap())
    }
}

impl<T> Hash for Shared<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.lock().unwrap().hash(state);
    }
}

impl<T> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().unwrap().fmt(f)
    }
}

impl<T> fmt::Display for Shared<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().unwrap().fmt(f)
    }
}
