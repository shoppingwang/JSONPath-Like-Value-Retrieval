// src/parser.rs
use serde_json::Value;

#[derive(Debug)]
pub enum ParseError {
    InvalidSyntax(String),
}

impl From<String> for ParseError {
    fn from(msg: String) -> Self {
        ParseError::InvalidSyntax(msg)
    }
}

pub struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }

    pub fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return Err(ParseError::InvalidSyntax("identifier expected".into()));
        }
        Ok(self.s[start..self.i].to_string())
    }

    pub fn parse_int(&mut self) -> Result<i64, ParseError> {
        let start = self.i;
        if self.peek_char() == Some('-') {
            self.i += 1;
        }
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start || (self.i == start + 1 && &self.s[start..self.i] == "-") {
            return Err(ParseError::InvalidSyntax("expected integer".into()));
        }
        self.s[start..self.i]
            .parse::<i64>()
            .map_err(|_| ParseError::InvalidSyntax("bad integer".into()))
    }

    pub fn parse_number_literal(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.peek_char() == Some('-') {
            self.i += 1;
        }
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.peek_char() == Some('.') {
            self.i += 1;
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    self.i += 1;
                } else {
                    break;
                }
            }
        }
        let s = &self.s[start..self.i];
        if s.is_empty() {
            return Err(ParseError::InvalidSyntax("number expected".into()));
        }
        if s.contains('.') {
            let f: f64 = s
                .parse()
                .map_err(|_| ParseError::InvalidSyntax("bad float".into()))?;
            Ok(Value::from(f))
        } else {
            let i: i64 = s
                .parse()
                .map_err(|_| ParseError::InvalidSyntax("bad int".into()))?;
            Ok(Value::from(i))
        }
    }

    pub fn parse_quoted_string(&mut self) -> Result<String, ParseError> {
        let quote = self
            .peek_char()
            .ok_or_else(|| ParseError::InvalidSyntax("string".into()))?;
        if quote != '\'' && quote != '"' {
            return Err(ParseError::InvalidSyntax("expected quoted string".into()));
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
        Err(ParseError::InvalidSyntax("unterminated string".into()))
    }

    pub fn capture_until(&mut self, end: char) -> Result<&'a str, ParseError> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == end {
                break;
            }
            self.i += 1;
        }
        if self.peek_char() != Some(end) {
            return Err(ParseError::InvalidSyntax(format!("expected '{end}'")));
        }
        Ok(&self.s[start..self.i])
    }

    pub fn expect(&mut self, c: char) -> Result<(), ParseError> {
        if self.consume_char(c) {
            Ok(())
        } else {
            Err(ParseError::InvalidSyntax(format!("expected '{}'", c)))
        }
    }

    pub fn consume_char(&mut self, c: char) -> bool {
        if self.peek_char() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    pub fn peek_char(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    pub fn peek_str(&self, lit: &str) -> bool {
        self.s[self.i..].starts_with(lit)
    }

    pub fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    pub fn eof(&self) -> bool {
        self.i >= self.s.len()
    }
}
