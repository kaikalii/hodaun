use std::fs::File;

use hodaun::*;

fn main() {
    // Read from an OGG file
    let file = File::open("examples/crescendo.ogg").unwrap();
    let source = ogg::OggSource::new(file).unwrap().resample::<Mono>();
    let mut output = OutputDeviceMixer::with_default_device().unwrap();
    output.add(source);
    output.play_blocking().unwrap();
}
