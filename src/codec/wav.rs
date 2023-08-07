//! Read and write wave files

use std::io::{Read, Seek, Write};

use hound::{SampleFormat, WavIntoSamples, WavReader, WavSpec, WavWriter};

use crate::{Frame, Source, UnrolledSource};

pub use hound::Error as WaveError;

/// A source that reads from a WAV file
pub struct WavSource<R> {
    sample_rate: u32,
    channels: u16,
    samples: GenericWaveSamples<R>,
}

enum GenericWaveSamples<R> {
    I16(WavIntoSamples<R, i16>),
    I32(WavIntoSamples<R, i32>),
    F32(WavIntoSamples<R, f32>),
}

impl<R> WavSource<R>
where
    R: Read,
{
    /// Create a new WAV source from a reader
    pub fn new(reader: R) -> Result<Self, WaveError> {
        let reader = WavReader::new(reader)?;
        Ok(Self {
            sample_rate: reader.spec().sample_rate,
            channels: reader.spec().channels,
            samples: match reader.spec().sample_format {
                SampleFormat::Int => match reader.spec().bits_per_sample {
                    16 => GenericWaveSamples::I16(reader.into_samples::<i16>()),
                    32 => GenericWaveSamples::I32(reader.into_samples::<i32>()),
                    _ => return Err(WaveError::Unsupported),
                },
                SampleFormat::Float => GenericWaveSamples::F32(reader.into_samples::<f32>()),
            },
        })
    }
}

impl<R> Iterator for WavSource<R>
where
    R: Read,
{
    type Item = f64;
    fn next(&mut self) -> Option<Self::Item> {
        Some(match &mut self.samples {
            GenericWaveSamples::I16(samples) => {
                samples.next()?.unwrap_or_else(|e| panic!("{e}")) as f64 / i16::MAX as f64
            }
            GenericWaveSamples::I32(samples) => {
                samples.next()?.unwrap_or_else(|e| panic!("{e}")) as f64 / i32::MAX as f64
            }
            GenericWaveSamples::F32(samples) => {
                samples.next()?.unwrap_or_else(|e| panic!("{e}")) as f64
            }
        })
    }
}

impl<R> UnrolledSource for WavSource<R>
where
    R: Read,
{
    fn channels(&self) -> usize {
        self.channels as usize
    }
    fn sample_rate(&self) -> f64 {
        self.sample_rate as f64
    }
}

/// Write a source to a WAV file
pub fn write_source<W, S>(writer: W, mut source: S, sample_rate: u32) -> Result<(), WaveError>
where
    W: Write + Seek,
    S: Source,
{
    let spec = WavSpec {
        channels: <S::Frame as Frame>::CHANNELS as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::new(writer, spec)?;
    while let Some(frame) = source.next(sample_rate as f64) {
        for i in 0..<S::Frame as Frame>::CHANNELS {
            writer.write_sample(frame.get_channel(i) as f32)?;
        }
    }
    Ok(())
}
