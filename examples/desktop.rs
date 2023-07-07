use std::{
    fs::File,
    io::{stdin, stdout, BufRead, Write},
};

use hodaun::*;

fn main() {
    print!("Recording 5 seconds of desktop audio output to `example.wav`...");
    stdout().flush().unwrap();

    let source = default_output_as_input()
        .unwrap()
        .resample::<Stereo>()
        .take(5);
    let file = File::create("example.wav").unwrap();
    wav::write_source(file, source, 44100).unwrap();
    println!("Done!");

    print!("Would you like to play it back? (y/n) ");
    stdout().flush().unwrap();

    let response = stdin().lock().lines().next().unwrap().unwrap();
    if response.trim().to_ascii_lowercase() == "y" {
        let file = File::open("example.wav").unwrap();
        let source = wav::WavSource::new(file).unwrap().resample::<Stereo>();
        let mut output = default_output::<Stereo>().unwrap();
        output.add(source);
        output.block();
    }
}
