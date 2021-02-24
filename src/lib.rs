pub mod gen;
pub mod source;

use std::sync::{Arc, Mutex};

use cpal::traits::*;

pub use cpal;
pub use source::{Frame, Mono, Source, Stereo};

pub fn default_output_device() -> Option<cpal::Device> {
    cpal::default_host().default_output_device()
}

type MixedSource<F> = (Box<dyn Source<Frame = F> + Send + 'static>, f32);

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
        self.sources.lock().unwrap().push((Box::new(source), 0.0));
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
        let mut curr_frame = None;
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0 as f32;
        let sources = Arc::clone(&self.sources);
        move |buffer, _| {
            buffer.fill(A::MIDPOINT);
            for (source, t) in &mut *sources.lock().unwrap() {
                let mut b = 0;
                loop {
                    if curr_frame.is_none() {
                        if let Some(frame) = source.next() {
                            *t += 1.0 / source.sample_rate();
                            curr_frame = Some(frame);
                        } else {
                            return;
                        }
                    }
                    let frame = curr_frame.as_ref().unwrap();
                    while i < channels as usize && b < buffer.len() {
                        let c = i % frame.channels();
                        let a = frame.get_channel(c);
                        buffer[b] += A::from_f32(a);
                        i += 1;
                        b += 1;
                    }
                    *t -= 1.0 / sample_rate;
                    if i == channels as usize {
                        if *t <= 0.0 {
                            curr_frame = None;
                        }
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
    mixer.add(gen::SineWave::new(220.0, 32000.0));
    mixer.add(gen::SquareWave::new(440.0, 32000.0));
    mixer.play().unwrap();
    sleep(Duration::from_secs(1));
}
