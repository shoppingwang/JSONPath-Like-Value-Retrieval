mod comparison;
pub mod engine;
pub mod errors;
mod expression;
mod filter;
mod jsonpath;
mod parser;

use errors::{EvalError, Result};
use serde_json::Value;

/// The main evaluator. Simplified to focus on core functionality.
pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate an expression; returns Result instead of Null-on-error.
    pub fn eval(&self, expr: &str) -> Result<Value> {
        // We delegate to the original engine for correctness.
        // If the engine's parser fails, convert to EvalError::Parse.
        let ast = match expression::parse_expr(expr) {
            Ok(ast) => ast,
            Err(e) => return Err(EvalError::Parse(format!("{e:?}"))),
        };
        let value = expression::eval_ast(&ast);
        Ok(value)
    }
}

/// Convenience: evaluate with default evaluator.
pub fn eval(expr: &str) -> Result<Value> {
    let ev = Evaluator::new();
    ev.eval(expr)
}

/// Back-compat helper that coerces to null on error.
pub fn eval_coerce_null(expr: &str) -> Value {
    eval(expr).unwrap_or(Value::Null)
}

/// Re-export the most-used helpers for users who call functions directly.
pub use engine::{first, from_json, or_default, unique};
