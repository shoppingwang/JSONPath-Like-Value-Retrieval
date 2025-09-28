mod comparison; // Handles comparison operations for expressions
pub mod engine; // Core engine logic, exposed publicly
pub mod errors; // Error types and result handling, exposed publicly
mod expression; // Expression parsing and evaluation logic
mod filter; // Filtering logic for data structures
mod jsonpath; // JSONPath query support
mod parser; // Parsing utilities

use errors::{EvalError, Result}; // Import custom error and result types
use serde_json::Value; // JSON value type from serde_json

/// The main evaluator struct.
/// Provides methods to evaluate expressions and return results.
pub struct Evaluator;

impl Evaluator {
    /// Creates a new Evaluator instance.
    pub fn new() -> Self {
        Self
    }

    /// Evaluates a string expression and returns a Result<Value>.
    /// If parsing fails, returns an EvalError::Parse.
    /// Delegates parsing and evaluation to the expression module.
    pub fn eval(&self, expr: &str) -> Result<Value> {
        // Parse the expression string into an AST (Abstract Syntax Tree)
        let ast = match expression::parse_expr(expr) {
            Ok(ast) => ast,
            // On parse error, wrap the error in EvalError::Parse and return
            Err(e) => return Err(EvalError::Parse(format!("{e:?}"))),
        };
        // Evaluate the AST and return the resulting value
        let value = expression::eval_ast(&ast);
        Ok(value)
    }
}

/// Convenience function to evaluate an expression using a default Evaluator.
/// Returns a Result<Value> for error handling.
pub fn eval(expr: &str) -> Result<Value> {
    let ev = Evaluator::new();
    ev.eval(expr)
}

/// Helper function for backward compatibility.
/// Evaluates an expression and returns Value::Null if any error occurs.
pub fn eval_coerce_null(expr: &str) -> Value {
    eval(expr).unwrap_or(Value::Null)
}

/// Re-export commonly used helpers from the engine module for convenience.
/// These functions can be called directly by users of this library.
pub use engine::{first, from_json, or_default, unique};
