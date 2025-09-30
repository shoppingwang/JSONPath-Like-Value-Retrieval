use clap::Parser; // Import clap for command-line argument parsing
use json_path_like_value_retrieval as jpl; // Import the json_path_like_value_retrieval crate as jpl
use std::fs; // Import filesystem utilities
use std::io::{self, Read}; // Import IO traits and types
use tracing::{error, info};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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

fn init_tracing() {
    // Layer formatting: suppress time/level/target so successful JSON result is printed raw.
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time();

    // Initialize global tracing subscriber once. If already set (e.g. by external caller), ignore error.
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt_layer)
        .with(ErrorLayer::default());
    let _ = tracing::subscriber::set_global_default(registry); // ignore error if already set
}

fn main() {
    init_tracing();
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

    // Evaluate the expression using the jpl crate
    match jpl::eval(&expr) {
        Ok(v) => {
            // If successful, pretty-print the result as JSON via tracing (info level)
            match serde_json::to_string_pretty(&v) {
                Ok(json) => info!(target: "jpl", "{json}"),
                Err(e) => {
                    error!(target: "jpl", error = %e, "Failed to serialize evaluation result")
                }
            }
        }
        Err(e) => {
            // If evaluation fails, log the error and exit with code 1
            error!(target: "jpl", error = %e, "Evaluation failed");
            std::process::exit(1);
        }
    }
}
