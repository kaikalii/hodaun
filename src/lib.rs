#![warn(missing_docs)]

/*!
This crate provides interfaces for audio input and output, as well as mixing and signal processing.
*/

mod frame;
pub mod gen;
#[cfg(any(feature = "input", feature = "output"))]
mod io;
#[cfg(feature = "notes")]
mod mixer;
mod note;
pub mod source;
#[cfg(feature = "wav")]
#[cfg_attr(docsrs, doc(cfg(feature = "wav")))]
pub mod wav;

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
    fn from_f32(f: f32) -> Self;
}

impl Amplitude for f32 {
    const MIDPOINT: Self = 0.0;
    fn from_f32(f: f32) -> Self {
        f
    }
}

impl Amplitude for u16 {
    const MIDPOINT: Self = u16::MAX / 2;
    fn from_f32(f: f32) -> Self {
        const HALF_U16_MAX: f32 = u16::MAX as f32 * 0.5;
        (f * HALF_U16_MAX + HALF_U16_MAX) as u16
    }
}

impl Amplitude for i16 {
    const MIDPOINT: Self = 0;
    fn from_f32(f: f32) -> Self {
        const I16_MAX: f32 = i16::MAX as f32;
        (f * I16_MAX) as i16
    }
}

/// Linearly interpolate two numbers
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + t * b
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

impl ToDuration for f32 {
    fn to_duration(self) -> Duration {
        Duration::from_secs_f32(self)
    }
}

/// Trait for automating source control value
pub trait Automation {
    /// Get the next value
    fn next_value(&mut self, sample_rate: f32) -> Option<f32>;
}

impl Automation for f32 {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(*self)
    }
}

impl Automation for u64 {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(*self as f32)
    }
}

impl Automation for Shared<f32> {
    #[inline(always)]
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(self.get())
    }
}

impl<S> Automation for S
where
    S: Source<Frame = f32>,
{
    fn next_value(&mut self, sample_rate: f32) -> Option<f32> {
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
    pub fn set(&self, val: T) {
        *self.0.lock() = val;
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
