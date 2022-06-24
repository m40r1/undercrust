extern crate crossbeam;
extern crate image;
extern crate num;

use image::png::PNGEncoder;
use image::ColorType;
use num::Complex;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

/// Parse the string `s` as a coordinate pair, like `"400x600"` or `"1.0,0.5"`.
///
/// Specifically, `s` should have the form <left><sep><right>, where <sep> is
/// the character given by the `separator` argument, and <left> and <right> are both
/// strings that can be parsed by `T::from_str`.
///
/// If `s` has the proper form, return `Some<(x, y)>`. If it doesn't parse
/// correctly, return `None`.
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
            (Ok(l), Ok(r)) => Some((l, r)),
            _ => None,
        },
    }
}
#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}
/// Parse a pair of floats separated by a ',' as a complex num
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None,
    }
}
#[test]
fn teste_parse_complex() {
    assert_eq!(
        parse_complex("1.25,-0.0625"),
        Some(Complex {
            re: 1.25,
            im: -0.0625
        })
    );
    assert_eq!(parse_complex(",-0.0625"), None);
}
/// Given the row & column of the pixel
/// return its position on the complex plane
///
///'bounds' is a pair with the width & height of the image in pixels
/// 'pixel' is a (column,row) pair for a particular pixel
/// 'up_left' & 'low_right' are points on the complex plane
/// designating the area of our image
fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    up_left: Complex<f64>,
    low_right: Complex<f64>,
) -> Complex<f64> {
    let (width, height) = (low_right.re - up_left.re, up_left.im - low_right.im);

    Complex {
        re: up_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: up_left.im - pixel.1 as f64 * height / bounds.1 as f64,
        // pixel.1 goes up and we go down
        // the imaginay parte incresaes as we go up
    }
}
#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (100, 100),
            (25, 75),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 }
        ),
        Complex { re: -0.5, im: -0.5 }
    );
}
/// Try to determine if `c` is in the Mandelbrot set, using at most `limit`
/// iterations to decide.p
///
/// If `c` is not a member, return `Some(i)`, where `i` is the number of
/// iterations it took for `c` to leave the circle of radius two centered on the
/// origin. If `c` seems to be a member (more precisely, if we reached the
/// iteration limit without being able to prove that `c` is not a member),
/// return `None`.
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };

    for bi in 0..limit {
        z = z * z + c;

        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}
/// Render a rectangle of the Mandelbrot set in a buffer
///
/// 'bounds' argument gives the width & height of the 'pixels' buffer
/// with one grayscale pixel per byte.up_left & low_right specify on the complex plane
/// the up_left & low_right corners of the buffer
fn render(
    pixels: &mut [u8],
    bounds: (usize, usize),
    up_left: Complex<f64>,
    low_right: Complex<f64>,
) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), up_left, low_right);

            pixels[row * bounds.0 + column] = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8,
            };
        }
    }
}
/// Write the buffer 'pixels',dimensions given by 'bounds'
/// to 'filename'
fn write_image(
    filename: &str,
    pixels: &[u8],
    bounds: (usize, usize),
) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(
        &pixels,
        bounds.0 as u32,
        bounds.1 as u32,
        ColorType::Gray(8),
    )?;

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        writeln!(
            std::io::stderr(),
            "Usage: mandelbrot FILE PIXELS UP_LEFT LOW_RIGHT"
        )
        .unwrap();
        writeln!(
            std::io::stderr(),
            "Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
            args[0]
        )
        .unwrap();
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2], 'x').expect("err parsing dimensions");
    let up_left = parse_complex(&args[3]).expect("err parsinf up_left corner point");
    let low_right = parse_complex(&args[4]).expect("err parsing low_right corner point");
    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = 2;
    let rows_per_band = bounds.1 / threads + 1;
    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);

                let band_up_left = pixel_to_point(bounds, (0, top), up_left, low_right);
                let band_low_right =
                    pixel_to_point(bounds, (bounds.0, top + height), up_left, low_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_up_left, band_low_right);
                });
            }
        });
    }

    write_image(&args[1], &pixels, bounds).expect("err writing png");
}
