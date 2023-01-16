use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();
    let sample_rate = output.sample_rate();

    const FREQ: f32 = 261.63;
    const DUR: u64 = 2;

    // Add waves
    // Even though all these sources are given different amplitudes,
    // the normalization will bring them all to 0.5
    let a = SineWave::new(FREQ, sample_rate).amplify(0.5).take(DUR);
    let b = SineWave::new(FREQ, sample_rate).amplify(0.1).take(DUR);
    let c = SineWave::new(FREQ, sample_rate).amplify(0.9).take(DUR);
    let d = SineWave::new(FREQ, sample_rate).amplify(0.3).take(DUR);
    output.add(a.chain(b).chain(c).chain(d).normalize(0.5, 0.005));

    // Play
    output.play_blocking().unwrap();
}
