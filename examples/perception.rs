use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    // Play other waveforms alongside a sinewave to ensure they have the same perceptual loudness
    let sine = SineWave::new(261.63).take(6);
    let square = SquareWave::new(261.63).take(2);
    let saw = SawWave::new(261.63).take(2);
    let triangle = TriangleWave::new(261.63).take(2);
    output.add(
        sine.pan(0.0)
            .zip(square.chain(saw).chain(triangle).pan(1.0), Frame::add)
            .amplify(0.5),
    );

    // Let it play
    output.block();
}
