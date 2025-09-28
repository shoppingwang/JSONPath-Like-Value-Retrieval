use clap::Parser; // Import clap for command-line argument parsing
use json_path_like_extraction as jple; // Import the json_path_like_extraction crate as jple
use std::fs; // Import filesystem utilities
use std::io::{self, Read}; // Import IO traits and types

// Define a struct to hold command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Expression string: e.g. first(from_json("<JSON>", "$.path"))
    #[arg(long = "expr")]
    expr: Option<String>, // Optional expression string

    /// File containing the expression
    #[arg(long = "expr-file")]
    expr_file: Option<String>, // Optional path to a file containing the expression
}

// Reads all data from stdin and returns it as a String
fn read_stdin() -> io::Result<String> {
    let mut buf = String::new(); // Buffer to store input
    io::stdin().read_to_string(&mut buf)?; // Read from stdin into buffer
    Ok(buf) // Return the buffer as a String
}

fn main() {
    let args = Args::parse(); // Parse command-line arguments

    // Determine the source of the expression:
    // 1. Use --expr if provided
    // 2. Otherwise, use --expr-file if provided
    // 3. Otherwise, read from stdin
    let expr = if let Some(e) = args.expr.clone() {
        e
    } else if let Some(path) = args.expr_file.clone() {
        fs::read_to_string(path).expect("failed to read --expr-file")
    } else {
        read_stdin().expect("failed to read expression from stdin")
    };

    // Evaluate the expression using the jple crate
    match jple::eval(&expr) {
        Ok(v) => {
            // If successful, pretty-print the result as JSON
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        Err(e) => {
            // If evaluation fails, print the error and exit with code 1
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
