use std::sync::mpsc;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, Sample, SampleFormat, Stream, StreamConfig,
};

use crate::{
    source::UnrolledSource, BuildSystemAudioError, BuildSystemAudioResult, DeviceIoBuilder,
};

/// Get the default input device
pub fn default_input_device() -> Option<cpal::Device> {
    cpal::default_host().default_input_device()
}

/// A [`SignalSource`] that receives audio samples from the system audio input
pub struct InputDeviceSource {
    _stream: Stream,
    recv: mpsc::Receiver<f32>,
    sample_rate: u32,
    channels: u16,
}

unsafe impl Send for InputDeviceSource {}

impl Iterator for InputDeviceSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv.recv().ok()
    }
}

impl UnrolledSource for InputDeviceSource {
    fn sample_rate(&self) -> f32 {
        self.sample_rate as f32
    }
    fn channels(&self) -> usize {
        self.channels as usize
    }
}

impl InputDeviceSource {
    /// Create a source using the default input device
    pub fn with_default_device() -> BuildSystemAudioResult<Self> {
        DeviceIoBuilder::default_input().build_input()
    }
    pub(crate) fn from_builder(builder: DeviceIoBuilder) -> BuildSystemAudioResult<Self> {
        let device = if let Some(device) = builder.device {
            device
        } else {
            default_input_device().ok_or(BuildSystemAudioError::NoDevice)?
        };
        let config = if let Some(config) = builder.config {
            config
        } else {
            device.default_input_config()?
        };
        let err_fn = |err| eprintln!("An error occurred on the input audio stream: {}", err);
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();
        let (send, recv) = mpsc::channel();
        macro_rules! input_stream {
            ($sample:ty) => {
                device.build_input_stream(
                    &config,
                    move |data: &[$sample], _: &InputCallbackInfo| {
                        for &s in data {
                            let _ = send.send(s.to_f32());
                        }
                    },
                    err_fn,
                )
            };
        }
        let stream = match sample_format {
            SampleFormat::F32 => input_stream!(f32),
            SampleFormat::I16 => input_stream!(i16),
            SampleFormat::U16 => input_stream!(u16),
        }?;

        stream.play()?;

        Ok(InputDeviceSource {
            _stream: stream,
            recv,
            channels: config.channels,
            sample_rate: config.sample_rate.0,
        })
    }
}
