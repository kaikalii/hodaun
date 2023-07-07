use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    // Add a 2 square wave with a simple envelope
    output.add(
        SquareWave::new(220)
            .amplify(0.6)
            .ads(AdsEnvelope::new(0.05, 0.1, 0.5))
            .take_release(1, 0.2)
            .repeat(4),
    );

    // Let it play
    output.block();
}
