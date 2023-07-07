use std::fs::File;

use hodaun::*;

fn main() {
    let mut output = default_output().unwrap();

    // Write to a WAV file
    let source = SineWave::new(Letter::C.oct(4)).amplify(0.5).take(2);
    let file = File::create("example.wav").unwrap();
    wav::write_source(file, source, output.sample_rate() as u32).unwrap();

    // Read from a WAV file
    let file = File::open("example.wav").unwrap();
    let source = wav::WavSource::new(file).unwrap().resample::<Mono>();
    output.add(source);

    output.block();
}
