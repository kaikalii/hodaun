use std::sync::{Arc, Mutex};

use crate::cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    *,
};

use crate::{
    Amplitude, BuildSystemAudioError, BuildSystemAudioResult, DeviceIoBuilder, Frame, MixedSource,
    MixerInterface, Source,
};

/// Get the default output device
pub fn default_output_device() -> Option<Device> {
    default_host().default_output_device()
}

type OutputDeviceMixerSources<F> = Arc<Mutex<Vec<MixedSource<F>>>>;

/// Mixes audio sources and outputs them to a device
///
/// It can be created with either [`OutputDeviceMixer::with_default_device`] or [`DeviceIoBuilder::build_output`]
pub struct OutputDeviceMixer<F> {
    sources: OutputDeviceMixerSources<F>,
    stream: Stream,
    sample_rate: u32,
}

impl<F> MixerInterface for OutputDeviceMixer<F>
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

impl<F> OutputDeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    /// Create a mixer using the default output device
    pub fn with_default_device() -> BuildSystemAudioResult<Self> {
        DeviceIoBuilder::default_output().build_output()
    }
    /// Get the sample rate
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate as f32
    }
    pub(crate) fn from_builder(builder: DeviceIoBuilder) -> BuildSystemAudioResult<Self> {
        let device = if let Some(device) = builder.device {
            device
        } else {
            default_output_device().ok_or(BuildSystemAudioError::NoDevice)?
        };
        let config = if let Some(config) = builder.config {
            config
        } else {
            device.default_output_config()?
        };
        let sample_format = config.sample_format();
        let config = StreamConfig::from(config);
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sources = OutputDeviceMixerSources::default();
        macro_rules! output_stream {
            ($sample:ty) => {
                device.build_output_stream(
                    &config,
                    write_sources::<F, $sample>(&sources, &config),
                    err_fn,
                )
            };
        }
        let stream = match sample_format {
            SampleFormat::F32 => output_stream!(f32),
            SampleFormat::I16 => output_stream!(i16),
            SampleFormat::U16 => output_stream!(u16),
        }
        .unwrap();
        Ok(OutputDeviceMixer {
            sources,
            stream,
            sample_rate: config.sample_rate.0,
        })
    }
    /// Start the mixer playing without blocking the thread
    ///
    /// Playback will stop if the mixer is dropped
    pub fn play(&mut self) -> Result<(), PlayStreamError> {
        self.stream.play()
    }
    /// Play the mixer, blocking the thread until all sources have finished
    pub fn play_blocking(&mut self) -> Result<(), PlayStreamError> {
        self.play()?;
        while self
            .sources
            .lock()
            .unwrap()
            .iter()
            .any(|source| !source.finished())
        {}
        Ok(())
    }
}

fn write_sources<F, A>(
    sources: &OutputDeviceMixerSources<F>,
    config: &StreamConfig,
) -> impl FnMut(&mut [A], &OutputCallbackInfo)
where
    F: Frame,
    A: Amplitude,
{
    let mut i = 0;
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f32;
    let sources = Arc::clone(sources);
    move |buffer, _| {
        buffer.fill(A::MIDPOINT);
        'sources_loop: for ms in &mut *sources.lock().unwrap() {
            let mut b = 0;
            loop {
                let frame = if let Some(frame) = ms.frame(sample_rate) {
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
