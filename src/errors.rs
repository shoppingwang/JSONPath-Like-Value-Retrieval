use thiserror::Error; // Import the `Error` derive macro from the `thiserror` crate

// Define an enum to represent possible evaluation errors
#[derive(Debug, Error)] // Automatically implement `Debug` and `Error` traits for the enum
pub enum EvalError {
    // Variant for errors that occur during parsing, with a message
    #[error("parse error: {0}")] // Custom error message formatting for this variant
    Parse(String),

    // Variant for errors that occur during runtime, with a message
    #[error("runtime error: {0}")] // Custom error message formatting for this variant
    Runtime(String),
}

// Type alias for results that use `EvalError` as the error type
pub type Result<T> = std::result::Result<T, EvalError>;
