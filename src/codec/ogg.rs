//! Read ogg files

use std::{
    io::{Read, Seek},
    vec,
};

use lewton::inside_ogg::OggStreamReader;

use crate::{Amplitude, UnrolledSource};

pub use lewton::VorbisError as OggError;

/// A source that reads from a OGG file
pub struct OggSource<R: Read + Seek> {
    sample_rate: u32,
    channels: u8,
    reader: OggStreamReader<R>,
    current_data: vec::IntoIter<i16>,
}

impl<R> OggSource<R>
where
    R: Read + Seek,
{
    /// Create a new OGG source from a reader
    pub fn new(reader: R) -> Result<Self, OggError> {
        let reader = OggStreamReader::new(reader)?;
        Ok(Self {
            sample_rate: reader.ident_hdr.audio_sample_rate,
            channels: reader.ident_hdr.audio_channels,
            reader,
            current_data: vec![].into_iter(),
        })
    }
}

impl<R> Iterator for OggSource<R>
where
    R: Read + Seek,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.current_data
            .next()
            .map(Amplitude::into_f32)
            .or_else(|| {
                let packet = self.reader.read_dec_packet_itl().ok()??;
                self.current_data = packet.into_iter();
                self.current_data.next().map(Amplitude::into_f32)
            })
    }
}

impl<R> UnrolledSource for OggSource<R>
where
    R: Read + Seek,
{
    fn channels(&self) -> usize {
        self.channels as usize
    }
    fn sample_rate(&self) -> f32 {
        self.sample_rate as f32
    }
}
