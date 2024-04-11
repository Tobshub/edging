use raylib::prelude::*;
use std::fs::File;
use std::io::Cursor;

fn main() {
    const IMG_PATH: &str = "owl.png";
    let decoder = png::Decoder::new(File::open(IMG_PATH).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info().clone();
    let mut buf = vec![0; reader.output_buffer_size()];

    let px_width = info.bytes_per_pixel();

    let frame_info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..frame_info.buffer_size()];

    println!(
        "{}x{}; len: {}; px_width: {}",
        frame_info.width,
        frame_info.height,
        bytes.len(),
        px_width
    );
    let bytes = bytes_to_grayscale(bytes, px_width);
    let bytes = gaussian_blur(bytes.as_slice(), frame_info.width as i32);
    let bytes = gradient_thresholding(bytes.as_slice(), frame_info.width as usize);
    let bytes = double_threshold(bytes.as_slice());
    let bytes = hysteresis(bytes, frame_info.width as i32);

    let mut w = Cursor::new(vec![]);

    let mut encoder = png::Encoder::new(&mut w, frame_info.width, frame_info.height);
    encoder.set_color(png::ColorType::Grayscale);
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

fn bytes_to_grayscale(src: &[u8], px_width: usize) -> Vec<u8> {
    let mut dst = vec![0; src.len() / px_width];
    let mut i = 0;
    while i < src.len() - px_width + 1 {
        let lum = match px_width {
            1 => src[i],
            3 | 4 => {
                (src[i] as f32 * Luminosity::Red.value()
                    + src[i + 1] as f32 * Luminosity::Green.value()
                    + src[i + 2] as f32 * Luminosity::Blue.value()) as u8
            }
            _ => unreachable!(),
        };

        dst[i / px_width] = lum;
        i += px_width;
    }

    dst
}

const KERNEL_RADIUS: i32 = 2;
const KERNEL_SIZE: usize = (KERNEL_RADIUS * 2 + 1) as usize;

// assumes grayscale has been applied
fn gaussian_blur(src: &[u8], image_width: i32) -> Vec<u8> {
    let mut dst = vec![0; src.len()];
    let mut kernel: [f64; KERNEL_SIZE] = [0.0; KERNEL_SIZE];
    let mut sum = 0.0;

    let sigma: f64 = ((KERNEL_RADIUS / 2) as f64).max(1.0);

    // compute kernel for 1D gaussian blur
    for x in -KERNEL_RADIUS..=KERNEL_RADIUS {
        let exponent = -(x * x) as f64 / (2.0 * sigma * sigma);
        let numerator = std::f64::consts::E.powf(exponent);
        let denominator = 2.0 * std::f64::consts::PI * sigma * sigma;

        let kernel_value = numerator / denominator;
        kernel[(x + KERNEL_RADIUS) as usize] = kernel_value;
        sum += kernel_value;
    }

    // normalize kernel
    (0..KERNEL_SIZE).for_each(|x| {
        kernel[x] /= sum;
    });

    // apply kernel in x direction
    (0..src.len()).for_each(|px| {
        let mut new_pixel = 0.0;

        for kernel_x in -KERNEL_RADIUS..=KERNEL_RADIUS {
            let kernel_value = kernel[(kernel_x + KERNEL_RADIUS) as usize];

            let neighbor_px = px as i32 + kernel_x;

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            let npx = src[neighbor_px as usize] as f64 * kernel_value;
            new_pixel += npx;
        }

        dst[px] = new_pixel as u8;
    });

    // apply kernel in y direction
    (0..dst.len()).for_each(|py| {
        let mut new_pixel = 0.0;

        for kernel_x in -KERNEL_RADIUS..=KERNEL_RADIUS {
            let kernel_value = kernel[(kernel_x + KERNEL_RADIUS) as usize];

            let neighbor_py = py as i32 + kernel_x * image_width;

            if neighbor_py < 0 || neighbor_py >= dst.len() as i32 {
                continue;
            }

            let npx = dst[neighbor_py as usize] as f64 * kernel_value;
            new_pixel += npx;
        }

        dst[py] = new_pixel as u8;
    });

    dst
}

/// assumes grayscale & gaussian blur have already been applied
/// the output is not suitable for drawing
/// output format: [gradient_magnitude, gradient_direction]
/// hence the output size is twice the input size
fn sobel_filter(src: &[u8], image_width: i32) -> Vec<u8> {
    let mut dst = vec![0; src.len() * 2];

    let kernel_phase_1: [i32; 3] = [1, 2, 1];
    let kernel_phase_2: [i32; 3] = [1, 0, -1];

    // apply kernel in x direction
    (0..src.len()).for_each(|px| {
        let mut new_pixel: i32 = 0;

        for kernel_x in -1..=1 {
            let kernel_value = kernel_phase_1[(kernel_x + 1) as usize];

            let neighbor_px = px as i32 + kernel_x;

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            let npx = src[neighbor_px as usize] as i32 * kernel_value;
            new_pixel += npx;
        }

        for kernel_x in -1..=1 {
            let kernel_value = kernel_phase_2[(kernel_x + 1) as usize];

            let neighbor_px = px as i32 + kernel_x;

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            let npx = new_pixel * kernel_value;
            new_pixel += npx;
        }

        dst[px * 2] = new_pixel.unsigned_abs() as u8;
    });

    // apply kernel in y direction
    (0..src.len()).for_each(|py| {
        let mut new_pixel: i32 = 0;

        for kernel_x in -1..=1 {
            let kernel_value = kernel_phase_2[(kernel_x + 1) as usize];

            let neighbor_px = py as i32 + kernel_x * image_width;

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            let npx = src[neighbor_px as usize] as i32 * kernel_value;
            new_pixel += npx;
        }

        for kernel_x in -1..=1 {
            let kernel_value = kernel_phase_1[(kernel_x + 1) as usize];

            let neighbor_px = py as i32 + kernel_x * image_width;

            if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                continue;
            }

            let npx = new_pixel * kernel_value;
            new_pixel += npx;
        }

        let ndst = ((dst[py * 2] as u32).pow(2) + new_pixel.unsigned_abs().pow(2)) as f32;
        let angle = (new_pixel as f32)
            .atan2(dst[py * 2] as f32)
            .to_degrees()
            .abs();
        dst[py * 2] = ndst.sqrt().ceil() as u8;
        let angle = match angle {
            0.0..=22.4 | 157.5..=180.0 => 0,
            22.5..=67.4 => 45,
            67.5..=112.4 => 90,
            112.5..=157.4 => 135,
            _ => panic!("Unexpected angle: {}", angle),
        };
        dst[py * 2 + 1] = angle;
    });

    dst
}

