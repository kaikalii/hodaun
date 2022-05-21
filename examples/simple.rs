use std::time::Duration;

use hodaun::{gen::SineWave, MixerInterface, Mono, OutputDeviceMixer, Source};

fn main() {
    // Initializer the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.sample_rate();

    // Add a 2 second sine wave
    output.add(
        SineWave::new(261.63, sample_rate)
            .amplify(0.5)
            .take(Duration::from_secs(2)),
    );

    // Play
    output.play_blocking().unwrap();
}
