use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    // Pan a sine wave back and forth at a frequency of 0.5 Hz
    output.add(
        SineWave::new(Letter::C.oct(4))
            .amplify(0.5)
            .pan(SineWave::new(0.5))
            .take(5),
    );

    // Let it play
    output.block();
}
