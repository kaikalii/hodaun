use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    // Add a 2 second sine wave
    output.add(SineWave::new(Letter::C.oct(4)).amplify(0.5).take(2));

    // Let it play
    output.block();
}
