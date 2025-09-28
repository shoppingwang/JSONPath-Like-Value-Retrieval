use serde_json::Value;
use crate::filter::FilterExpr;
use crate::engine::JpOptions;

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
pub enum Segment {
    Root,        // $
    Key(String), // .foo or ['foo']
    Wildcard,    // .* or [*]
    Index(i64),  // [0]
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    }, // [start:end:step]
    Recursive,   // ..
    Filter(Box<FilterExpr>), // [?(expr)]
}

#[derive(Debug)]
pub enum ParseErr {
    InvalidSyntax(String),
}

/// Internal engine
pub fn from_value_with_opts(data: &Value, path: &str, opts: &JpOptions) -> Value {
    match parse_path(path) {
        Ok(ast) => {
            let refs = eval_path(data, &ast, opts);
            if refs.is_empty() {
                opts.default.clone().unwrap_or(Value::Null)
            } else {
                Value::Array(refs.into_iter().cloned().collect())
            }
        }
        Err(_) => opts.default.clone().unwrap_or(Value::Null),
    }
}

fn parse_path(input: &str) -> Result<Path, ParseErr> {
    let mut p = Parser::new(input);
    p.parse()
}

pub struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }

    fn parse(&mut self) -> Result<Path, ParseErr> {
        let mut segments = Vec::new();
        self.skip_ws();
        if !self.consume_char('$') {
            return Err(ParseErr::InvalidSyntax("path must start with `$`".into()));
        }
        segments.push(Segment::Root);

        while !self.eof() {
            self.skip_ws();
            if self.peek_str("..") {
                self.i += 2;
                segments.push(Segment::Recursive);
                continue;
            }
            if self.consume_char('.') {
                if self.consume_char('*') {
                    segments.push(Segment::Wildcard);
                    continue;
                }
                let key = self.parse_identifier()?;
                segments.push(Segment::Key(key));
                continue;
            }
            if self.consume_char('[') {
                self.skip_ws();
                if self.consume_char('*') {
                    self.expect(']')?;
                    segments.push(Segment::Wildcard);
                    continue;
                }
                if self.peek_char() == Some('?') {
                    self.i += 1;
                    self.expect('(')?;
                    let expr = crate::filter::parse_filter_or(self)?;
                    self.expect(')')?;
                    self.expect(']')?;
                    segments.push(Segment::Filter(Box::new(expr)));
                    continue;
                }
                if self.peek_char() == Some('\'') || self.peek_char() == Some('"') {
                    let key = self.parse_quoted_string()?;
                    self.expect(']')?;
                    segments.push(Segment::Key(key));
                    continue;
                }
                let slice_content = self.capture_until(']')?;
                self.expect(']')?;
                if slice_content.contains(':') {
                    let parts: Vec<&str> = slice_content.split(':').collect();
                    if parts.len() > 3 {
                        return Err(ParseErr::InvalidSyntax("slice too many components".into()));
                    }
                    let parse_opt_i64 = |s: &str| -> Result<Option<i64>, ParseErr> {
                        let t = s.trim();
                        if t.is_empty() {
                            Ok(None)
                        } else {
                            t.parse::<i64>()
                                .map(Some)
                                .map_err(|_| ParseErr::InvalidSyntax("bad slice number".into()))
                        }
                    };
                    let start = parse_opt_i64(parts.get(0).copied().unwrap_or(""))?;
                    let end = parse_opt_i64(parts.get(1).copied().unwrap_or(""))?;
                    let step = parse_opt_i64(parts.get(2).copied().unwrap_or(""))?;
                    segments.push(Segment::Slice { start, end, step });
                } else {
                    let mut tmp = Parser::new(slice_content);
                    let idx = tmp.parse_int()?;
                    segments.push(Segment::Index(idx));
                }
                continue;
            }
            break;
        }
        Ok(Path { segments })
    }

    // Helper methods for parsing
    pub fn parse_identifier(&mut self) -> Result<String, ParseErr> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return Err(ParseErr::InvalidSyntax("identifier expected".into()));
        }
        Ok(self.s[start..self.i].to_string())
    }

    pub fn parse_int(&mut self) -> Result<i64, ParseErr> {
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
            return Err(ParseErr::InvalidSyntax("expected integer".into()));
        }
        self.s[start..self.i]
            .parse::<i64>()
            .map_err(|_| ParseErr::InvalidSyntax("bad integer".into()))
    }

    pub fn parse_number_literal(&mut self) -> Result<Value, ParseErr> {
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
            return Err(ParseErr::InvalidSyntax("number expected".into()));
        }
        if s.contains('.') {
            let f: f64 = s
                .parse()
                .map_err(|_| ParseErr::InvalidSyntax("bad float".into()))?;
            Ok(Value::from(f))
        } else {
            let i: i64 = s
                .parse()
                .map_err(|_| ParseErr::InvalidSyntax("bad int".into()))?;
            Ok(Value::from(i))
        }
    }

    pub fn parse_quoted_string(&mut self) -> Result<String, ParseErr> {
        let quote = self
            .peek_char()
            .ok_or_else(|| ParseErr::InvalidSyntax("string".into()))?;
        if quote != '\'' && quote != '"' {
            return Err(ParseErr::InvalidSyntax("expected quoted string".into()));
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
        Err(ParseErr::InvalidSyntax("unterminated string".into()))
    }

    pub fn capture_until(&mut self, end: char) -> Result<&'a str, ParseErr> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == end {
                break;
            }
            self.i += 1;
        }
        if self.peek_char() != Some(end) {
            return Err(ParseErr::InvalidSyntax(format!("expected '{end}'")));
        }
        Ok(&self.s[start..self.i])
    }

    pub fn expect(&mut self, c: char) -> Result<(), ParseErr> {
        if self.consume_char(c) {
            Ok(())
        } else {
            Err(ParseErr::InvalidSyntax(format!("expected '{}'", c)))
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

fn eval_path<'a>(root: &'a Value, path: &Path, opts: &JpOptions) -> Vec<&'a Value> {
    let mut current: Vec<&Value> = vec![root];
    for seg in &path.segments {
        current = match seg {
            Segment::Root => vec![root],
            Segment::Key(k) => current
                .into_iter()
                .flat_map(|v| match v {
                    Value::Object(map) => map.get(k).into_iter().collect(),
                    _ => Vec::new(),
                })
                .collect(),
            Segment::Index(i) => {
                if *i < 0 {
                    Vec::new()
                } else {
                    let idx = *i as usize;
                    current
                        .into_iter()
                        .flat_map(|v| match v {
                            Value::Array(arr) => arr.get(idx).into_iter().collect(),
                            _ => Vec::new(),
                        })
                        .collect()
                }
            }
            Segment::Slice { start, end, step } => current
                .into_iter()
                .flat_map(|v| match v {
                    Value::Array(arr) => slice_array(arr, *start, *end, *step),
                    _ => Vec::new(),
                })
                .collect(),
            Segment::Wildcard => current
                .into_iter()
                .flat_map(|v| match v {
                    Value::Array(arr) => arr.iter().collect(),
                    Value::Object(map) => map.values().collect(),
                    _ => Vec::new(),
                })
                .collect(),
            Segment::Recursive => current
                .into_iter()
                .flat_map(|v| {
                    let mut out = Vec::new();
                    recurse_collect(v, &mut out);
                    out
                })
                .collect(),
            Segment::Filter(expr) => current
                .into_iter()
                .flat_map(|v| match v {
                    Value::Array(arr) => arr.iter().collect(),
                    _ => vec![v],
                })
                .filter(|v| crate::filter::eval_filter(expr, v, opts))
                .collect(),
        };
    }
    current
}

