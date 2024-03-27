pub mod gol;

use gol::GOL;
use hound;

fn main() {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create("test.wav", spec).unwrap();
    let mut gol = GOL::new(64, 69);
    for i in 0..(44100 / 32) {
        let ir = gol.advance();

        for mut sample in ir {
            sample *= 1 as f32 / (i + 1) as f32;
            writer.write_sample(sample).unwrap();
        }
    }

    writer.finalize().unwrap();
}
