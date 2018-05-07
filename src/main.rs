extern crate crossbeam;
extern crate image;
extern crate num;

use num::Complex;
use std::str::FromStr;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(
            std::io::stderr(),
            r#"
            Usage: {0} FILE PIXELS UPPERLEFT LOWERRIGHT
            Example: {0} mandel.png 1000x750 -1.20,0.35 -1.0,0.20
        "#,
            args[0]
        ).unwrap();

        std::process::exit(1);
    }

    let size = parse_pair(&args[2], 'x').expect("error while parsing pixel size");
    let upper_left = parse_complex(&args[3]).expect("error while parsing upper left point");
    let lower_right = parse_complex(&args[4]).expect("error while parsing upper left point");

    let mut pixels = vec![0; size.0 * size.1];

    let threads = 4;
    println!("Threads: {}", threads);

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

    write_image(&args[1], &pixels, size).expect("error writing file");
}

fn write_image(filename: &str, pixels: &[u8], size: (usize, usize)) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, size.0 as u32, size.1 as u32, ColorType::Gray(8))?;

    Ok(())
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
                None => 0,
                Some(count) => 255 - count as u8,
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
