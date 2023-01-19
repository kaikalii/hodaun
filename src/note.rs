#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Automation;

/// Type alias for an octave
pub type Octave = i8;

/// The twelve notes of the western chromatic scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[allow(missing_docs)]
pub enum Letter {
    C,
    Db,
    D,
    Eb,
    E,
    F,
    Gb,
    G,
    Ab,
    A,
    Bb,
    B,
}

impl Letter {
    /// Make a pitch with this letter and the given octave.
    pub const fn oct(self, octave: Octave) -> Pitch {
        Pitch::new(self, octave)
    }
    /// Get the frequency of this letter in the given octave.
    pub fn frequency(self, octave: Octave) -> f32 {
        440.0 * 2f32.powf(((octave - 4) * 12 + (self as i8 - 9)) as f32 / 12.0)
    }
    /// Get the number of half-steps above C0
    pub const fn half_steps(self, octave: Octave) -> i16 {
        (octave as i16 * 12) + (self as i16)
    }
    #[allow(missing_docs, non_upper_case_globals)]
    pub const Csh: Self = Self::Db;
    #[allow(missing_docs, non_upper_case_globals)]
    pub const Dsh: Self = Self::Eb;
    #[allow(missing_docs, non_upper_case_globals)]
    pub const Fsh: Self = Self::Gb;
    #[allow(missing_docs, non_upper_case_globals)]
    pub const Gsh: Self = Self::Ab;
    #[allow(missing_docs, non_upper_case_globals)]
    pub const Ash: Self = Self::Bb;
}

/// A letter-octave pair representing a frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch {
    /// The octave of the pitch
    pub octave: Octave,
    /// The letter of the pitch
    pub letter: Letter,
}

impl Pitch {
    /// Make a new pitch with the given letter and octave
    pub const fn new(letter: Letter, octave: Octave) -> Self {
        Self { letter, octave }
    }
    /// Get the frequency of this pitch
    pub fn frequency(&self) -> f32 {
        self.letter.frequency(self.octave)
    }
    /// Make a pitch from some snumber of half-steps above C0
    pub const fn from_half_steps(half_steps: i16) -> Self {
        let octave = (half_steps / 12) as i8;
        let letter = match half_steps % 12 {
            0 => Letter::C,
            1 => Letter::Db,
            2 => Letter::D,
            3 => Letter::Eb,
            4 => Letter::E,
            5 => Letter::F,
            6 => Letter::Gb,
            7 => Letter::G,
            8 => Letter::Ab,
            9 => Letter::A,
            10 => Letter::Bb,
            11 => Letter::B,
            _ => unreachable!(),
        };
        Self { letter, octave }
    }
    /// Get the number of half-steps above C0
    pub const fn to_half_steps(&self) -> i16 {
        self.letter.half_steps(self.octave)
    }
}

impl Automation for Pitch {
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(self.frequency())
    }
}

impl Automation for (Letter, Octave) {
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(self.0.frequency(self.1))
    }
}

impl From<(Letter, Octave)> for Pitch {
    fn from((letter, octave): (Letter, Octave)) -> Self {
        Self { letter, octave }
    }
}

impl From<i16> for Pitch {
    fn from(half_steps: i16) -> Self {
        Self::from_half_steps(half_steps)
    }
}

impl PartialEq<(Letter, Octave)> for Pitch {
    fn eq(&self, other: &(Letter, Octave)) -> bool {
        self.letter == other.0 && self.octave == other.1
    }
}

impl PartialEq<Pitch> for (Letter, Octave) {
    fn eq(&self, other: &Pitch) -> bool {
        self.0 == other.letter && self.1 == other.octave
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Letter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "c" => Ok(Letter::C),
            "db" | "c#" | "csh" => Ok(Letter::Db),
            "d" => Ok(Letter::D),
            "eb" | "d#" | "dsh" => Ok(Letter::Eb),
            "e" => Ok(Letter::E),
            "f" => Ok(Letter::F),
            "gb" | "f#" | "fsh" => Ok(Letter::Gb),
            "g" => Ok(Letter::G),
            "ab" | "g#" | "gsh" => Ok(Letter::Ab),
            "a" => Ok(Letter::A),
            "bb" | "a#" | "ash" => Ok(Letter::Bb),
            "b" => Ok(Letter::B),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid note letter: {s:?}"
            ))),
        }
    }
}

#[cfg(feature = "serde")]
mod pitch_ser {
    use super::*;
    impl Serialize for Pitch {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            (self.letter, self.octave).serialize(serializer)
        }
    }
    impl<'de> Deserialize<'de> for Pitch {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let (letter, octave) = Deserialize::deserialize(deserializer)?;
            Ok(Self { letter, octave })
        }
    }
}

