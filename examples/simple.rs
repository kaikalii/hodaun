use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();

    // Add a 2 second sine wave
    output.add(SineWave::new(Letter::C.oct(4)).amplify(0.5).take(2));

    // Play
    output.play_blocking().unwrap();
}
