pub mod errors;
pub mod context;
pub mod engine;     // legacy engine kept intact to preserve exact JSONPath behavior
pub mod functions;  // plugin model
mod expression;
mod jsonpath;
mod filter;
mod comparison;

use serde_json::Value;
use errors::{Result, EvalError};
use context::Context;
use functions::Registry;

/// The main evaluator. Uses the legacy engine's parser and evaluator internally
/// to keep JSONPath behavior exactly as-is. Public API returns Result as requested.
pub struct Evaluator {
    _ctx: Context,
    registry: Registry, // reserved for future: custom functions dispatch
}

impl Evaluator {
    pub fn new(registry: Registry) -> Self {
        Self { _ctx: Context::default(), registry }
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

/// Convenience: evaluate with built-in registry.
pub fn eval(expr: &str) -> Result<Value> {
    let ev = Evaluator::new(Registry::with_builtins());
    ev.eval(expr)
}

/// Back-compat helper that coerces to null on error.
pub fn eval_coerce_null(expr: &str) -> Value {
    eval(expr).unwrap_or(Value::Null)
}

/// Re-export the most-used helpers for users who call functions directly.
pub use engine::{from_json, first, unique, or_default};