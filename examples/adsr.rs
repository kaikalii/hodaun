use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();

    // Add a 2 square sine wave with a simple envelope
    output.add(
        SquareWave::new(261.63)
            .ads(AdsEnvelope::new(0.05, 0.1, 0.5))
            .take_release(2, 0.2),
    );

    // Play
    output.play_blocking().unwrap();
}
