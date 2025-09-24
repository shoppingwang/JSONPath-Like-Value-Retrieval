use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Full expression string, e.g.
    /// first(from_json("<JSON>", "$.path"))
    expr: String,
}

fn main() {
    let args = Args::parse();
    let v = jp::eval_expr(&args.expr);
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}