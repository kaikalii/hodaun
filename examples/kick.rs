use std::f64::consts::TAU;

use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = OutputDeviceMixer::<Mono>::with_default_device().unwrap();

    // Add a 2 second sine wave
    output.add(Kick::default().take(0.5).repeat(8));

    // Play
    output.play_blocking().unwrap();
}

#[derive(Debug, Clone, Copy, Default)]
struct Kick {
    time: f64,
}

impl Source for Kick {
    type Frame = Mono;
    fn next(&mut self, sample_rate: f64) -> Option<Self::Frame> {
        let s =
            (-self.time * 4.0).exp() * (TAU * self.time * 300.0 * (-self.time * 20.0).exp()).sin();
        self.time += 1.0 / sample_rate;
        Some(s)
    }
}
