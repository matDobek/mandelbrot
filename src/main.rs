extern crate crossbeam;
extern crate num;
extern crate gif;

use std::io::{self, Write};
use std::str::FromStr;
use std::fs::File;
use std::borrow::Cow;
use num::Complex;
use gif::SetParameter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(
            std::io::stderr(),
            r#"
                Usage: {0} NAME NO_OF_FRAMES ZOOM_SPEED POINT
                Example: {0} mandel 50 0.1 -0.77568377,0.13646737
            "#,
            args[0]
        ).unwrap();

        std::process::exit(1);
    }

    let size = (750, 750);
    let name = &args[1];
    let number_of_frames = usize::from_str(&args[2]).unwrap();
    let zoom_speed = f64::from_str(&args[3]).unwrap();
    let central_point = parse_complex(&args[4]).expect("error while parsing upper left point");
    let (upper_left, lower_right) = fetch_upper_left_and_lower_right_coordinates_based_on_central_point(central_point);

    generate_gif(name, number_of_frames, zoom_speed, size, upper_left, lower_right);
}

/// Fetch touple with `(upper_left, lower_right)` points, based on the `central_point`.
fn fetch_upper_left_and_lower_right_coordinates_based_on_central_point(central_point: Complex<f64>) -> (Complex<f64>, Complex<f64>){
    let upper_left: Complex<f64> = Complex { re: -2.0, im: 2.0 };
    let lower_right: Complex<f64> = Complex {
        re: central_point.re + ((upper_left.re - central_point.re).abs()).abs(),
        im: central_point.im - ((upper_left.im - central_point.im).abs()).abs(),
    };

    (upper_left, lower_right)
}

fn generate_gif(
    filename: &str,
    number_of_frames: usize,
    zoom_speed: f64,
    size: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
    ) -> () {

    let mut frame_states = Vec::new();

    let mut zoomed_upper_left = upper_left;
    let mut zoomed_lower_right = lower_right;

    for index in 0..number_of_frames {
        let frame_state = generate_frame(size, zoomed_upper_left, zoomed_lower_right);
        frame_states.push(frame_state);

        let width = (zoomed_upper_left.re - zoomed_lower_right.re).abs();
        let height = (zoomed_upper_left.im - zoomed_lower_right.im).abs();

        let zoomed_width = width * zoom_speed;
        let zoomed_height = height * zoom_speed;

        zoomed_upper_left = zoomed_upper_left + Complex {
            re: zoomed_width,
            im: -zoomed_height,
        };
        zoomed_lower_right = zoomed_lower_right + Complex {
            re: -zoomed_width,
            im: zoomed_height,
        };

        print!("\rProgress: {}%", ((1.0 + index as f32) / number_of_frames as f32 * 100.0).round());
        io::stdout().flush().unwrap();
    }

    let filename_with_extension = format!("{}.gif", filename);
    let mut image = File::create(filename_with_extension).unwrap();
    let mut encoder = gif::Encoder::new(&mut image, size.0 as u16, size.1 as u16, &fetch_color_map()).unwrap();
    encoder.set(gif::Repeat::Infinite).unwrap();

    for i in 0..frame_states.len() * 2 {
        let index = match i {
            i if i < frame_states.len()  => i,
            _ => 2*frame_states.len() - 1 - i,
        };

        let mut frame = gif::Frame::default();
        frame.width = size.0 as u16;
        frame.height = size.1 as u16;
        frame.buffer = Cow::Borrowed(&*frame_states[index]);
        encoder.write_frame(&frame).unwrap();
    }
}

fn fetch_color_map() -> [u8; 256 * 3] {
    let mut color_map: [u8; 256*3] = [0; 256*3];

    for i in 0..256 {
        let rgb = [255 - i, 255 - i, 255 - i];

        for j in 0..rgb.len() {
            color_map[i * rgb.len() + j] = rgb[j] as u8;
        }
    }

    color_map
}

/// Generate single frame, represented as `Vec<u8>` where `u8` values represent how many iterations there were needed for given point, to leave the set.
fn generate_frame(
    size: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
    ) -> Vec<u8> {
    let mut pixels = vec![0; size.0 * size.1];

    let threads = 4;

    let rows_per_band = size.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * size.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / size.0;

                let band_size = (size.0, height);
                let band_upper_left = pixel_to_point(size, (0, top), upper_left, lower_right);
                let band_lower_right =
                    pixel_to_point(size, (size.0, top + height), upper_left, lower_right);

                spawner.spawn(move || {
                    render(band, band_size, band_upper_left, band_lower_right);
                });
            }
        });
    }

    return pixels;
}

/// Render a rectangle of the Mandelbrot set into a buffer of pixels
///
/// The `size` argument gives the width and height of the buffer `pixels`,
/// which holds one grayscale pixel per byte.
/// The `upper_left`, and `lower_right` specify points on the complex plane
/// corresponding to the upper left and lower right corners of the `pixels` buffer.
///
fn render(
    pixels: &mut [u8],
    size: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    assert!(pixels.len() == size.0 * size.1);

    for row in 0..size.1 {
        for column in 0..size.0 {
            let point = pixel_to_point(size, (column, row), upper_left, lower_right);
            pixels[column + row * size.0] = match escape_time(point, 255) {
                None => 255,
                Some(count) => count as u8,
            };
        }
    }
}

/// Given the row and column of pixel on the image,
/// return the corresponding point on the complex plane.
///
/// `size` is a pair of (width, height) of given image
/// `pixel` represents (column, row)
/// The `upper_left` and `lower_right` are points on the complex plane, designating the arena of the image
///
fn pixel_to_point(
    size: (usize, usize),
    pixel: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) -> Complex<f64> {
    let complex_width = lower_right.re - upper_left.re;
    let complex_height = upper_left.im - lower_right.im;

    Complex {
        re: upper_left.re + pixel.0 as f64 * complex_width / size.0 as f64,
        im: upper_left.im - pixel.1 as f64 * complex_height / size.1 as f64,
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (100, 100),
            (25, 75),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 },
        ),
        Complex { re: -0.5, im: -0.5 }
    );
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re: re, im: im }),
        None => None,
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.0"), None);
    assert_eq!(parse_complex("1.0,2.0"), Some(Complex { re: 1.0, im: 2.0 }));
}

/// Parse string `s` as a coordinate pair, like `"400x600", or "1.0,0.5"`
/// If `s` has the proper form, return `Some<(x,y)>`
/// If it don't parse correctly, return `None`
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
            (Ok(left), Ok(right)) => Some((left, right)),
            _ => None,
        },
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",20", ','), None);
    assert_eq!(parse_pair::<i32>("10,20x", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10x20", 'x'), Some((10, 20)));
}

/// Try to determine if `c` is in the Mandelbrot set, using at most `limit` iterations.
///
/// If `c` is NOT a member return `Some(i)`, where `i` is the number of iterations it took `c` to leave the set.
/// If `c` seems to be a member (stays withing set in `limit` iterations), return `None`.
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;

        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }

    None
}
