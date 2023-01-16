use hodaun::*;

fn main() {
    let input = InputDeviceSource::with_default_device().unwrap();
    println!("{} input channel(s)", input.channels());

    let mut output = OutputDeviceMixer::<Stereo>::with_default_device().unwrap();

    println!(
        "sample rates: {} -> {}",
        input.sample_rate(),
        output.sample_rate()
    );

    output.add(input.resample());

    output.play_blocking().unwrap();
}
