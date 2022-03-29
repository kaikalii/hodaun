use std::{thread::sleep, time::Duration};

use hodaun::{
    gen::{Sine, Wave},
    DeviceMixer, Mono, Source,
};

fn main() {
    let mut mixer = DeviceMixer::<Mono>::with_default_device().unwrap();

    mixer.add(Wave::<Sine>::new(220.0, 44100.0).amplify(0.5));
    mixer.add(Wave::<Sine>::new(220.0 * 2f32.powf(4.0 / 12.0), 44100.0).amplify(0.5));
    mixer.add(Wave::<Sine>::new(220.0 * 2f32.powf(7.0 / 12.0), 44100.0).amplify(0.5));
    mixer.play().unwrap();

    loop {
        sleep(Duration::from_secs(1))
    }
}
