#![warn(missing_docs)]
#![allow(clippy::needless_range_loop)]

/*!
[<img alt="github" src="https://img.shields.io/badge/GitHub-kaikalii%2Fhodaun-8da0cb?logo=github">](https://github.com/kaikalii/hodaun)
[<img alt="crates.io" src="https://img.shields.io/badge/crates.io-hodaun-orange?logo=rust">](https://crates.io/crates/hodaun)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-hodaun-blue?logo=docs.rs">](https://docs.rs/hodaun)

This crate provides interfaces for audio synthesis, mixing, input, and output.

# Usage

## Audio Sources

The [`Source`] trait generalizes streamed audio data. The associated type [`Source::Frame`]
implements the [`Frame`] trait, and represents a single sample of audio data for multiple channels.

[`Source`] has many utility functions, much like [`Iterator`], for processing and combining audio data.

## Automation

Many [`Source`] functions take parameters that can be automated, meaning they may change either automatically
over time or manually by some other code. The [`Automation`] trait is for any value which can be used
as an automation parameter.

The primary [`Automation`] implementors are:
- [`f64`], `(`[`Letter`]`,`[`Octave`]`)`, and [`Pitch`] for constant values
- [`Shared`]`<A: `[`Automation`]`>` for values that can be changed by other code
- [`Source`]`<Frame = f64>` for values that change over time

## Mixing

[`Mixer`] is a [`Source`] that allows simple audio mixing.

Sources can be added to a [`Mixer`] with [`Mixer::add`].

## Synthesis

The [`gen`] module provides a functions for generating audio data.

[`Wave`] is a source that generates a wave corresponding to a [`Waveform`].

There are helpful type aliases for common waveforms such as [`SineWave`] and [`SquareWave`].

[`Noise`] is a source that generates white noise. It requires the `noise` feature.

## Output

[`OutputDeviceMixer`] allows the mixing of audio [`Source`]s and output to an audio device.
An [`OutputDeviceMixer`] for the default output device can be created with [`default_output`].
For more nuanced control, use [`DeviceIoBuilder::build_output`].

Output functionality is only available when the `output` feature is enabled.

## Input

[`InputDeviceSource`] is a [`Source`] interface for an audio input device.
An [`InputDeviceSource`] for the default input device can be created with
[`default_input`].
For more nuanced control, use [`DeviceIoBuilder::build_input`].

Input functionality is only available when the `input` feature is enabled.

## Audio Files

The [`wav`] module provides [`wav::WavSource`] for reading WAV files and
[`wav::write_source`] for writing WAV files.

WAV functionality is only available when the `wav` feature is enabled.

## Musical Notes

A [`Letter`] is a note in the western chromatic scale, such as `A` or `C#`.

When combined with an [`Octave`], a [`Letter`] can be converted to a [`Pitch`].

[`Pitch`] supports querying for frequency and number of half-steps.
It also implements [`Automation`].

[`Mode`] is a musical mode, such as major or minor.
It can be used to choose notes from a scale.

Musical note functionality is only available when the `notes` feature is enabled.

## A note on sample types

While this library can handle audio input and output streams that work with various sample types,
the library itself works with [`f64`] samples only, converting when necessary.

There are two reasons for this:
- Floating point is more natural to work with, as we often conceive of amplitude as a non-discrete value.
- [`f64`] has higher precision than [`f32`], which is important for this library's audio synthesis algorithms.
*/

#[cfg(any(feature = "wav", feature = "ogg"))]
mod codec;
mod frame;
pub mod gen;
#[cfg(any(feature = "input", feature = "output"))]
mod io;
#[cfg(feature = "notes")]
mod mixer;
mod note;
pub mod source;

#[cfg(any(feature = "wav", feature = "ogg"))]
pub use codec::*;
#[cfg(any(feature = "input", feature = "output"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "input", feature = "output"))))]
pub use io::*;
#[cfg(feature = "notes")]
pub use note::*;
#[doc(inline)]
pub use source::{AdsEnvelope, Constant, Maintainer, Source, UnrolledSource};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};
pub use {frame::*, gen::*, mixer::*};

use parking_lot::Mutex;

trait Amplitude: Clone + std::ops::AddAssign<Self> {
    const MIDPOINT: Self;
    fn from_f64(f: f64) -> Self;
}

impl Amplitude for f32 {
    const MIDPOINT: Self = 0.0;
    fn from_f64(f: f64) -> Self {
        f as f32
    }
}

impl Amplitude for u16 {
    const MIDPOINT: Self = u16::MAX / 2;
    fn from_f64(f: f64) -> Self {
        const HALF_U16_MAX: f64 = u16::MAX as f64 * 0.5;
        (f * HALF_U16_MAX + HALF_U16_MAX) as u16
    }
}

impl Amplitude for i16 {
    const MIDPOINT: Self = 0;
    fn from_f64(f: f64) -> Self {
        const I16_MAX: f64 = i16::MAX as f64;
        (f * I16_MAX) as i16
    }
}

