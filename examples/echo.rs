use hodaun::*;

fn main() {
    let input = default_input().unwrap();
    println!("{} input channel(s)", input.channels());

    let mut output = default_output::<Stereo>().unwrap();

    println!(
        "sample rates: {} -> {}",
        input.sample_rate(),
        output.sample_rate()
    );

    output.add(input.resample());

    output.block();
}