// assumes grayscale has already been applied
fn gradient_thresholding(src: &[u8], image_width: usize) -> Vec<u8> {
    let src = sobel_filter(src, image_width as i32);
    let image_width = image_width * 2;
    let mut dst = vec![0; src.len() / 2];

    let mut px = 0;
    while px < src.len() {
        let mut new_pixel = 0;

        let angle = src[px + 1];
        let mut cmp_pxs: [i32; 2] = [0; 2];
        match angle {
            0 => {
                cmp_pxs[0] = px as i32 - 2;
                cmp_pxs[1] = px as i32 + 2;
            }
            45 => {
                cmp_pxs[0] = px as i32 - image_width as i32 + 2;
                cmp_pxs[1] = px as i32 + image_width as i32 - 2;
            }
            90 => {
                cmp_pxs[0] = px as i32 - image_width as i32;
                cmp_pxs[1] = px as i32 + image_width as i32;
            }
            135 => {
                cmp_pxs[0] = px as i32 - image_width as i32 - 2;
                cmp_pxs[1] = px as i32 + image_width as i32 + 2;
            }
            _ => panic!("Unexpected angle: {}", angle),
        };

        for cmp_px in cmp_pxs {
            if cmp_px < 0 || cmp_px >= src.len() as i32 {
                continue;
            }
            if src[cmp_px as usize] > src[px] {
                new_pixel = 0;
                break;
            }

            new_pixel = src[px];
        }

        dst[px / 2] = new_pixel;

        px += 2;
    }

    dst
}

fn double_threshold(src: &[u8]) -> Vec<u8> {
    let mut dst = vec![0; src.len()];

    let max = *src.iter().max().unwrap();
    let high = (max as u32 * 7 / 10) as u8;
    let low = (max as u32 * 5 / 10) as u8;

    (0..src.len()).for_each(|px| {
        if src[px] < low {
            dst[px] = 0;
        } else if src[px] > high {
            dst[px] = 255;
        } else {
            dst[px] = 25;
        }
    });

    dst
}

fn hysteresis(mut src: Vec<u8>, image_width: i32) -> Vec<u8> {
    (0..src.len()).for_each(|px| {
        if src[px] != 255 && src[px] != 0 {
            let blob = [
                //top
                px as i32 - image_width - 1,
                px as i32 - image_width,
                px as i32 - image_width + 1,
                // left and right
                px as i32 - 1,
                px as i32 + 1,
                // bottom
                px as i32 + image_width - 1,
                px as i32 + image_width,
                px as i32 + image_width + 1,
            ];

            for neighbor_px in blob {
                if neighbor_px < 0 || neighbor_px >= src.len() as i32 {
                    continue;
                }
                match src[neighbor_px as usize] {
                    255 => {
                        src[px] = 255;
                        break;
                    }
                    _ => {
                        src[px] = 0;
                    }
                }
            }
        }
    });

    src
}
