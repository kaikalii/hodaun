use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();

    let wave = TriangleWave::new(55).take_release(0.5, 0.5);
    output.add(wave.repeat(4).every(1));

    // Play
    output.play_blocking().unwrap();
}
