use crate::{first, from_json, or_default, unique};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ENode {
    Call { name: String, args: Vec<ENode> },
    Str(String),
}

#[derive(Debug)]
pub enum EParseErr {
    Invalid(String),
}

pub fn parse_expr(input: &str) -> Result<ENode, EParseErr> {
    let mut p = EParser::new(input);
    let node = p.parse_node()?;
    p.skip_ws();
    if !p.eof() {
        return Err(EParseErr::Invalid("trailing input".into()));
    }
    Ok(node)
}

struct EParser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> EParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }

    fn parse_node(&mut self) -> Result<ENode, EParseErr> {
        self.skip_ws();
        if self.peek_char() == Some('"') || self.peek_char() == Some('\'') {
            return Ok(ENode::Str(self.parse_string()?));
        }
        let name = self.parse_ident()?;
        self.skip_ws();
        self.expect('(')?;
        let args = self.parse_args()?;
        self.expect(')')?;
        Ok(ENode::Call { name, args })
    }

    fn parse_args(&mut self) -> Result<Vec<ENode>, EParseErr> {
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek_char() == Some(')') {
            return Ok(out);
        }
        loop {
            let node = self.parse_node()?;
            out.push(node);
            self.skip_ws();
            if self.consume_char(',') {
                self.skip_ws();
                continue;
            }
            break;
        }
        Ok(out)
    }

    fn parse_ident(&mut self) -> Result<String, EParseErr> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return Err(EParseErr::Invalid("identifier expected".into()));
        }
        Ok(self.s[start..self.i].to_string())
    }

    fn parse_string(&mut self) -> Result<String, EParseErr> {
        let quote = self
            .peek_char()
            .ok_or_else(|| EParseErr::Invalid("string".into()))?;
        if quote != '"' && quote != '\'' {
            return Err(EParseErr::Invalid("quoted string expected".into()));
        }
        self.i += 1;
        let mut out = String::new();
        while let Some(c) = self.peek_char() {
            self.i += 1;
            if c == quote {
                return Ok(out);
            }
            if c == '\\' {
                if let Some(nc) = self.peek_char() {
                    self.i += 1;
                    match nc {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        '\'' => out.push('\''),
                        _ => {
                            out.push('\\');
                            out.push(nc);
                        }
                    }
                } else {
                    break;
                }
            } else {
                out.push(c);
            }
        }
        Err(EParseErr::Invalid("unterminated string".into()))
    }

    fn expect(&mut self, c: char) -> Result<(), EParseErr> {
        if self.consume_char(c) {
            Ok(())
        } else {
            Err(EParseErr::Invalid(format!("expected '{}'", c)))
        }
    }

    fn consume_char(&mut self, c: char) -> bool {
        if self.peek_char() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn eof(&self) -> bool {
        self.i >= self.s.len()
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
