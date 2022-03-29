use std::time::Duration;

use hodaun::{
    gen::{Sine, Wave},
    DeviceMixer, Mixer, Mono, Source,
};

const SAMPLE_RATE: f32 = 44100.0;

fn main() {
    let mut output = DeviceMixer::<Mono>::with_default_device().unwrap();
    let (mixer, mixer_source) = Mixer::new();

    mixer.add(Wave::<Sine>::new(220.0, SAMPLE_RATE).amplify(0.5));
    mixer.add(Wave::<Sine>::new(220.0 * 2f32.powf(4.0 / 12.0), SAMPLE_RATE).amplify(0.5));
    mixer.add(Wave::<Sine>::new(220.0 * 2f32.powf(7.0 / 12.0), SAMPLE_RATE).amplify(0.5));

    output.add(mixer_source.take(Duration::from_secs(1)));

    output.blocking_play().unwrap();
}
