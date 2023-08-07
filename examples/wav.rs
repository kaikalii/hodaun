use std::fs::File;

use hodaun::*;

fn main() {
    let mut output = default_output().unwrap();

    // Write to a WAV file
    let chord = Mixer::new();
    let base = 220.0;
    chord.add(SineWave::new(base));
    chord.add(SineWave::new(base * 2f32.powf(4.0 / 12.0)));
    chord.add(SineWave::new(base * 2f32.powf(7.0 / 12.0)));
    let source = chord.amplify(0.5).take(2);
    let file = File::create("example.wav").unwrap();
    wav::write_source(file, source, output.sample_rate() as u32).unwrap();

    // Read from a WAV file
    let file = File::open("example.wav").unwrap();
    let source = wav::WavSource::new(file).unwrap().resample::<Mono>();
    output.add(source);

    output.block();
}
