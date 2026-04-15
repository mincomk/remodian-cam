use std::{
    io::{self, Write},
    path::Path,
};

fn main() {
    println!("Transforming templates...");

    // 0-9
    for i in 0..10 {
        let input_file = input_filename(i);
        let output_file = output_filename(i);
        transform_file(input_file, output_file).expect("Failed to transform file");
        println!("Transformed template for digit {}", i);
    }
}

fn input_filename(i: u8) -> String {
    format!("data/20_bin/p{i}.txt.bin")
}

fn output_filename(i: u8) -> String {
    format!("templates/{}.bin", i)
}

fn transform_file(input_file: impl AsRef<Path>, output: impl AsRef<Path>) -> io::Result<()> {
    let input = std::fs::read_to_string(input_file)?;
    let cells = transform_text(&input);
    let mut output_file = std::fs::File::create(output)?;
    output_file.write_all(&cells)?;

    Ok(())
}

fn transform_text(input: &str) -> [u8; 35] {
    let mut cells = [0u8; 35];
    for (i, c) in input.replace("\n", "").chars().take(35).enumerate() {
        if c == '1' {
            cells[i] = 255;
        } else if c == '0' {
            cells[i] = 0;
        } else {
            panic!("Invalid character in input: {}", c);
        }
    }
    cells
}
