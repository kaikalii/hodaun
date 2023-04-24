use std::{fmt, mem::transmute, ops::*};

/// Mono [`Frame`] type
pub type Mono = f64;

/// Convert a [`Frame`] to mono
pub fn mono(frame: impl AsRef<[f64]>) -> Mono {
    frame.as_ref().iter().sum::<f64>() / frame.as_ref().len() as f64
}

/// Convert a mono [`Frame`] to stereo
pub fn stereo(frame: Mono) -> Stereo {
    Stereo::both(frame)
}

/// A single multi-channel frame in an audio source
pub trait Frame: Clone {
    /// The number of audio channels
    const CHANNELS: usize;
    /// Create a frame with a uniform amplitude across all channels
    fn uniform(amplitude: f64) -> Self;
    /// Get the amplitude of a channel
    fn get_channel(&self, index: usize) -> f64;
    /// Set the amplitude of a channel
    fn set_channel(&mut self, index: usize, amplitude: f64);
    /// Apply a function to each channel
    fn map(self, f: impl Fn(f64) -> f64) -> Self;
    /// Combine two frames by applying a function
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64);
    /// Get the average amplitude
    fn avg(&self) -> f64 {
        (0..Self::CHANNELS)
            .map(|i| self.get_channel(i))
            .sum::<f64>()
            / Self::CHANNELS as f64
    }
    /// Add two frames
    fn add(mut self, other: Self) -> Self {
        self.merge(other, Add::add);
        self
    }
    /// Write the frame to a channel slice
    ///
    /// The channel counts of the frame and slice need not match.
    fn write_slice(self, slice: &mut [f64]) {
        match (Self::CHANNELS, slice.len()) {
            (1, _) => slice.fill(self.get_channel(0)),
            (_, 1) => slice[0] = self.avg(),
            (a, b) => {
                for i in 0..a.min(b) {
                    slice[i] = self.get_channel(i);
                }
            }
        }
    }
}

impl Frame for f64 {
    const CHANNELS: usize = 1;
    fn uniform(amplitude: f64) -> Self {
        amplitude
    }
    fn get_channel(&self, _index: usize) -> f64 {
        *self
    }
    fn set_channel(&mut self, _index: usize, amplitude: f64) {
        *self = amplitude;
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        f(self)
    }
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64) {
        *self = f(*self, other);
    }
    fn avg(&self) -> f64 {
        *self
    }
    fn add(self, other: Self) -> Self {
        self + other
    }
}

impl<const N: usize> Frame for [f64; N]
where
    Self: Default,
{
    const CHANNELS: usize = N;
    fn uniform(amplitude: f64) -> Self {
        [amplitude; N]
    }
    fn get_channel(&self, index: usize) -> f64 {
        self[index]
    }
    fn set_channel(&mut self, index: usize, amplitude: f64) {
        self[index] = amplitude;
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        self.map(f)
    }
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64) {
        for (a, b) in self.iter_mut().zip(other) {
            *a = f(*a, b);
        }
    }
}

/// Stereo [`Frame`] type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Stereo<T = f64> {
    /// The left channel
    pub left: T,
    /// The right channel
    pub right: T,
}

impl<T> Stereo<T> {
    #[inline]
    /// Create a new stereo frame
    pub const fn new(left: T, right: T) -> Self {
        Self { left, right }
    }
    /// Map the channels of the frame
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Stereo<U> {
        Stereo::new(f(self.left), f(self.right))
    }
    /// Combine two frames by applying a function
    pub fn with<U, V>(self, other: Stereo<U>, mut f: impl FnMut(T, U) -> V) -> Stereo<V> {
        Stereo::new(f(self.left, other.left), f(self.right, other.right))
    }
    /// Combine two frames
    pub fn zip<U>(self, other: Stereo<U>) -> Stereo<(T, U)> {
        self.with(other, |a, b| (a, b))
    }
    /// Call a function on the channels of the frame
    pub fn reduce(self, mut f: impl FnMut(T, T) -> T) -> T {
        f(self.left, self.right)
    }
}

impl<T: fmt::Display + PartialEq + Default> fmt::Display for Stereo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.left == self.right {
            write!(f, "{}", self.left)
        } else {
            write!(f, "({} {})", self.left, self.right)
        }
    }
}

impl<T: Clone> Stereo<T> {
    #[inline]
    /// Create a new stereo frame with the same value for both channels
    pub fn both(v: T) -> Self {
        Self {
            left: v.clone(),
            right: v,
        }
    }
}

impl Stereo {
    /// `[0.0, 0.0]`
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// `[1.0, 0.0]`
    pub const LEFT: Self = Self::new(1.0, 0.0);
    /// `[0.0, 1.0]`
    pub const RIGHT: Self = Self::new(0.0, 1.0);
    /// Get the average of the channels
    pub fn average(self) -> f64 {
        (self.left + self.right) / 2.0
    }
}

macro_rules! bin_op {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident) => {
        impl $trait for Stereo {
            type Output = Self;
            fn $method(mut self, other: Self) -> Self {
                self.$assign_method(other);
                self
            }
        }

        impl $assign_trait for Stereo {
            fn $assign_method(&mut self, other: Self) {
                self.left.$assign_method(other.left);
                self.right.$assign_method(other.right);
            }
        }

        impl $trait<f64> for Stereo {
            type Output = Self;
            fn $method(mut self, other: f64) -> Self {
                self.$assign_method(Self::both(other));
                self
            }
        }

        impl $trait<Stereo> for f64 {
            type Output = Stereo;
            fn $method(self, other: Stereo) -> Stereo {
                other.map(|v| $trait::$method(self, v))
            }
        }

        impl $assign_trait<f64> for Stereo {
            fn $assign_method(&mut self, other: f64) {
                self.$assign_method(Self::both(other));
            }
        }
    };
}

bin_op!(Add, add, AddAssign, add_assign);
bin_op!(Sub, sub, SubAssign, sub_assign);
bin_op!(Mul, mul, MulAssign, mul_assign);
bin_op!(Div, div, DivAssign, div_assign);
bin_op!(Rem, rem, RemAssign, rem_assign);

impl Neg for Stereo {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.left, -self.right)
    }
}

impl Frame for Stereo {
    const CHANNELS: usize = 2;
    fn uniform(amplitude: f64) -> Self {
        Self::both(amplitude)
    }
    fn get_channel(&self, index: usize) -> f64 {
        unsafe { transmute::<&Self, &[f64; 2]>(self)[index] }
    }
    fn set_channel(&mut self, index: usize, amplitude: f64) {
        unsafe { transmute::<&mut Self, &mut [f64; 2]>(self)[index] = amplitude }
    }
    fn map(self, f: impl Fn(f64) -> f64) -> Self {
        Self::map(self, f)
    }
    fn merge(&mut self, other: Self, f: impl Fn(f64, f64) -> f64) {
        *self = self.with(other, f);
    }
}
