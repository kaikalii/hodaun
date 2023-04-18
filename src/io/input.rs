use std::sync::mpsc;

use crate::cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    *,
};

use crate::{
    source::UnrolledSource, BuildSystemAudioError, BuildSystemAudioResult, DeviceIoBuilder,
};

/// Get the default input device
pub fn default_input_device() -> Option<Device> {
    default_host().default_input_device()
}

/// An [`UnrolledSource`] that receives audio samples from the system audio input
///
/// It can be created with either [`InputDeviceSource::with_default_device`] or [`DeviceIoBuilder::build_input`]
///
/// It can be turned into a source with [`UnrolledSource::resample`]
pub struct InputDeviceSource {
    _stream: Stream,
    recv: mpsc::Receiver<f64>,
    sample_rate: u32,
    channels: u16,
}

unsafe impl Send for InputDeviceSource {}

impl Iterator for InputDeviceSource {
    type Item = f64;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv.recv().ok()
    }
}

impl UnrolledSource for InputDeviceSource {
    fn channels(&self) -> usize {
        self.channels as usize
    }
    fn sample_rate(&self) -> f64 {
        self.sample_rate as f64
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
        let err_fn = |err| eprintln!("An error occurred on the input audio stream: {err}");
        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();
        let (send, recv) = mpsc::channel();
        macro_rules! input_stream {
            ($sample:ty, |$x:ident| $convert:expr) => {
                device.build_input_stream(
                    &config,
                    move |data: &[$sample], _: &InputCallbackInfo| {
                        for &$x in data {
                            let _ = send.send($convert);
                        }
                    },
                    err_fn,
                    None,
                )
            };
        }
        let stream = match sample_format {
            SampleFormat::F32 => input_stream!(f32, |x| x as f64),
            SampleFormat::I16 => input_stream!(i16, |x| x as f64 / i16::MAX as f64),
            SampleFormat::U16 => input_stream!(u16, |x| x as f64 - u16::MAX as f64 / 2.0),
            SampleFormat::I8 => input_stream!(i8, |x| x as f64 / i8::MAX as f64),
            SampleFormat::I32 => input_stream!(i32, |x| x as f64 / i32::MAX as f64),
            SampleFormat::I64 => input_stream!(i64, |x| x as f64 / i64::MAX as f64),
            SampleFormat::U8 => input_stream!(u8, |x| x as f64 - u8::MAX as f64 / 2.0),
            SampleFormat::U32 => input_stream!(u32, |x| x as f64 - u32::MAX as f64 / 2.0),
            SampleFormat::U64 => input_stream!(u64, |x| x as f64 - u64::MAX as f64 / 2.0),
            SampleFormat::F64 => input_stream!(f64, |x| x),
            _ => return Err(BuildSystemAudioError::UnsupportedSampleFormat),
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
