use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{Amplitude, Frame, MixedSource, MixerInterface, Source};

/// Get the default output device
pub fn default_output_device() -> Option<cpal::Device> {
    cpal::default_host().default_output_device()
}

/// Mixes audio sources and outputs them to a device
pub struct DeviceMixer<F> {
    device: cpal::Device,
    sources: Arc<Mutex<Vec<MixedSource<F>>>>,
    stream: Option<cpal::Stream>,
}

impl<F> DeviceMixer<F> {
    /// Create a new [`DeviceMixer`] that will output on the given device
    pub fn new(device: cpal::Device) -> Self {
        DeviceMixer {
            device,
            sources: Default::default(),
            stream: None,
        }
    }
    /// Create a new [`DeviceMixer`] that will output on the default output device
    pub fn with_default_device() -> Option<Self> {
        default_output_device().map(Self::new)
    }
    /// Get the default supported stream config from the mixer
    pub fn default_config(&self) -> Option<cpal::SupportedStreamConfig> {
        self.device
            .supported_output_configs()
            .ok()
            .and_then(|mut scs| scs.next())
            .map(|sc| sc.with_max_sample_rate())
    }
    /// Get the sample rate of the default stream config
    pub fn default_sample_rate(&self) -> Option<f32> {
        self.default_config()
            .map(|config| config.sample_rate().0 as f32)
    }
}

impl<F> MixerInterface for DeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    type Frame = F;
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        self.sources.lock().unwrap().push(MixedSource::new(source));
    }
}

impl<F> DeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    /// Start the mixer playing without blocking the thread
    ///
    /// Playback will stop if the mixer is dropped
    pub fn play(&mut self) -> Result<(), cpal::PlayStreamError> {
        if let Some(config) = self.default_config() {
            self.play_with_config(config)
        } else {
            Ok(())
        }
    }
    /// Play the mixer, blocking the thread until all sources have finished
    pub fn blocking_play(&mut self) -> Result<(), cpal::PlayStreamError> {
        if let Some(config) = self.default_config() {
            self.blocking_play_with_config(config)?;
        }
        Ok(())
    }
    /// Play the mixer with the given config, blocking the thread until all sources have finished
    pub fn blocking_play_with_config(
        &mut self,
        config: cpal::SupportedStreamConfig,
    ) -> Result<(), cpal::PlayStreamError> {
        self.play_with_config(config)?;
        while self
            .sources
            .lock()
            .unwrap()
            .iter()
            .any(|source| !source.finished())
        {}
        Ok(())
    }
    /// Start the mixer playing with the given config without blocking the thread
    ///
    /// Playback will stop if the mixer is dropped
    pub fn play_with_config(
        &mut self,
        config: cpal::SupportedStreamConfig,
    ) -> Result<(), cpal::PlayStreamError> {
        let sample_format = config.sample_format();
        let config = cpal::StreamConfig::from(config);
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        macro_rules! output_stream {
            ($sample:ty) => {
                self.device.build_output_stream(
                    &config,
                    self.write_sources::<$sample>(&config),
                    err_fn,
                )
            };
        }
        let stream = match sample_format {
            cpal::SampleFormat::F32 => output_stream!(f32),
            cpal::SampleFormat::I16 => output_stream!(i16),
            cpal::SampleFormat::U16 => output_stream!(u16),
        }
        .unwrap();
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
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
            'sources_loop: for ms in &mut *sources.lock().unwrap() {
                let mut b = 0;
                loop {
                    let frame = if let Some(frame) = ms.frame() {
                        frame
                    } else {
                        continue 'sources_loop;
                    };
                    while i < channels as usize && b < buffer.len() {
                        let c = i % F::CHANNELS;
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
                        i = 0;
                        break;
                    }
                }
            }
        }
    }
}
