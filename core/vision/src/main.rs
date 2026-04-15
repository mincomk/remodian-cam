mod detect;
mod preprocess;

use detect::{calc::get_digit, template::load_templates};
use preprocess::{preprocess, FileImageSource};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 10 {
        eprintln!("Usage: {} <image> <x1> <y1> <x2> <y2> <x3> <y3> <x4> <y4>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    let points = parse_points(&args[2..]);

    let source = FileImageSource::new(path).unwrap_or_else(|e| {
        eprintln!("Failed to load image '{}': {}", path, e);
        std::process::exit(1);
    });

    let sample = preprocess(&source, points);
    let cells = sample.extract_cells();
    let templates = load_templates();

    match get_digit(&templates, &cells, &sample) {
        Some(digit) => println!("Digit: {}", digit),
        None => println!("Digit: 0"),
    }
}

fn parse_points(args: &[String]) -> [(u32, u32); 4] {
    let nums: Vec<u32> = args.iter()
        .map(|s| s.parse().unwrap_or_else(|_| {
            eprintln!("Invalid coordinate: {}", s);
            std::process::exit(1);
        }))
        .collect();
    [(nums[0], nums[1]), (nums[2], nums[3]), (nums[4], nums[5]), (nums[6], nums[7])]
}
