use hodaun::*;

fn main() {
    // Initialize the output and a chord mixer
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.sample_rate();
    let (chord, chord_source) = Mixer::new();

    // Add notes to the chord
    let base = 220.0;
    chord.add(SineWave::new(base, sample_rate));
    chord.add(SineWave::new(base * 2f32.powf(4.0 / 12.0), sample_rate));
    chord.add(SineWave::new(base * 2f32.powf(7.0 / 12.0), sample_rate));

    // Add the chord to the output, only playing for 3 seconds
    output.add(chord_source.amplify(0.5).take(3));

    // Play
    output.play_blocking().unwrap();
}
