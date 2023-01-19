use std::{thread, time::Duration};

use crate::{
    cpal::{
        traits::{DeviceTrait, HostTrait, StreamTrait},
        *,
    },
    Mixer,
};

use crate::{
    Amplitude, BuildSystemAudioError, BuildSystemAudioResult, DeviceIoBuilder, Frame, Mix, Source,
};

/// Get the default output device
pub fn default_output_device() -> Option<Device> {
    default_host().default_output_device()
}

/// Mixes audio sources and outputs them to a device
///
/// It can be created with either [`OutputDeviceMixer::with_default_device`] or [`DeviceIoBuilder::build_output`]
pub struct OutputDeviceMixer<F> {
    mixer: Mixer<F>,
    stream: Stream,
    sample_rate: u32,
}

impl<F> Mix for OutputDeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    type Frame = F;
    fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        self.mixer.add(source);
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
        let mixer = Mixer::new();
        let mixer_clone = mixer.clone();
        macro_rules! output_stream {
            ($sample:ty) => {
                device.build_output_stream(
                    &config,
                    write_sources::<F, $sample>(mixer_clone, &config),
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
            mixer,
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
            .mixer
            .sources
            .try_lock()
            .map_or(true, |sources| !sources.is_empty())
        {
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
}

fn write_sources<F, A>(
    mut mixer: Mixer<F>,
    config: &StreamConfig,
) -> impl FnMut(&mut [A], &OutputCallbackInfo)
where
    F: Frame,
    A: Amplitude,
{
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f32;
    let mut frame_buffer = vec![0.0; channels];
    let mut i = channels;
    move |buffer, _| {
        buffer.fill(A::MIDPOINT);
        for out_sample in buffer {
            if i >= channels {
                i = 0;
                if let Some(frame) = mixer.next(sample_rate) {
                    frame.write_slice(&mut frame_buffer);
                } else {
                    break;
                }
            }
            *out_sample = A::from_f32(frame_buffer[i]);
            i += 1;
        }
    }
}
