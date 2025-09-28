use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

pub type Result<T> = std::result::Result<T, EvalError>;