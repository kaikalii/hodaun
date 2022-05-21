use std::time::Duration;

use hodaun::{gen::SineWave, DeviceMixer, Mixer, MixerInterface, Mono, Source};

fn main() {
    // Initializer the output and a chord mixer
    let mut output = DeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.default_sample_rate().unwrap();
    let (chord, chord_source) = Mixer::new();

    // Add notes to the chord
    let base = 220.0;
    chord.add(SineWave::new(base, sample_rate).amplify(0.5));
    chord.add(SineWave::new(base * 2f32.powf(4.0 / 12.0), sample_rate).amplify(0.5));
    chord.add(SineWave::new(base * 2f32.powf(7.0 / 12.0), sample_rate).amplify(0.5));

    // Add the chord to the output, only playing for 3 seconds
    output.add(chord_source.take(Duration::from_secs(3)));

    // Play
    output.blocking_play().unwrap();
}
