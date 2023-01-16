use std::fs::File;

use hodaun::*;

fn main() {
    // Write to a WAV file
    let source = SineWave::new(261.63).amplify(0.5).take(2);
    let file = File::create("example.wav").unwrap();
    wav::write_source(file, source, 44100).unwrap();

    // Read from a WAV file
    let file = File::open("example.wav").unwrap();
    let source = wav::WaveSource::new(file).unwrap().resample::<Mono>();
    let mut output = OutputDeviceMixer::with_default_device().unwrap();
    output.add(source);
    output.play_blocking().unwrap();
}