fn slice_array<'a>(
    arr: &'a Vec<Value>,
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
) -> Vec<&'a Value> {
    let n = arr.len() as i64;
    let step = step.unwrap_or(1);
    if step == 0 {
        return Vec::new();
    }
    let norm = |i: i64| -> i64 {
        if i < 0 {
            (n + i).clamp(0, n)
        } else {
            i.clamp(0, n)
        }
    };
    let (mut lo, mut hi) = (start.unwrap_or(0), end.unwrap_or(n));
    lo = norm(lo);
    hi = norm(hi);
    let mut out = Vec::new();
    if step > 0 {
        let mut i = lo;
        while i < hi {
            if let Some(v) = arr.get(i as usize) {
                out.push(v);
            }
            i += step;
        }
    } else {
        if hi == 0 {
            return out;
        }
        let mut i = (hi - 1).clamp(0, n - 1);
        while i >= lo {
            if let Some(v) = arr.get(i as usize) {
                out.push(v);
            }
            i += step;
            if i < 0 {
                break;
            }
        }
    }
    out
}

fn recurse_collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(v);
    match v {
        Value::Array(arr) => {
            for elt in arr {
                recurse_collect(elt, out);
            }
        }
        Value::Object(map) => {
            for elt in map.values() {
                recurse_collect(elt, out);
            }
        }
        _ => {}
    }
}
