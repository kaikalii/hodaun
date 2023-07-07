use std::{thread, time::Duration};

use crate::{
    cpal::{
        traits::{DeviceTrait, HostTrait, StreamTrait},
        *,
    },
    Mixer,
};

use crate::{
    Amplitude, BuildSystemAudioError, BuildSystemAudioResult, DeviceIoBuilder, Frame, Source,
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

impl<F> OutputDeviceMixer<F>
where
    F: Frame + Send + 'static,
{
    /// Create a mixer using the default output device
    pub fn with_default_device() -> BuildSystemAudioResult<Self> {
        DeviceIoBuilder::default_output().build_output()
    }
    /// Get the sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate as f64
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
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {err}");
        let mixer = Mixer::new();
        let mixer_clone = mixer.clone();
        macro_rules! output_stream {
            ($sample:ty) => {
                device.build_output_stream(
                    &config,
                    write_sources::<F, $sample>(mixer_clone, &config),
                    err_fn,
                    None,
                )
            };
        }
        let stream = match sample_format {
            SampleFormat::F32 => output_stream!(f32),
            SampleFormat::I16 => output_stream!(i16),
            SampleFormat::U16 => output_stream!(u16),
            SampleFormat::I8 => output_stream!(i8),
            SampleFormat::I32 => output_stream!(i32),
            SampleFormat::I64 => output_stream!(i64),
            SampleFormat::U8 => output_stream!(u8),
            SampleFormat::U32 => output_stream!(u32),
            SampleFormat::U64 => output_stream!(u64),
            SampleFormat::F64 => output_stream!(f64),
            _ => return Err(BuildSystemAudioError::UnsupportedSampleFormat),
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
    /// Method-chaining version of [`OutputDeviceMixer::play`]
    ///
    /// # Example
    /// ```
    /// use hodaun::*;
    ///
    /// let output = OutputDeviceMixer::<Mono>::with_default_device().unwrap().playing().unwrap();
    /// ```
    pub fn playing(mut self) -> Result<Self, PlayStreamError> {
        self.play()?;
        Ok(self)
    }
    /// Add a source to the mixer to be played immediately
    pub fn add<S>(&self, source: S)
    where
        S: Source<Frame = F> + Send + 'static,
    {
        self.mixer.add(source);
    }
    /// Get a reference to the mixer
    pub fn mixer(&self) -> &Mixer<F> {
        &self.mixer
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
    let sample_rate = config.sample_rate.0 as f64;
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
            *out_sample = A::from_f64(frame_buffer[i]);
            i += 1;
        }
    }
}
