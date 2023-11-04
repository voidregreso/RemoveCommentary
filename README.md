# RemoveCommentary

A simple and efficient tool to clean up comments from your codebase recursively.

## Features

- Supports multiple languages: C-styled languages (C/C++/Java/Rust/C#), Python, Haskell, and Markup (HTML/XML).
- Recursively traverses directories to process all applicable files.
- Safely removes both single-line and multi-line/block comments.

## Usage

To use this tool, run the executable with the directory path as the argument:

```
RemoveCommentary <path_to_directory>
```

For example:

```
RemoveCommentary ./src
```

This will process all files in the `./src` directory, removing comments based on the file extensions.

**Note: Please backup your codebase in advance in case of unexpected damages.**

## Requirements

- Rust Programming Language
- `walkdir` and `derive_more` crates

## Building

Compile the program with Rust's package manager, Cargo:

```
cargo build --release
```

The resulting executable will be in `target/release`.

## Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request.
