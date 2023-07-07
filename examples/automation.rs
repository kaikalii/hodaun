use hodaun::*;

fn main() {
    // Initialize the output
    let mut output = default_output().unwrap();

    // Build up a wubby wave
    let frequency_automation = Constant(Letter::G.frequency(1))
        .take(4)
        .chain(Constant(Letter::F.frequency(1)).take(4))
        .repeat_indefinitely();
    let base = SquareWave::new(frequency_automation.clone()).zip(
        SawWave::new(frequency_automation.map(|s| s * 2.0)),
        Frame::add,
    );
    let low_pass_automation_automation = Constant(4.0)
        .take(1)
        .chain(Constant(8.0).take(1))
        .repeat_indefinitely();
    let low_pass_automation = SineWave::new(low_pass_automation_automation)
        .positive()
        .amplify(1000.0);
    let wub = base.low_pass(low_pass_automation);
    output.add(wub.take(16));

    // Let it play
    output.block();
}