impl Amplitude for u8 {
    const MIDPOINT: Self = u8::MAX / 2;
    fn from_f64(f: f64) -> Self {
        const HALF_U8_MAX: f64 = u8::MAX as f64 * 0.5;
        (f * HALF_U8_MAX + HALF_U8_MAX) as u8
    }
}

impl Amplitude for i8 {
    const MIDPOINT: Self = 0;
    fn from_f64(f: f64) -> Self {
        const I8_MAX: f64 = i8::MAX as f64;
        (f * I8_MAX) as i8
    }
}

impl Amplitude for u32 {
    const MIDPOINT: Self = u32::MAX / 2;
    fn from_f64(f: f64) -> Self {
        const HALF_U32_MAX: f64 = u32::MAX as f64 * 0.5;
        f.mul_add(HALF_U32_MAX, HALF_U32_MAX) as u32
    }
}

impl Amplitude for i32 {
    const MIDPOINT: Self = 0;
    fn from_f64(f: f64) -> Self {
        const I32_MAX: f64 = i32::MAX as f64;
        (f * I32_MAX) as i32
    }
}

impl Amplitude for u64 {
    const MIDPOINT: Self = u64::MAX / 2;
    fn from_f64(f: f64) -> Self {
        const HALF_U64_MAX: f64 = u64::MAX as f64 * 0.5;
        f.mul_add(HALF_U64_MAX, HALF_U64_MAX) as u64
    }
}

impl Amplitude for i64 {
    const MIDPOINT: Self = 0;
    fn from_f64(f: f64) -> Self {
        const I64_MAX: f64 = i64::MAX as f64;
        (f * I64_MAX) as i64
    }
}

impl Amplitude for f64 {
    const MIDPOINT: Self = 0.0;
    fn from_f64(f: f64) -> Self {
        f
    }
}

/// Linearly interpolate two numbers
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (b - a).mul_add(t, a)
}

/// A trait for converting to a [`Duration`]
pub trait ToDuration {
    /// Convert to a duration
    fn to_duration(self) -> Duration;
}

impl ToDuration for Duration {
    fn to_duration(self) -> Duration {
        self
    }
}

impl ToDuration for u64 {
    fn to_duration(self) -> Duration {
        Duration::from_secs(self)
    }
}

impl ToDuration for f64 {
    fn to_duration(self) -> Duration {
        Duration::from_secs_f64(self)
    }
}

/// Trait for automating source control value
pub trait Automation {
    /// Get the next value
    fn next_value(&mut self, sample_rate: f64) -> Option<f64>;
}

impl Automation for f32 {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f64) -> Option<f64> {
        Some(*self as f64)
    }
}

impl Automation for f64 {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f64) -> Option<f64> {
        Some(*self)
    }
}

impl Automation for u64 {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f64) -> Option<f64> {
        Some(*self as f64)
    }
}

impl<A> Automation for Shared<A>
where
    A: Automation,
{
    #[inline(always)]
    fn next_value(&mut self, sample_rate: f64) -> Option<f64> {
        self.with(|auto| auto.next_value(sample_rate))
    }
}

impl<S> Automation for S
where
    S: Source<Frame = f64>,
{
    fn next_value(&mut self, sample_rate: f64) -> Option<f64> {
        Source::next(self, sample_rate)
    }
}

/// A thread-safe, reference-counted, locked wrapper
///
/// This is mostly used to allow audio source parameters
/// to be changed while the source is playing.
#[derive(Default)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Shared<T> {
    /// Create a new shared
    pub fn new(val: T) -> Self {
        Shared(Arc::new(Mutex::new(val)))
    }
    /// Set the value
    pub fn set(&mut self, val: T) {
        *self.0.lock() = val;
    }
    /// Modify the value
    pub fn with<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut *self.0.lock())
    }
}

impl<T> Shared<T>
where
    T: Copy,
{
    /// Copy the value out
    pub fn get(&self) -> T {
        *self.0.lock()
    }
}

impl<T> Shared<T>
where
    T: Clone,
{
    /// Clone the value out
    pub fn cloned(&self) -> T {
        self.0.lock().clone()
    }
}

impl<T> PartialEq for Shared<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        *self.0.lock() == *other.0.lock()
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Arc::clone(&self.0))
    }
}

impl<T> Eq for Shared<T> where T: Eq {}

impl<T> PartialOrd for Shared<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.lock().partial_cmp(&*other.0.lock())
    }
}

impl<T> Ord for Shared<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.lock().cmp(&*other.0.lock())
    }
}

impl<T> Hash for Shared<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.lock().hash(state);
    }
}

impl<T> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().fmt(f)
    }
}

impl<T> fmt::Display for Shared<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().fmt(f)
    }
}

impl<'a, T> From<&'a Shared<T>> for Shared<T> {
    fn from(shared: &'a Shared<T>) -> Self {
        (*shared).clone()
    }
}

impl<T> From<T> for Shared<T> {
    fn from(val: T) -> Self {
        Shared::new(val)
    }
}

impl<'a, T> From<&'a T> for Shared<T>
where
    T: Clone,
{
    fn from(val: &'a T) -> Self {
        Shared::new(val.clone())
    }
}
