use crate::Automation;

/// The twelve notes of the western chromatic scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn oct(self, octave: i8) -> Pitch {
        Pitch::new(self, octave)
    }
    /// Get the frequency of this letter in the given octave.
    pub fn frequency(&self, octave: i8) -> f32 {
        440.0 * 2f32.powf(((octave - 4) * 12 + (*self as i8 - 9)) as f32 / 12.0)
    }
}

/// A letter-octave pair representing a frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    /// The letter of the pitch
    pub letter: Letter,
    /// The octave of the pitch
    pub octave: i8,
}

impl Pitch {
    /// Make a new pitch with the given letter and octave
    pub fn new(letter: Letter, octave: i8) -> Self {
        Self { letter, octave }
    }
    /// Get the frequency of this pitch
    pub fn frequency(&self) -> f32 {
        self.letter.frequency(self.octave)
    }
    /// Make a pitch from some snumber of half-steps above C0
    pub fn from_half_steps(half_steps: i8) -> Self {
        let octave = half_steps / 12;
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
    pub fn to_half_steps(&self) -> i8 {
        (self.octave * 12) + (self.letter as i8)
    }
}

impl Automation for Pitch {
    fn next_value(&mut self, _sample_rate: f32) -> Option<f32> {
        Some(self.frequency())
    }
}