/// Musical modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Mode {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
    HarmonicMinor,
    MelodicMinor,
    WholeTone,
    Diminished,
}

const MAJOR_SCALE: [i16; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR_SCALE: [i16; 7] = [0, 2, 3, 5, 7, 8, 10];
const DORIAN_SCALE: [i16; 7] = [0, 2, 3, 5, 7, 9, 10];
const PHRYGIAN_SCALE: [i16; 7] = [0, 1, 3, 5, 7, 8, 10];
const LYDIAN_SCALE: [i16; 7] = [0, 2, 4, 6, 7, 9, 11];
const MIXOLYDIAN_SCALE: [i16; 7] = [0, 2, 4, 5, 7, 9, 10];
const AEOLIAN_SCALE: [i16; 7] = [0, 2, 3, 5, 7, 8, 10];
const LOCRIAN_SCALE: [i16; 7] = [0, 1, 3, 5, 6, 8, 10];
const HARMONIC_MINOR_SCALE: [i16; 7] = [0, 2, 3, 5, 7, 8, 11];
const MELODIC_MINOR_SCALE: [i16; 7] = [0, 2, 3, 5, 7, 9, 11];
const WHOLE_TONE_SCALE: [i16; 7] = [0, 2, 4, 6, 8, 10, 12];
const DIMINISHED_SCALE: [i16; 7] = [0, 2, 3, 5, 6, 8, 9];

impl Mode {
    /// Get the scale of this mode
    pub fn scale(&self) -> [i16; 7] {
        match self {
            Mode::Major => MAJOR_SCALE,
            Mode::Minor => MINOR_SCALE,
            Mode::Dorian => DORIAN_SCALE,
            Mode::Phrygian => PHRYGIAN_SCALE,
            Mode::Lydian => LYDIAN_SCALE,
            Mode::Mixolydian => MIXOLYDIAN_SCALE,
            Mode::Aeolian => AEOLIAN_SCALE,
            Mode::Locrian => LOCRIAN_SCALE,
            Mode::HarmonicMinor => HARMONIC_MINOR_SCALE,
            Mode::MelodicMinor => MELODIC_MINOR_SCALE,
            Mode::WholeTone => WHOLE_TONE_SCALE,
            Mode::Diminished => DIMINISHED_SCALE,
        }
    }
    /// Get the pitch at the given scale-steps from the base pitch
    ///
    /// `steps` is *not* a number of half-steps, but a number of scale-steps.
    /// There are 7 scale-steps in an octave.
    ///
    /// # Example
    /// ```
    /// use hodaun::*;
    /// use Letter::*;
    ///
    /// let base = (C, 3);
    /// let major_third = Mode::Major.note(base, 2);
    /// assert_eq!(major_third, (E, 3));
    /// let fifth = Mode::Major.note(base, 4);
    /// assert_eq!(fifth, (G, 3));
    /// ```
    pub fn note(&self, base: impl Into<Pitch>, steps: i16) -> Pitch {
        let i = base.into().to_half_steps() * 7 / 12 + steps;
        let octave = (i / 7) as i16;
        let half_step = self.scale()[(i % 7) as usize];
        let half_steps = (octave * 12) + (half_step);
        Pitch::from_half_steps(half_steps)
    }
    /// Round the given pitch to the nearest note in this mode
    ///
    /// # Example
    /// ```
    /// use hodaun::*;
    /// use Letter::*;
    ///
    /// let base = (C, 3);
    /// assert_eq!((C, 3), Mode::Major.round(base, (C, 3)));
    /// assert_eq!((D, 3), Mode::Major.round(base, (Db, 3)));
    /// assert_eq!((D, 3), Mode::Major.round(base, (D, 3)));
    /// assert_eq!((E, 3), Mode::Major.round(base, (Eb, 3)));
    /// assert_eq!((F, 3), Mode::Major.round(base, (F, 3)));
    /// assert_eq!((G, 3), Mode::Major.round(base, (Gb, 3)));
    /// assert_eq!((G, 3), Mode::Major.round(base, (G, 3)));
    /// assert_eq!((A, 3), Mode::Major.round(base, (Ab, 3)));
    /// assert_eq!((A, 3), Mode::Major.round(base, (A, 3)));
    /// assert_eq!((B, 3), Mode::Major.round(base, (Bb, 3)));
    /// assert_eq!((C, 4), Mode::Major.round(base, (C, 4)));
    /// ```
    pub fn round(&self, base: impl Into<Pitch>, pitch: impl Into<Pitch>) -> Pitch {
        let base = base.into();
        let pitch = pitch.into();
        let steps =
            ((pitch.to_half_steps() - base.to_half_steps()) as f32 / 12.0 * 7.0).round() as i16;
        self.note(base, steps)
    }
}
