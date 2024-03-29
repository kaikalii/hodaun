use hodaun::*;

fn main() {
    // Initialize the output and a chord mixer
    let mut output = default_output().unwrap();
    let chord = Mixer::new();

    // Add notes to the chord
    let base = 220.0;
    chord.add(SineWave::new(base));
    chord.add(SineWave::new(base * 2f32.powf(4.0 / 12.0)));
    chord.add(SineWave::new(base * 2f32.powf(7.0 / 12.0)));

    // Add the chord to the output, only playing for 3 seconds
    output.add(chord.amplify(0.5).take(3));

    // Let it play
    output.block();
}
