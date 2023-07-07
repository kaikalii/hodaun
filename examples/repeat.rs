use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    let wave = TriangleWave::new(55).take_release(0.5, 0.5);
    output.add(wave.repeat(4).every(1));

    // Let it play
    output.block();
}
