use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.sample_rate();

    // Add a 2 second sine wave
    output.add(SineWave::new(261.63, sample_rate).amplify(0.5).take(2));

    // Play
    output.play_blocking().unwrap();
}