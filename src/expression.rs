// src/expression.rs

// Import required modules and functions from other files
use crate::{first, from_json, or_default, unique};
use crate::parser::{Parser, ParseError};
use serde_json::Value;

/// Enum representing an expression node in the AST.
/// - `Call`: Function call with a name and arguments.
/// - `Str`: String literal.
#[derive(Debug, Clone)]
pub enum ENode {
    Call { name: String, args: Vec<ENode> },
    Str(String),
}

/// Type alias for parse errors.
pub type EParseErr = ParseError;

/// Parses an expression from a string input into an AST node.
/// Returns an error if parsing fails or if there is trailing input.
pub fn parse_expr(input: &str) -> Result<ENode, EParseErr> {
    let mut p = EParser::new(input);
    let node = p.parse_node()?; // Parse the main node
    p.skip_ws(); // Skip any trailing whitespace
    if !p.eof() {
        // If there is extra input, return an error
        return Err(EParseErr::InvalidSyntax("trailing input".into()));
    }
    Ok(node)
}

/// Expression parser struct, wraps the generic `Parser`.
struct EParser<'a> {
    parser: Parser<'a>,
}

impl<'a> EParser<'a> {
    /// Creates a new expression parser from a string slice.
    fn new(s: &'a str) -> Self {
        Self {
            parser: Parser::new(s),
        }
    }

    /// Parses a single AST node (either a string or a function call).
    fn parse_node(&mut self) -> Result<ENode, EParseErr> {
        self.parser.skip_ws();
        // If the next character is a quote, parse a string literal
        if self.parser.peek_char() == Some('"') || self.parser.peek_char() == Some('\'') {
            return Ok(ENode::Str(self.parser.parse_quoted_string()?));
        }
        // Otherwise, parse a function call: name(args)
        let name = self.parser.parse_identifier()?;
        self.parser.skip_ws();
        self.parser.expect('(')?; // Expect opening parenthesis
        let args = self.parse_args()?; // Parse arguments
        self.parser.expect(')')?; // Expect closing parenthesis
        Ok(ENode::Call { name, args })
    }

    /// Parses a comma-separated list of arguments for a function call.
    fn parse_args(&mut self) -> Result<Vec<ENode>, EParseErr> {
        let mut out = Vec::new();
        self.parser.skip_ws();
        // If the next character is a closing parenthesis, there are no arguments
        if self.parser.peek_char() == Some(')') {
            return Ok(out);
        }
        loop {
            let node = self.parse_node()?; // Parse each argument node
            out.push(node);
            self.parser.skip_ws();
            // If a comma is found, continue parsing more arguments
            if self.parser.consume_char(',') {
                self.parser.skip_ws();
                continue;
            }
            break;
        }
        Ok(out)
    }

    /// Skips whitespace in the input.
    fn skip_ws(&mut self) {
        self.parser.skip_ws();
    }

    /// Checks if the parser has reached the end of input.
    fn eof(&self) -> bool {
        self.parser.eof()
    }
}

/// Helper function to check if the number of arguments matches the expected count.
fn check_arg_count(args: &[ENode], expected: usize) -> bool {
    args.len() == expected
}

/// Helper function to extract a string value from a JSON value.
/// Returns `Some(String)` if the value is a string, otherwise `None`.
fn extract_string(value: Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s),
        _ => None,
    }
}

/// Evaluates an AST node and returns a JSON value.
/// Supports built-in functions: from_json, first, unique, or_default.
pub fn eval_ast(node: &ENode) -> Value {
    match node {
        // If the node is a string, return it as a JSON string
        ENode::Str(s) => Value::String(s.clone()),
        // If the node is a function call, match the function name
        ENode::Call { name, args } => match name.as_str() {
            "from_json" => {
                // from_json(json_string, path_string)
                if !check_arg_count(args, 2) {
                    return Value::Null;
                }
                let json_s = match extract_string(eval_ast(&args[0])) {
                    Some(s) => s,
                    None => return Value::Null,
                };
                let path_s = match extract_string(eval_ast(&args[1])) {
                    Some(s) => s,
                    None => return Value::Null,
                };
                from_json(&json_s, &path_s)
            }
            "first" => {
                // first(array)
                if !check_arg_count(args, 1) {
                    return Value::Null;
                }
                first(&eval_ast(&args[0]))
            }
            "unique" => {
                // unique(array)
                if !check_arg_count(args, 1) {
                    return Value::Null;
                }
                unique(&eval_ast(&args[0]))
            }
            "or_default" => {
                // or_default(value, default_string)
                if !check_arg_count(args, 2) {
                    return Value::Null;
                }
                let v = eval_ast(&args[0]);
                let d = match extract_string(eval_ast(&args[1])) {
                    Some(s) => s,
                    None => return Value::Null,
                };
                or_default(&v, &d)
            }
            // Unknown function name, return null
            _ => Value::Null,
        },
    }
}
