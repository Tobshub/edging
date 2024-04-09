use raylib::prelude::*;
use std::fs::File;
use std::io::Cursor;

fn main() {
    const IMG_PATH: &str = "/home/tobs/Downloads/owl.png";
    let decoder = png::Decoder::new(File::open(IMG_PATH).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info().clone();
    let mut buf = vec![0; reader.output_buffer_size()];

    let frame_info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..frame_info.buffer_size()];

    let bytes = bytes_to_grayscale(bytes);
    let bytes = gaussian_blur(bytes.as_slice(), frame_info.width as usize);
    println!(
        "{}x{}; len: {};",
        frame_info.width,
        frame_info.height,
        bytes.len(),
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

    writer.write_image_data(bytes.as_slice()).unwrap();
    writer.finish().unwrap();

    let (mut rl, thread) = raylib::init().size(800, 680).title("Edging").build();

    let image_bytes = w.into_inner();
    let image =
        raylib::core::texture::Image::load_image_from_mem(".png", &image_bytes, bytes.len() as i32)
            .unwrap();

    let texture = rl.load_texture_from_image(&thread, &image).unwrap();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::RAYWHITE);

        let screen_width = d.get_screen_width();
        let screen_height = d.get_screen_height();
        d.draw_texture(
            &texture,
            screen_width / 2 - image.width / 2,
            screen_height / 2 - image.height / 2,
            Color::WHITE,
        );
    }
}

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

const KERNEL_RADIUS: i32 = 5;
const KERNEL_SIZE: usize = (KERNEL_RADIUS * 2 + 1) as usize;

fn gaussian_blur(src: &[u8], image_width: usize) -> Vec<u8> {
    let mut dst = vec![0; src.len()];
    let mut kernel: [f64; KERNEL_SIZE] = [0.0; KERNEL_SIZE];
    let mut sum = 0.0;

    let sigma: f64 = ((KERNEL_RADIUS / 2) as f64).max(1.0);

    // compute kernal for 1D gaussian blur
    for x in -KERNEL_RADIUS..=KERNEL_RADIUS {
        let exponent = -(x * x) as f64 / (2.0 * sigma * sigma);
        let numerator = std::f64::consts::E.powf(exponent);
        let denominator = 2.0 * std::f64::consts::PI * sigma * sigma;

        let kernal_value = numerator / denominator;
        kernel[(x + KERNEL_RADIUS) as usize] = kernal_value;
        sum += kernal_value;
    }

    // normalize kernel
    (0..KERNEL_SIZE).for_each(|x| {
        kernel[x] /= sum;
    });

    // apply kernel in x direction
    let mut px = 0;
    while px < src.len() {
        let mut new_pixel = [0.0; 3];

        for kernel_x in -KERNEL_RADIUS..=KERNEL_RADIUS {
            let kernal_value = kernel[(kernel_x + KERNEL_RADIUS) as usize];

            let neighbor_px = px as i32 + (kernel_x * 4);

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            new_pixel[0] += src[neighbor_px as usize] as f64 * kernal_value;
            new_pixel[1] += src[neighbor_px as usize + 1] as f64 * kernal_value;
            new_pixel[2] += src[neighbor_px as usize + 2] as f64 * kernal_value;
        }

        dst[px] = new_pixel[0] as u8;
        dst[px + 1] = new_pixel[1] as u8;
        dst[px + 2] = new_pixel[2] as u8;
        dst[px + 3] = src[px + 3];

        px += 4;
    }

    // apply kernel in y direction
    let delta = image_width * 4;
    let mut py = 0;
    while py < dst.len() {
        let mut new_pixel = [0.0; 3];

        for kernel_x in -KERNEL_RADIUS..=KERNEL_RADIUS {
            let kernal_value = kernel[(kernel_x + KERNEL_RADIUS) as usize];

            let neighbor_py = py as i32 + (kernel_x * 4) + (kernel_x * delta as i32);

            if neighbor_py < 0 || neighbor_py >= dst.len() as i32 {
                continue;
            }

            new_pixel[0] += dst[neighbor_py as usize] as f64 * kernal_value;
            new_pixel[1] += dst[neighbor_py as usize + 1] as f64 * kernal_value;
            new_pixel[2] += dst[neighbor_py as usize + 2] as f64 * kernal_value;
        }

        dst[py] = new_pixel[0] as u8;
        dst[py + 1] = new_pixel[1] as u8;
        dst[py + 2] = new_pixel[2] as u8;
        dst[py + 3] = src[py + 3];

        py += 4;
    }

    dst
}
