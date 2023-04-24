use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Stereo>::with_default_device().unwrap();

    // Pan a sine wave back and forth at a frequency of 0.5 Hz
    output.add(
        SineWave::new(Letter::C.oct(4))
            .amplify(0.5)
            .pan(SineWave::new(0.5))
            .take(5),
    );

    // Play
    output.play_blocking().unwrap();
}
