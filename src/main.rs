use raylib::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Cursor};

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

    let mut w = Cursor::new(vec![]);

    let mut encoder = png::Encoder::new(&mut w, frame_info.width, frame_info.height);
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
    writer.finish().unwrap();

    let (mut rl, thread) = raylib::init()
        .size(frame_info.width as i32, frame_info.height as i32)
        .title("Edging")
        .build();

    let image_bytes = w.into_inner();
    let image = raylib::core::texture::Image::load_image_from_mem(
        ".png",
        &image_bytes,
        grayscale_bytes.len() as i32,
    )
    .unwrap();

    let texture = rl.load_texture_from_image(&thread, &image).unwrap();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::RAYWHITE);
        d.draw_texture(&texture, 0, 0, Color::WHITE);
    }
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
