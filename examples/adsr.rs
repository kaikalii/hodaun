use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.sample_rate();

    // Add a 2 second sine wave
    output.add(
        TriangleWave::new(261.63, sample_rate)
            .ads(AdsEnvelope::new(0.05, 0.1, 0.5))
            .take_release(2, 0.2),
    );

    // Play
    output.play_blocking().unwrap();
}
