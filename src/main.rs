use crate::decomments::{format_from_file, Type};
use std::env;
use std::fs::write;
use std::path::Path;

mod decomments;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <path> <language code>", args[0]);
        return;
    }

    let file_path = &args[1];
    let lang_code = args[2].parse::<usize>().unwrap_or_else(|_| {
        println!("Please provide a valid integer for the language code.");
        std::process::exit(1);
    });

    let lang_type = match lang_code {
        0 => Type::Rust,
        1 => Type::CPP,
        2 => Type::Python,
        3 => Type::Haskell,
        _ => {
            println!("Invalid language code. Use 0 for Rust, 1 for CPP, 2 for Python, 3 for Haskell.");
            std::process::exit(1);
        }
    };

    match format_from_file(file_path, lang_type) {
        Ok(result) => {
            let path = Path::new(file_path);
            let formatted_file_path = path.with_file_name(
                format!(
                    "{}_formatted{}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    path.extension().map(|s| format!(".{}", s.to_str().unwrap())).unwrap_or_default()
                )
            );

            if let Err(err) = write(formatted_file_path, result) {
                println!("Error writing to file: {}", err);
            }
        }
        Err(err) => println!("Error: {}", err),
    }
}
