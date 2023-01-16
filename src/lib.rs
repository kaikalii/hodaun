#![warn(missing_docs)]

/*!
This crate provides interfaces for audio input and output, as well as mixing and signal processing.
*/

pub mod gen;

#[cfg(any(feature = "input", feature = "output"))]
mod io;
mod mixer;
pub mod source;

#[cfg(any(feature = "input", feature = "output"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "input", feature = "output"))))]
pub use io::*;
pub use mixer::*;
#[doc(inline)]
pub use source::{mono, stereo, Frame, Maintainer, Mono, Silence, Source, Stereo};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

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

#[test]
fn test() {
    use std::{thread::sleep, time::Duration};
    let mut mixer = OutputDeviceMixer::with_default_device().unwrap();
    mixer.add(gen::SineWave::new(220.0, 32000.0).zip(
        gen::Noise::new(32000.0),
        // gen::SineWave::new(277.18, 44100.0),
        // Silence::new(32000.0),
        |[a]: Mono, [b]: Mono| [a, b],
    ));
    // mixer.add(gen::SquareWave::new(440.0, 32000.0));
    mixer.play().unwrap();
    sleep(Duration::from_secs(1));
}

/// Linearly interpolate two numbers
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + t * b
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
        *self.0.lock().unwrap() = val;
    }
}

impl<T> Shared<T>
where
    T: Copy,
{
    /// Copy the value out
    pub fn get(&self) -> T {
        *self.0.lock().unwrap()
    }
}

impl<T> Shared<T>
where
    T: Clone,
{
    /// Clone the value out
    pub fn cloned(&self) -> T {
        self.0.lock().unwrap().clone()
    }
}

impl<T> PartialEq for Shared<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        *self.0.lock().unwrap() == *other.0.lock().unwrap()
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
        self.0
            .lock()
            .unwrap()
            .partial_cmp(&*other.0.lock().unwrap())
    }
}

impl<T> Ord for Shared<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.lock().unwrap().cmp(&*other.0.lock().unwrap())
    }
}

impl<T> Hash for Shared<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.lock().unwrap().hash(state);
    }
}

impl<T> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().unwrap().fmt(f)
    }
}

impl<T> fmt::Display for Shared<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.lock().unwrap().fmt(f)
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
