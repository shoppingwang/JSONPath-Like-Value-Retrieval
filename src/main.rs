use clap::Parser;
use json_path_like_extraction as jple;
use std::fs;
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Expression string: e.g. first(from_json("<JSON>", "$.path"))
    #[arg(long = "expr")]
    expr: Option<String>,

    /// File containing the expression
    #[arg(long = "expr-file")]
    expr_file: Option<String>,
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() {
    let args = Args::parse();

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
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
