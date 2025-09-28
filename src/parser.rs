use serde_json::Value;

/// Represents possible errors that can occur during parsing.
#[derive(Debug)]
pub enum ParseError {
    /// Indicates invalid syntax with a message describing the error.
    InvalidSyntax(String),
}

/// Allows conversion from a `String` to a `ParseError`.
impl From<String> for ParseError {
    fn from(msg: String) -> Self {
        ParseError::InvalidSyntax(msg)
    }
}

/// Parser struct for parsing strings, tracking the current position.
pub struct Parser<'a> {
    /// The input string to parse.
    s: &'a str,
    /// The current index in the input string.
    i: usize,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given string.
    pub fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }

    /// Parses an identifier (alphanumeric or underscore).
    /// Returns the identifier as a `String` or an error if not found.
    pub fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let start = self.i;
        // Loop through characters as long as they are valid identifier characters
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() {
                self.i += 1;
            } else {
                break;
            }
        }
        // If no valid identifier was found, return an error
        if self.i == start {
            return Err(ParseError::InvalidSyntax("identifier expected".into()));
        }
        // Return the identifier substring
        Ok(self.s[start..self.i].to_string())
    }

    /// Parses an integer, handling optional leading minus.
    /// Returns the integer or an error if parsing fails.
    pub fn parse_int(&mut self) -> Result<i64, ParseError> {
        let start = self.i;
        // Check for optional minus sign
        if self.peek_char() == Some('-') {
            self.i += 1;
        }
        // Consume all digit characters
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        // If no digits found or only a minus sign, return error
        if self.i == start || (self.i == start + 1 && &self.s[start..self.i] == "-") {
            return Err(ParseError::InvalidSyntax("expected integer".into()));
        }
        // Parse the substring as i64
        self.s[start..self.i]
            .parse::<i64>()
            .map_err(|_| ParseError::InvalidSyntax("bad integer".into()))
    }

    /// Parses a number literal (integer or float).
    /// Returns a `serde_json::Value` containing the number.
    pub fn parse_number_literal(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        // Check for optional minus sign
        if self.peek_char() == Some('-') {
            self.i += 1;
        }
        // Consume all digit characters before decimal point
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        // If decimal point is present, parse fractional part
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
        // If nothing was parsed, return error
        if s.is_empty() {
            return Err(ParseError::InvalidSyntax("number expected".into()));
        }
        // Parse as float if decimal point is present, otherwise as integer
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

    /// Parses a quoted string, handling escape sequences.
    /// Supports both single and double quotes.
    pub fn parse_quoted_string(&mut self) -> Result<String, ParseError> {
        // Get the quote character (either ' or ")
        let quote = self
            .peek_char()
            .ok_or_else(|| ParseError::InvalidSyntax("string".into()))?;
        if quote != '\'' && quote != '"' {
            return Err(ParseError::InvalidSyntax("expected quoted string".into()));
        }
        self.i += 1; // Consume the opening quote
        let mut out = String::new();
        // Loop until closing quote or end of input
        while let Some(c) = self.peek_char() {
            self.i += 1;
            if c == quote {
                // Found closing quote
                return Ok(out);
            }
            if c == '\\' {
                // Handle escape sequences
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
                            // Unknown escape, keep as-is
                            out.push('\\');
                            out.push(nc);
                        }
                    }
                } else {
                    // Unterminated escape sequence
                    break;
                }
            } else {
                // Regular character
                out.push(c);
            }
        }
        // If loop ends without finding closing quote, return error
        Err(ParseError::InvalidSyntax("unterminated string".into()))
    }

    /// Captures a substring until the specified end character is found.
    /// Returns the substring or an error if the end character is missing.
    pub fn capture_until(&mut self, end: char) -> Result<&'a str, ParseError> {
        let start = self.i;
        // Loop until end character is found
        while let Some(c) = self.peek_char() {
            if c == end {
                break;
            }
            self.i += 1;
        }
        // If end character not found, return error
        if self.peek_char() != Some(end) {
            return Err(ParseError::InvalidSyntax(format!("expected '{end}'")));
        }
        // Return the captured substring
        Ok(&self.s[start..self.i])
    }

    /// Expects the next character to match `c`, consuming it if so.
    /// Returns an error if the character does not match.
    pub fn expect(&mut self, c: char) -> Result<(), ParseError> {
        if self.consume_char(c) {
            Ok(())
        } else {
            Err(ParseError::InvalidSyntax(format!("expected '{}'", c)))
        }
    }

    /// Consumes the next character if it matches `c`.
    /// Returns true if consumed, false otherwise.
    pub fn consume_char(&mut self, c: char) -> bool {
        if self.peek_char() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    /// Peeks at the next character without consuming it.
    /// Returns `Some(char)` if available, otherwise `None`.
    pub fn peek_char(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    /// Checks if the next substring matches the given literal.
    /// Returns true if it matches, false otherwise.
    pub fn peek_str(&self, lit: &str) -> bool {
        self.s[self.i..].starts_with(lit)
    }

    /// Skips whitespace characters.
    /// Advances the index past any whitespace.
    pub fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    /// Checks if the parser has reached the end of the input.
    /// Returns true if at end, false otherwise.
    pub fn eof(&self) -> bool {
        self.i >= self.s.len()
    }
}
