extern crate opencv;
extern crate term_size;

use opencv::{
    imgcodecs,
    core,
    prelude::*,
    Result,
    imgproc
};
use std::io::{self, Write};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
use std::thread;
use std::time::Duration;
use std::error::Error;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn get_terminal_size() -> Result<(i32, i32), &'static str> {
    match term_size::dimensions() {
        Some((w, h)) => Ok((w as i32, h as i32)),
        None => Err("Unable to determine terminal size"),
    }
}

fn scale_image(terminal_wh:(i32,i32), image_wh:(i32,i32), height_scale: f32) -> (i32, i32){
    let image_ar: f32 = image_wh.0 as f32 * (1.0/height_scale as f32) as f32 / image_wh.1 as f32 ;
    let termi_ar: f32 = terminal_wh.0 as f32   / terminal_wh.1 as f32;

    if image_ar > termi_ar {
        let new_height = (terminal_wh.0 as f32 / image_ar) as f32;
        return (terminal_wh.0  as i32, new_height  as i32) 
    } else {
        let new_width = (terminal_wh.1 as f32 * image_ar as f32);
        return (new_width as i32, terminal_wh.1 as i32) 
    }
}

fn build_colormap(image: &Mat, dimensions: (i32, i32)) -> Result<Vec<Vec<u8>>, opencv::Error> {
    // Resize the image to the new dimensions
    let mut resized = Mat::default();
    imgproc::resize(
        &image, 
        &mut resized, 
        core::Size::new(dimensions.0, dimensions.1), 
        0.0, 0.0, imgproc::INTER_LINEAR
    );

    // Create a map of colors
    let rows = resized.rows() as i32;
    let cols = resized.cols() as i32;
    let mut array = Vec::with_capacity(rows as usize);

    // Loop over everything and convert BGR info to a Color Index
    for r in 0..rows {
        let mut row = vec![0; cols as usize];  // Initialize each row with zeroes (or some other value)
        for c in 0..cols {
            let p = resized.at_2d::<core::Vec3b>(r, c)?; // Returns in BGR, not RGB
            row[c as usize] = rgb_to_256_color(p[2], p[1], p[0]);
        }
        array.push(row);
    }
    return Ok(array);
}

fn print_bitmap(colormap: Vec<Vec<u8>>, dimensions: (i32, i32)) {
    for r in 0..dimensions.1 {
        for c in 0..dimensions.0 {
            print!("\x1B[48;5;{}m \x1B[0m", colormap[r as usize][c as usize]);
        }
        println!();
    }
}


// Convert RGB to a color index (0-255)
fn rgb_to_256_color(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return (r - 8) / 247 * 24 + 232;
    }
    16 + (36 * (r / 51)) + (6 * (g / 51)) + (b / 51)
}

// Re-Render the image. Called by SIGWINCH.
fn render(image: &Mat, input_dims: (i32, i32)) -> Result<(), Box<dyn std::error::Error>> {
    // Get Terminal Size
    let (mut width, mut height) = match get_terminal_size() {
        Ok((h, w)) => (h, w),
        Err(e) => {
            eprintln!("Error getting terminal size: {}", e);
            return Ok(());
        }
    };

    // Calculate Scaling first
    let new_dimensions: (i32, i32) = scale_image((width, height), input_dims, DEFAULT_HEIGHT_RESCALE);   

    // Change the color map
    let colormap = build_colormap(&image, new_dimensions);
    match colormap {
        Ok(colormap) => print_bitmap(colormap, new_dimensions),
        Err(e) => eprintln!("Error: {}", e),
    }
    println!("Press Ctrl-C to Exit");
    return Ok(());
}

use std::env;
const DEFAULT_HEIGHT_RESCALE: f32 = 0.5;  // Shrink the height slightly
fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Handle args
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Please provide a target image path.");
        return Ok(());
    }

    // eg usage "imprev demo.png"
    let image_path = &args[1];


    let mut input_dims: (i32, i32)= (0,  0);
    let mut image = imgcodecs::imread(&image_path, imgcodecs::IMREAD_COLOR)?;
    if image.empty() {
        eprintln!("Could not read the image: {}", image_path);
        return Ok(());
    } else {
        let size: core::Size = image.size()?;
        input_dims = (size.width, size.height);
    }

    render(&image, input_dims);

    let mut signals = Signals::new(&[SIGWINCH])?;
    std::thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGWINCH => {
                    clear_screen();
                    render(&image, input_dims);
                },
                _ => unreachable!(),
            }
        }
    });

    loop {
        thread::sleep(Duration::from_secs(1));

    }
    
    // return Ok(());
}
