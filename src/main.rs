use clap::{Parser, ValueEnum};
use json_path_like_extraction as jple;
use serde_json::json;
use std::fs;
use std::io::{self, Read};

#[derive(Clone, ValueEnum, Debug)]
enum ErrorFormat {
    Human,
    Json,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Expression string: e.g. first(from_json("<JSON>", "$.path"))
    #[arg(long = "expr")]
    expr: Option<String>,

    /// File containing the expression
    #[arg(long = "expr-file")]
    expr_file: Option<String>,

    /// Error output format
    #[arg(long = "error-format", value_enum, default_value_t = ErrorFormat::Human)]
    error_format: ErrorFormat,

    /// Increase verbosity (also controllable via RUST_LOG)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() {
    // init logging
    let args = Args::parse();
    if args.verbose > 0 {
        let level = match args.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };
        std::env::set_var("RUST_LOG", level);
    }
    tracing_subscriber::fmt::init();

    let expr = if let Some(e) = args.expr.clone() {
        e
    } else if let Some(path) = args.expr_file.clone() {
        fs::read_to_string(path).expect("failed to read --expr-file")
    } else {
        read_stdin().expect("failed to read expression from stdin")
    };

    match jple::eval(&expr) {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        Err(e) => {
            match args.error_format {
                ErrorFormat::Human => {
                    eprintln!("{e}");
                }
                ErrorFormat::Json => {
                    let out = json!({
                        "error": e.to_string(),
                        "kind": format!("{:?}", e),
                    });
                    println!("{}", serde_json::to_string_pretty(&out).unwrap());
                }
            }
            std::process::exit(1);
        }
    }
}
