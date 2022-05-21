use std::time::Duration;

use hodaun::{gen::SineWave, DeviceMixer, MixerInterface, Mono, Source};

fn main() {
    // Initializer the output
    let mut output = DeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.default_sample_rate().unwrap();

    // Add a 2 second sine wave
    output.add(
        SineWave::new(261.63, sample_rate)
            .amplify(0.5)
            .take(Duration::from_secs(2)),
    );

    // Play
    output.blocking_play().unwrap();
}
