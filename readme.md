# Hodaun

An audio IO and synthesis library for Rust.

[Documentation](https://docs.rs/hodaun)

## Features

- Audio input and output
- Statically sized sample frames
- Basic waveform generators
- Modular audio processing nodes with the `Source` trait
- Audio automation
- Low-pass filter
- ADSR envelope
- Types for working with musical notes

## Differences from Rodio

While this library was inspired by [Rodio](https://github.com/RustAudio/rodio), it was designed to address some of the pain-points of using Rodio in a real application.

### Statically sized sample frames

Like Rodio, Hodaun has a `Source` trait that abstracts audio streams.
The main thing that makes Hodaun different from Rodio is that Hodaun's `Source`s process audio at the frame level rather than the sample level.
In Rodio, `Source` processes one sample at a time, even if consecutive samples correspond to different channels of the same sample frame.
This makes writing custom `Source`s that are channel-aware difficult.
By contrast, Hodaun's `Source` trait groups samples in the same frame into a statically-sized array (or just a number for mono audio).

### External control and automation

Rodio's `Source` trait lets you do many basic transformations on audio streams, but once the sound is playing, the parameters used for these transformations cannot be changed.
For example `rodio::Source::amplify` takes an `f32` that changes the volume of the `Source`.
But what if you want to change the volume of the `Source` while it's playing? This is common in basically any application that plays audio.
Hodaun also has `Source::amplify`, but it takes a value implementing the `Automation` trait.
This can be a static number just like `Rodio`, but it can also be an externally-controlled value modified elsewhere in your code, or it can even be another `Source` that changes the value over time.