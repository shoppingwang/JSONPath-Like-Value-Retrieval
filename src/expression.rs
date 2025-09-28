// src/expression.rs
use crate::{first, from_json, or_default, unique};
use crate::parser::{Parser, ParseError};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ENode {
    Call { name: String, args: Vec<ENode> },
    Str(String),
}

pub type EParseErr = ParseError;

pub fn parse_expr(input: &str) -> Result<ENode, EParseErr> {
    let mut p = EParser::new(input);
    let node = p.parse_node()?;
    p.skip_ws();
    if !p.eof() {
        return Err(EParseErr::InvalidSyntax("trailing input".into()));
    }
    Ok(node)
}

struct EParser<'a> {
    parser: Parser<'a>,
}

impl<'a> EParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            parser: Parser::new(s),
        }
    }

    fn parse_node(&mut self) -> Result<ENode, EParseErr> {
        self.parser.skip_ws();
        if self.parser.peek_char() == Some('"') || self.parser.peek_char() == Some('\'') {
            return Ok(ENode::Str(self.parser.parse_quoted_string()?));
        }
        let name = self.parser.parse_identifier()?;
        self.parser.skip_ws();
        self.parser.expect('(')?;
        let args = self.parse_args()?;
        self.parser.expect(')')?;
        Ok(ENode::Call { name, args })
    }

    fn parse_args(&mut self) -> Result<Vec<ENode>, EParseErr> {
        let mut out = Vec::new();
        self.parser.skip_ws();
        if self.parser.peek_char() == Some(')') {
            return Ok(out);
        }
        loop {
            let node = self.parse_node()?;
            out.push(node);
            self.parser.skip_ws();
            if self.parser.consume_char(',') {
                self.parser.skip_ws();
                continue;
            }
            break;
        }
        Ok(out)
    }

    fn skip_ws(&mut self) {
        self.parser.skip_ws();
    }

    fn eof(&self) -> bool {
        self.parser.eof()
    }
}

/// Helper function to check argument count and return null if invalid
fn check_arg_count(args: &[ENode], expected: usize) -> bool {
    args.len() == expected
}

/// Helper function to extract string value from evaluated node
fn extract_string(value: Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s),
        _ => None,
    }
}

/// Evaluate AST node → Value
pub fn eval_ast(node: &ENode) -> Value {
    match node {
        ENode::Str(s) => Value::String(s.clone()),
        ENode::Call { name, args } => match name.as_str() {
            "from_json" => {
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
                if !check_arg_count(args, 1) {
                    return Value::Null;
                }
                first(&eval_ast(&args[0]))
            }
            "unique" => {
                if !check_arg_count(args, 1) {
                    return Value::Null;
                }
                unique(&eval_ast(&args[0]))
            }
            "or_default" => {
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
            _ => Value::Null,
        },
    }
}
