#[cfg(feature = "input")]
mod input;
#[cfg(feature = "output")]
mod output;

#[cfg(feature = "output")]
use crate::Frame;
use cpal::{
    traits::DeviceTrait, BuildStreamError, DefaultStreamConfigError, Device, PlayStreamError,
    SupportedStreamConfig, SupportedStreamConfigsError,
};
#[cfg(feature = "input")]
pub use input::*;
#[cfg(feature = "output")]
pub use output::*;

pub use cpal;

/// A error encountered when trying to build a [`SystemAudio`]
#[derive(Debug, thiserror::Error)]
pub enum BuildSystemAudioError {
    /// An error building the audio stream
    #[error("{0}")]
    Stream(#[from] BuildStreamError),
    /// An error starting the audio stream
    #[error("{0}")]
    Play(#[from] PlayStreamError),
    /// An error querying stream configurations
    #[error("{0}")]
    SupportedConfigs(#[from] SupportedStreamConfigsError),
    /// An error getting a default stream configuration
    #[error("{0}")]
    DefaultConfig(#[from] DefaultStreamConfigError),
    /// No default input device is available
    #[error("No device available")]
    NoDevice,
}

/// A result type for trying to build a [`SystemAudio`]
pub type BuildSystemAudioResult<T> = Result<T, BuildSystemAudioError>;

/**
A builder for creating [`InputDeviceSource`]s and [`OutputDeviceMixer`]s
*/
#[derive(Default)]
pub struct DeviceIoBuilder {
    /// The device to use. If not set, the default device will be used.
    pub device: Option<Device>,
    /// The stream configuration to be used. If not set, the default will be used.
    pub config: Option<SupportedStreamConfig>,
}

impl DeviceIoBuilder {
    /// Initialize a builder with the default input device and stream configuration
    #[cfg(feature = "input")]
    pub fn default_input() -> Self {
        let device = default_input_device();
        let config = device
            .as_ref()
            .and_then(|device| device.default_input_config().ok());
        DeviceIoBuilder { device, config }
    }
    /// Initialize a builder with the default output device and stream configuration
    #[cfg(feature = "output")]
    pub fn default_output() -> Self {
        let device = default_output_device();
        let config = device
            .as_ref()
            .and_then(|device| device.default_output_config().ok());
        DeviceIoBuilder { device, config }
    }
    /// Set the input device
    pub fn device(self, device: Device) -> Self {
        DeviceIoBuilder {
            device: Some(device),
            ..self
        }
    }
    /// Set the stream configuration
    pub fn config(self, config: SupportedStreamConfig) -> Self {
        DeviceIoBuilder {
            config: Some(config),
            ..self
        }
    }
    /// Build an [`InputDeviceSource`]
    #[cfg(feature = "input")]
    pub fn build_input(self) -> BuildSystemAudioResult<InputDeviceSource> {
        InputDeviceSource::from_builder(self)
    }
    /// Build an [`OutputDeviceMixer`]
    #[cfg(feature = "output")]
    pub fn build_output<F>(self) -> BuildSystemAudioResult<OutputDeviceMixer<F>>
    where
        F: Frame + Send + 'static,
    {
        OutputDeviceMixer::from_builder(self)
    }
}
