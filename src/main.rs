use clap::Parser;
use serde_json::Value;
use jp::{JsonPath, JpOptions, CmpMode};

/// Simple runner: pass JSON and JSONPath via CLI.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// JSON document (string). You can also pipe a file using shell quoting.
    json: String,
    /// JSONPath-like expression
    path: String,
    /// Case-insensitive compare via lower-casing (optional flag)
    #[arg(long)]
    ci: bool,
    /// Fallback default JSON (optional)
    #[arg(long)]
    default: Option<String>,
    /// Show only the first match (optional flag)
    #[arg(long)]
    first: bool,
    /// Deduplicate results (optional flag)
    #[arg(long)]
    unique: bool,
}

fn main() {
    // Parse CLI arguments.
    let args = Args::parse();

    // Parse input JSON.
    let data: Value = match serde_json::from_str(&args.json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Invalid JSON: {e}");
            std::process::exit(1);
        }
    };

    // Build options.
    let mut opts = JpOptions::default();
    if args.ci { opts.cmp = CmpMode::CaseFoldLower; }
    if let Some(def) = args.default.as_ref() {
        match serde_json::from_str::<Value>(def) {
            Ok(v) => opts.default = Some(v),
            Err(_) => opts.default = Some(Value::String(def.clone())),
        }
    }

    // Create JsonPath evaluator.
    let jp = JsonPath::new(&data).with_options(opts.clone());

    // Query the path.
    let mut out = jp.query(&args.path);

    // Post-process results as requested.
    if args.unique { out = jp.unique(&out); }
    if args.first { out = jp.first(&out); }
    if opts.default.is_some() { out = jp.or_default(&out, opts.default.clone().unwrap()); }

    // Output result.
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}