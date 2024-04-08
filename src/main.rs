use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

enum Luminosity {
    Red,
    Green,
    Blue,
}

impl Luminosity {
    fn value(self) -> f32 {
        match self {
            Luminosity::Red => 0.299,
            Luminosity::Green => 0.587,
            Luminosity::Blue => 0.144,
        }
    }
}

fn main() {
    let decoder = png::Decoder::new(File::open("white-square.png").unwrap());
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info().clone();
    let mut buf = vec![0; reader.output_buffer_size()];

    let frame_info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..frame_info.buffer_size()];

    let grayscale_bytes = bytes_to_grayscale(bytes);
    println!(
        "{}x{} origin len: {}; grayscale len: {}",
        frame_info.width,
        frame_info.height,
        bytes.len(),
        grayscale_bytes.len()
    );

    let buf: Vec<u8> = vec![];
    let w = BufWriter::new(buf);

    let mut encoder = png::Encoder::new(w, frame_info.width, frame_info.height);
    encoder.set_color(info.color_type);
    encoder.set_depth(info.bit_depth);
    if let Some(sg) = info.source_gamma {
        encoder.set_source_gamma(sg);
    }
    if let Some(sc) = info.source_chromaticities {
        encoder.set_source_chromaticities(sc);
    }
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(grayscale_bytes.as_slice()).unwrap();
}

fn bytes_to_grayscale(src: &[u8]) -> Vec<u8> {
    let mut dst = vec![0; src.len()];
    let mut i = 0;
    while i < src.len() {
        let lum = (src[i] as f32 * Luminosity::Red.value()
            + src[i + 1] as f32 * Luminosity::Green.value()
            + src[i + 2] as f32 * Luminosity::Blue.value()) as u8;
        dst[i] = lum;
        dst[i + 1] = lum;
        dst[i + 2] = lum;
        dst[i + 3] = src[i + 3];

        i += 4;
    }

    dst
}
