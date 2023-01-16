//! Read and write wave files

use std::io::{Read, Seek, Write};

use hound::{SampleFormat, WavIntoSamples, WavReader, WavSpec, WavWriter};

use crate::{Frame, Source, UnrolledSource};

pub use hound::Error as WaveError;

/// A source that reads from a WAV file
pub struct WaveSource<R> {
    sample_rate: u32,
    channels: u16,
    samples: WavIntoSamples<R, f32>,
}

impl<R> WaveSource<R>
where
    R: Read,
{
    /// Create a new WAV source from a reader
    pub fn new(reader: R) -> Result<Self, WaveError> {
        let reader = WavReader::new(reader)?;
        Ok(Self {
            sample_rate: reader.spec().sample_rate,
            channels: reader.spec().channels,
            samples: reader.into_samples(),
        })
    }
}

impl<R> Iterator for WaveSource<R>
where
    R: Read,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.samples
            .next()
            .map(|s| s.unwrap_or_else(|e| panic!("{}", e)))
    }
}

impl<R> UnrolledSource for WaveSource<R>
where
    R: Read,
{
    fn channels(&self) -> usize {
        self.channels as usize
    }
    fn sample_rate(&self) -> f32 {
        self.sample_rate as f32
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
    while let Some(frame) = source.next(sample_rate as f32) {
        for i in 0..<S::Frame as Frame>::CHANNELS {
            writer.write_sample(frame.get_channel(i))?;
        }
    }
    Ok(())
}
