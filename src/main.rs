use std::env;
use std::fs::write;
use walkdir::WalkDir;
use crate::decomments::{proc_trimming, Type};

mod decomments;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }

    let root_path = &args[1];

    for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();
        if file_path.is_file() {
            if let Some(extension) = file_path.extension().and_then(|s| s.to_str()) {
                let lang_type = match extension {
                    "c" | "cpp" | "cs" | "h" | "hpp" | "inl" | "rs" | "java" | "kt" => Type::RustC,
                    "py" => Type::Python,
                    "hs" => Type::Haskell,
                    "htm" | "html" | "xml" => Type::Markup,
                    _ => continue, // Skip files with other extensions
                };

                match proc_trimming(file_path.to_str().unwrap(), lang_type) {
                    Ok(contents) => {
                        if write(file_path, contents).is_ok() {
                            println!("*** {} has been successfully processed", file_path.display());
                        }
                    }
                    Err(_) => {
                        println!("*** Failed to process {}", file_path.display());
                    }
                }
            }
        }
    }
}
