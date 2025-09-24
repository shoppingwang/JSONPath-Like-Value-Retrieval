use itertools::Itertools;
use serde_json::{json, Value};

/// --------- JSONPath Evaluator Struct ---------

/// Main struct for evaluating JSONPath-like queries over JSON data.
pub struct JsonPath<'a> {
    data: &'a Value,
    options: JpOptions,
}

impl<'a> JsonPath<'a> {
    /// Create a new JsonPath evaluator for the given data.
    pub fn new(data: &'a Value) -> Self {
        Self {
            data,
            options: JpOptions::default(),
        }
    }

    /// Set custom options for evaluation.
    pub fn with_options(mut self, options: JpOptions) -> Self {
        self.options = options;
        self
    }

    /// Evaluate a JSONPath-like expression and return the result as a JSON Value.
    pub fn query(&self, path: &str) -> Value {
        match parse_path(path) {
            Ok(ast) => {
                let refs = eval_path(self.data, &ast, &self.options);
                if refs.is_empty() {
                    self.options.default.clone().unwrap_or(Value::Null)
                } else {
                    Value::Array(refs.into_iter().cloned().collect())
                }
            }
            Err(_) => self.options.default.clone().unwrap_or(Value::Null),
        }
    }

    /// Return the first element from the result array, or null.
    pub fn first(&self, vals: &Value) -> Value {
        match vals {
            Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }

    /// Deduplicate an array result via deep JSON equality.
    pub fn unique(&self, vals: &Value) -> Value {
        match vals {
            Value::Array(a) => {
                let dedup = a.iter()
                    .map(|v| v.clone())
                    .unique_by(|x| serde_json::to_string(x).unwrap_or_default())
                    .collect::<Vec<_>>();
                Value::Array(dedup)
            }
            _ => vals.clone(),
        }
    }

    /// If result is empty array or null, return default; else return result unchanged.
    pub fn or_default(&self, vals: &Value, default: Value) -> Value {
        match vals {
            Value::Null => default,
            Value::Array(a) if a.is_empty() => default,
            _ => vals.clone(),
        }
    }
}

/// --------- Options and Comparison Modes ---------

/// Options for JSONPath evaluation.
#[derive(Default, Clone, Debug)]
pub struct JpOptions {
    /// Default value to return if no matches or invalid path.
    pub default: Option<Value>,
    /// Comparison mode for string equality.
    pub cmp: CmpMode,
}

/// String comparison modes.
#[derive(Clone, Copy, Debug)]
pub enum CmpMode {
    CaseSensitive,
    CaseFoldLower,
    CaseFoldUpper,
}
impl Default for CmpMode {
    fn default() -> Self { CmpMode::CaseSensitive }
}

/// --------- Path AST ---------

#[derive(Debug, Clone)]
struct Path { segments: Vec<Segment> }

#[derive(Debug, Clone)]
enum Segment {
    Root,            // $
    Key(String),     // .foo or ['foo']
    Wildcard,        // .* or [*]
    Index(i64),      // [0]
    Slice { start: Option<i64>, end: Option<i64>, step: Option<i64> }, // [start:end:step]
    Recursive,       // ..
    Filter(Box<FilterExpr>), // [?(expr)]
}

#[derive(Debug, Clone)]
enum FilterExpr {
    // comparisons
    Eq(Operand, Operand),
    Ne(Operand, Operand),
    Lt(Operand, Operand),
    Lte(Operand, Operand),
    Gt(Operand, Operand),
    Gte(Operand, Operand),
    // logical
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    // grouping / truthiness of operand (exists / non-null / non-false)
    Truthy(Operand),
}

#[derive(Debug, Clone)]
enum Operand {
    CurrentPath(Vec<PathToken>), // @.a['b'][0]
    Literal(Value),              // "abc", 123, true/false/null
    Lower(Box<Operand>),
    Upper(Box<Operand>),
    Length(Box<Operand>),
}

#[derive(Debug, Clone)]
enum PathToken {
    Key(String),
    Index(i64),
    Wildcard,
}

/// --------- Parser ---------

#[derive(Debug)]
enum ParseErr { InvalidSyntax(String) }

/// Parse a JSONPath-like string into an AST.
fn parse_path(input: &str) -> Result<Path, ParseErr> {
    let mut p = Parser::new(input);
    p.parse()
}

/// Internal parser struct for JSONPath expressions.
struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self { Self { s, i: 0 } }

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
                    self.i += 1; // '?'
                    self.expect('(')?;
                    let expr = self.parse_filter_or()?; // full precedence parser
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

                // Decide: index or slice
                let slice_content = self.capture_until(']')?;
                self.expect(']')?;
                if slice_content.contains(':') {
                    let parts: Vec<&str> = slice_content.split(':').collect();
                    if parts.len() > 3 {
                        return Err(ParseErr::InvalidSyntax("slice has too many components".into()));
                    }
                    let parse_opt_i64 = |s: &str| -> Result<Option<i64>, ParseErr> {
                        let t = s.trim();
                        if t.is_empty() { Ok(None) } else {
                            t.parse::<i64>().map(Some).map_err(|_| ParseErr::InvalidSyntax("bad slice number".into()))
                        }
                    };
                    let start = parse_opt_i64(parts.get(0).copied().unwrap_or(""))?;
                    let end   = parse_opt_i64(parts.get(1).copied().unwrap_or(""))?;
                    let step  = parse_opt_i64(parts.get(2).copied().unwrap_or(""))?;
                    segments.push(Segment::Slice { start, end, step });
                } else {
                    // index
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

    // --- precedence: Or -> And -> Not -> Compare/Truthy
    fn parse_filter_or(&mut self) -> Result<FilterExpr, ParseErr> {
        let mut left = self.parse_filter_and()?;
        loop {
            self.skip_ws();
            if self.peek_str("||") {
                self.i += 2;
                let right = self.parse_filter_and()?;
                left = FilterExpr::Or(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_filter_and(&mut self) -> Result<FilterExpr, ParseErr> {
        let mut left = self.parse_filter_not()?;
        loop {
            self.skip_ws();
            if self.peek_str("&&") {
                self.i += 2;
                let right = self.parse_filter_not()?;
                left = FilterExpr::And(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_filter_not(&mut self) -> Result<FilterExpr, ParseErr> {
        self.skip_ws();
        if self.consume_char('!') {
            let inner = self.parse_filter_not()?;
            Ok(FilterExpr::Not(Box::new(inner)))
        } else {
            self.parse_filter_compare()
        }
    }

    fn parse_filter_compare(&mut self) -> Result<FilterExpr, ParseErr> {
        self.skip_ws();
        // Allow parentheses inside filter expressions
        if self.consume_char('(') {
            let inner = self.parse_filter_or()?;
            self.expect(')')?;
            return Ok(inner);
        }

        // Try: <operand> (op) <operand>
        let left_opnd = self.parse_operand()?;
        self.skip_ws();

        // Peek operator
        let op = if self.peek_str("==") { self.i += 2; Some("==") }
        else if self.peek_str("!=") { self.i += 2; Some("!=") }
        else if self.peek_str("<=") { self.i += 2; Some("<=") }
        else if self.peek_str(">=") { self.i += 2; Some(">=") }
        else if self.peek_char() == Some('<') { self.i += 1; Some("<") }
        else if self.peek_char() == Some('>') { self.i += 1; Some(">") }
        else { None };

        if let Some(op) = op {
            self.skip_ws();
            let right_opnd = self.parse_operand()?;
            return Ok(match op {
                "==" => FilterExpr::Eq(left_opnd, right_opnd),
                "!=" => FilterExpr::Ne(left_opnd, right_opnd),
                "<"  => FilterExpr::Lt(left_opnd, right_opnd),
                "<=" => FilterExpr::Lte(left_opnd, right_opnd),
                ">"  => FilterExpr::Gt(left_opnd, right_opnd),
                ">=" => FilterExpr::Gte(left_opnd, right_opnd),
                _ => unreachable!(),
            });
        }
        // No operator: interpret as truthiness check (e.g., @.foo)
        Ok(FilterExpr::Truthy(left_opnd))
    }

    fn parse_operand(&mut self) -> Result<Operand, ParseErr> {
        self.skip_ws();
        if self.peek_char() == Some('"') || self.peek_char() == Some('\'') {
            return Ok(Operand::Literal(Value::String(self.parse_quoted_string()?)));
        }
        if self.peek_str("true")  { self.i += 4; return Ok(Operand::Literal(Value::Bool(true))); }
        if self.peek_str("false") { self.i += 5; return Ok(Operand::Literal(Value::Bool(false))); }
        if self.peek_str("null")  { self.i += 4; return Ok(Operand::Literal(Value::Null)); }

        if self.peek_str("lower(") {
            self.i += "lower(".len();
            let inner = self.parse_operand()?;
            self.expect(')')?;
            return Ok(Operand::Lower(Box::new(inner)));
        }
        if self.peek_str("upper(") {
            self.i += "upper(".len();
            let inner = self.parse_operand()?;
            self.expect(')')?;
            return Ok(Operand::Upper(Box::new(inner)));
        }
        if self.peek_str("length(") {
            self.i += "length(".len();
            let inner = self.parse_operand()?;
            self.expect(')')?;
            return Ok(Operand::Length(Box::new(inner)));
        }
        if self.peek_char() == Some('@') {
            self.i += 1; // '@'
            let mut tokens = Vec::new();
            loop {
                self.skip_ws();
                if self.consume_char('.') {
                    if self.consume_char('*') { tokens.push(PathToken::Wildcard); continue; }
                    let k = self.parse_identifier()?;
                    tokens.push(PathToken::Key(k));
                    continue;
                } else if self.consume_char('[') {
                    if self.consume_char('*') {
                        self.expect(']')?;
                        tokens.push(PathToken::Wildcard);
                        continue;
                    }
                    if self.peek_char() == Some('"') || self.peek_char() == Some('\'') {
                        let k = self.parse_quoted_string()?;
                        self.expect(']')?;
                        tokens.push(PathToken::Key(k));
                        continue;
                    }
                    // allow only simple index in @-paths for now
                    let idx_content = self.capture_until(']')?;
                    self.expect(']')?;
                    let mut tmp = Parser::new(idx_content);
                    // slice inside @ not supported; fallback: index
                    let idx = tmp.parse_int()?;
                    tokens.push(PathToken::Index(idx));
                    continue;
                }
                break;
            }
            return Ok(Operand::CurrentPath(tokens));
        }
        // number
        if self.peek_char().map(|c| c == '-' || c.is_ascii_digit()).unwrap_or(false) {
            let n = self.parse_number_literal()?;
            return Ok(Operand::Literal(n));
        }

        Err(ParseErr::InvalidSyntax("invalid operand".into()))
    }

    fn parse_identifier(&mut self) -> Result<String, ParseErr> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() { self.i += 1; } else { break; }
        }
        if self.i == start { return Err(ParseErr::InvalidSyntax("expected identifier".into())); }
        Ok(self.s[start..self.i].to_string())
    }

    fn parse_int(&mut self) -> Result<i64, ParseErr> {
        let start = self.i;
        if self.peek_char() == Some('-') { self.i += 1; }
        while let Some(c) = self.peek_char() { if c.is_ascii_digit() { self.i += 1; } else { break; } }
        if self.i == start || (self.i == start + 1 && &self.s[start..self.i] == "-") {
            return Err(ParseErr::InvalidSyntax("expected integer".into()));
        }
        self.s[start..self.i].parse::<i64>().map_err(|_| ParseErr::InvalidSyntax("bad integer".into()))
    }

    fn parse_number_literal(&mut self) -> Result<Value, ParseErr> {
        let start = self.i;
        if self.peek_char() == Some('-') { self.i += 1; }
        while let Some(c) = self.peek_char() { if c.is_ascii_digit() { self.i += 1; } else { break; } }
        if self.peek_char() == Some('.') {
            self.i += 1;
            while let Some(c) = self.peek_char() { if c.is_ascii_digit() { self.i += 1; } else { break; } }
        }
        let s = &self.s[start..self.i];
        if s.is_empty() { return Err(ParseErr::InvalidSyntax("number expected".into())); }
        if s.contains('.') {
            let f: f64 = s.parse().map_err(|_| ParseErr::InvalidSyntax("bad float".into()))?;
            Ok(Value::from(f))
        } else {
            let i: i64 = s.parse().map_err(|_| ParseErr::InvalidSyntax("bad int".into()))?;
            Ok(Value::from(i))
        }
    }

    fn parse_quoted_string(&mut self) -> Result<String, ParseErr> {
        let quote = self.peek_char().ok_or_else(|| ParseErr::InvalidSyntax("string".into()))?;
        if quote != '\'' && quote != '"' { return Err(ParseErr::InvalidSyntax("expected quoted string".into())); }
        self.i += 1;
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == quote {
                let s = &self.s[start..self.i];
                self.i += 1;
                return Ok(s.to_string());
            }
            // TODO: escapes
            self.i += 1;
        }
        Err(ParseErr::InvalidSyntax("unterminated string".into()))
    }

    fn capture_until(&mut self, end: char) -> Result<&'a str, ParseErr> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c == end { break; }
            self.i += 1;
        }
        if self.peek_char() != Some(end) {
            return Err(ParseErr::InvalidSyntax(format!("expected '{}'", end)));
        }
        Ok(&self.s[start..self.i])
    }

    fn expect(&mut self, c: char) -> Result<(), ParseErr> {
        if self.consume_char(c) { Ok(()) } else { Err(ParseErr::InvalidSyntax(format!("expected '{}'", c))) }
    }
    fn consume_char(&mut self, c: char) -> bool {
        if self.peek_char() == Some(c) { self.i += 1; true } else { false }
    }
    fn peek_char(&self) -> Option<char> { self.s[self.i..].chars().next() }
    fn peek_str(&self, lit: &str) -> bool { self.s[self.i..].starts_with(lit) }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() { self.i += 1; } else { break; }
        }
    }
    fn eof(&self) -> bool { self.i >= self.s.len() }
}

/// --------- Evaluator ---------

/// Evaluate the parsed path AST over the given JSON data.
fn eval_path<'a>(root: &'a Value, path: &Path, opts: &JpOptions) -> Vec<&'a Value> {
    use Segment::*;
    let mut current: Vec<&Value> = vec![root];

    for seg in &path.segments {
        match seg {
            Segment::Root => {
                current = vec![root];
                println!("At root: {:?}", current);
            }
            Segment::Key(k) => {
                println!("Key: {:?}", current);
                current = current.into_iter().flat_map(|v| match v {
                    Value::Object(map) => map.get(k).into_iter().collect(),
                    _ => Vec::new(),
                }).collect();
                println!("At key '{}': {:?}", k, current);
            }
            Segment::Index(i) => {
                if *i < 0 { current.clear(); continue; }
                let idx = *i as usize;
                current = current.into_iter().flat_map(|v| match v {
                    Value::Array(arr) => arr.get(idx).into_iter().collect(),
                    _ => Vec::new(),
                }).collect();
                println!("At index [{}]: {:?}", i, current);
            }
            Segment::Slice { start, end, step } => {
                current = current.into_iter().flat_map(|v| match v {
                    Value::Array(arr) => slice_array(arr, *start, *end, *step),
                    _ => Vec::new(),
                }).collect();
                println!("At slice [{:?}:{:?}:{:?}]: {:?}", start, end, step, current);
            }
            Segment::Wildcard => {
                current = current.into_iter().flat_map(|v| match v {
                    Value::Array(arr) => arr.iter().collect(),
                    Value::Object(map) => map.values().collect(),
                    _ => Vec::new(),
                }).collect();
                println!("At wildcard: {:?}", current);
            }
            Segment::Recursive => {
                current = current.into_iter().flat_map(|v| {
                    let mut out = Vec::new();
                    recurse_collect(v, &mut out);
                    out
                }).collect();
                println!("At recursive descent: {:?}", current);
            }
            Segment::Filter(expr) => {
                println!("Filter: {:?} ==== {:?}", expr, current);
                current = current.into_iter()
                    .flat_map(|v| match v {
                        Value::Array(arr) => arr.iter().collect(),
                        _ => vec![v],
                    })
                    .filter(|v| eval_filter(expr, v, opts))
                    .collect();
                println!("After filter: {:?}", current);
            }
        }
    }
    current
}

/// Helper for array slicing logic.
fn slice_array<'a>(arr: &'a Vec<Value>, start: Option<i64>, end: Option<i64>, step: Option<i64>) -> Vec<&'a Value> {
    let n = arr.len() as i64;
    let step = step.unwrap_or(1);
    if step == 0 { return Vec::new(); }
    let norm = |i: i64| -> i64 {
        if i < 0 { (n + i).clamp(0, n) } else { i.clamp(0, n) }
    };
    let (mut lo, mut hi) = (start.unwrap_or(if step > 0 { 0 } else { n - 1 }), end.unwrap_or(if step > 0 { n } else { -1 }));
    lo = norm(lo);
    hi = norm(hi);
    let mut out = Vec::new();
    if step > 0 {
        let mut i = lo;
        while i < hi {
            if let Some(v) = arr.get(i as usize) { out.push(v); }
            i += step;
        }
    } else {
        let mut i = hi;
        while i >= lo && i < n {
            if let Some(v) = arr.get(i as usize) { out.push(v); }
            if i == 0 { break; }
            i += step; // step is negative
        }
    }
    out
}

/// Recursively collect all values for recursive descent.
fn recurse_collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(v);
    match v {
        Value::Array(arr) => for elt in arr { recurse_collect(elt, out); }
        Value::Object(map) => for elt in map.values() { recurse_collect(elt, out); }
        _ => {}
    }
}

/// Evaluate a filter expression for a given value.
fn eval_filter(expr: &FilterExpr, current: &Value, opts: &JpOptions) -> bool {
    println!("eval_filter({:?}, === {:?})", expr, current);
    match expr {
        FilterExpr::Eq(a, b)  => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o == 0),
        FilterExpr::Ne(a, b)  => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o != 0),
        FilterExpr::Lt(a, b)  => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o <  0),
        FilterExpr::Lte(a, b) => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o <= 0),
        FilterExpr::Gt(a, b)  => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o >  0),
        FilterExpr::Gte(a, b) => cmp_values(&eval_operand(a, current, opts), &eval_operand(b, current, opts), opts, |o| o >= 0),
        FilterExpr::And(l, r) => eval_filter(l, current, opts) && eval_filter(r, current, opts),
        FilterExpr::Or(l, r)  => eval_filter(l, current, opts) || eval_filter(r, current, opts),
        FilterExpr::Not(inner)=> !eval_filter(inner, current, opts),
        FilterExpr::Truthy(op)=> truthy(&eval_operand(op, current, opts)),
    }
}

/// Determine truthiness of a JSON value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Evaluate an operand in a filter expression.
fn eval_operand(op: &Operand, current: &Value, opts: &JpOptions) -> Value {
    println!("eval_operand({:?}, {:?})", op, current);
    match op {
        Operand::Literal(v) => v.clone(),
        Operand::Lower(inner) => {
            let v = eval_operand(inner, current, opts);
            if let Some(s) = v.as_str() { Value::String(s.to_lowercase()) } else { v }
        }
        Operand::Upper(inner) => {
            let v = eval_operand(inner, current, opts);
            if let Some(s) = v.as_str() { Value::String(s.to_uppercase()) } else { v }
        }
        Operand::Length(inner) => {
            let v = eval_operand(inner, current, opts);
            let len = match v {
                Value::Array(ref a) => a.len() as i64,
                Value::Object(ref m) => m.len() as i64,
                Value::String(ref s) => s.chars().count() as i64,
                _ => 0,
            };
            Value::from(len)
        }
        Operand::CurrentPath(tokens) => {
            let mut nodes = vec![current];
            println!("eval_operand({:?}, {:?})", tokens, current);
            for t in tokens {
                nodes = match t {
                    PathToken::Key(k) =>
                        nodes.into_iter().flat_map(|n| match n {
                        Value::Object(m) => {
                            println!("Key: {:?}, Object: {:?}", k, m);
                            m.get(k).into_iter().collect()
                        },
                        _ => {
                            println!("Key: {:?}, Non-Object: {:?}", k, n);
                            Vec::new()
                        },
                    }).collect(),
                    PathToken::Index(i) => {
                        if *i < 0 { Vec::new() } else {
                            let idx = *i as usize;
                            nodes.into_iter().flat_map(|n| match n {
                                Value::Array(a) => a.get(idx).into_iter().collect(),
                                _ => Vec::new(),
                            }).collect()
                        }
                    }
                    PathToken::Wildcard => nodes.into_iter().flat_map(|n| match n {
                        Value::Array(a) => a.iter().collect(),
                        Value::Object(m) => m.values().collect(),
                        _ => Vec::new(),
                    }).collect(),
                }
            }
            // For now, take first node if multiple (common JSONPath filter behavior
            // typically applies comparisons per-element; simplifying here).
            nodes.first().cloned().cloned().unwrap_or(Value::Null)
        }
    }
}

/// Compare two JSON values using the provided comparison mode.
fn cmp_values<F>(a: &Value, b: &Value, opts: &JpOptions, pred_on_ord: F) -> bool
where F: Fn(i32) -> bool
{
    println!("cmp_values({:?}, {:?}, {:?})", a, b, opts);
    // Try type-aware compare; fall back to string compare with case mode
    match (a, b) {

        (Value::String(sa), Value::String(sb)) => {
            let (la, lb) = match opts.cmp {
                CmpMode::CaseSensitive => (sa.clone(), sb.clone()),
                CmpMode::CaseFoldLower => (sa.to_lowercase(), sb.to_lowercase()),
                CmpMode::CaseFoldUpper => (sa.to_uppercase(), sb.to_uppercase()),
            };
            pred_on_ord(la.cmp(&lb) as i32)
        }
        (Value::Number(na), Value::Number(nb)) => {
            if let (Some(da), Some(db)) = (na.as_f64(), nb.as_f64()) {
                let ord = if (da - db).abs() < f64::EPSILON { 0 } else if da < db { -1 } else { 1 };
                pred_on_ord(ord)
            } else { pred_on_ord(0) && na == nb }
        }
        (Value::Bool(ba), Value::Bool(bb)) => {
            let ord = (*ba as i32) - (*bb as i32);
            pred_on_ord(ord)
        }
        // Cross-type compares: number vs string (try parse), else fallback to string compare
        (Value::Number(na), Value::String(sb)) |
        (Value::String(sb), Value::Number(na)) => {
            if let (Some(da), Ok(db)) = (na.as_f64(), sb.parse::<f64>()) {
                let ord = if (da - db).abs() < f64::EPSILON { 0 } else if da < db { -1 } else { 1 };
                pred_on_ord(ord)
            } else {
                let sa = a.to_string();
                let sb = b.to_string();
                let (la, lb) = match opts.cmp {
                    CmpMode::CaseSensitive => (sa, sb),
                    CmpMode::CaseFoldLower => (sa.to_lowercase(), sb.to_lowercase()),
                    CmpMode::CaseFoldUpper => (sa.to_uppercase(), sb.to_uppercase()),
                };
                pred_on_ord(la.cmp(&lb) as i32)
            }
        }
        _ => {
            let sa = a.to_string();
            let sb = b.to_string();
            let (la, lb) = match opts.cmp {
                CmpMode::CaseSensitive => (sa, sb),
                CmpMode::CaseFoldLower => (sa.to_lowercase(), sb.to_lowercase()),
                CmpMode::CaseFoldUpper => (sa.to_uppercase(), sb.to_uppercase()),
            };
            pred_on_ord(la.cmp(&lb) as i32)
        }
    }
}

/// --------- Tests ---------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn fixture() -> Value {
        serde_json::from_str(r#"
        {
          "_schema": "otel",
          "otel": {
            "clid": "1131109258.1919201358.556614944.1080005413",
            "client_id": [1131109258, 1919201358, 556614944, 1080005413],
            "collector_tstamp": "2025-09-19 13:00:35.681",
            "customer_id": 1960180521,
            "dvce_created_tstamp": "2025-09-19 13:00:31.840",
            "dvce_sent_tstamp": "2025-09-19 13:00:35.681",
            "m3dv": {},
            "resourceSpans": [
              {
                "resource": {
                  "attributes": [
                    { "key": "service.name", "value": "nexa-agent-server" },
                    { "key": "service.version", "value": "0.0.3" },
                    { "key": "environment", "value": "production" },
                    { "key": "team", "value": "ai-platform" }
                  ],
                  "droppedAttributesCount": 0
                },
                "scopeSpans": [
                  {
                    "scope": { "droppedAttributesCount": 0, "name": "gcp.vertex.agent" },
                    "spans": [
                      {
                        "attributes": [
                          { "key": "c3.convID", "value": "nexa-app:k1tOlgG86ZGKTlXoYh78X:1810667" }
                        ],
                        "droppedAttributesCount": 0,
                        "droppedEventsCount": 0,
                        "droppedLinksCount": 0,
                        "endTimeUnixNano": "1758286831840012081",
                        "flags": 256,
                        "kind": 1,
                        "name": "invocation",
                        "spanId": "bf7d2ab5e7a5865f",
                        "startTimeUnixNano": "1758286813086110705",
                        "status": { "code": 0 },
                        "traceId": "89eb0472bde1e4ce2832bb5fbb9a44bd"
                      }
                    ]
                  }
                ]
              }
            ]
          }
        }
        "#).unwrap()
    }

    #[test]
    fn service_name_basic() {
        let j = fixture();
        let jp = JsonPath::new(&j);
        let path = r#"$.otel.resourceSpans[*].resource.attributes[?(@.key == "service.name")].value"#;
        let out = jp.query(path);
        assert_eq!(out, json!(["nexa-agent-server"]));
    }

    #[test]
    fn case_insensitive_with_lower_helper() {
        let j = fixture();
        let jp = JsonPath::new(&j);
        let path = r#"$.otel.resourceSpans[*].resource.attributes[? (lower(@.key) == "SERVICE.NAME") ].value"#;
        let out = jp.query(path);
        assert_eq!(out, json!(["nexa-agent-server"]));
    }

    #[test]
    fn wildcard_and_index_and_slice() {
        let j = fixture();
        let jp = JsonPath::new(&j);
        assert_eq!(jp.query("$.otel.client_id[0]"), json!([1131109258]));
        assert_eq!(jp.query("$.otel.client_id[*]"), json!([1131109258,1919201358,556614944,1080005413]));
        assert_eq!(jp.query("$.otel.client_id[1:3]"), json!([1919201358,556614944]));
        assert_eq!(jp.query("$.otel.client_id[::-2]").as_array().unwrap().len() > 0, true);
    }

    #[test]
    fn recursive_descent_names() {
        let j = fixture();
        let jp = JsonPath::new(&j);
        let out = jp.query("$..name");
        assert!(out.as_array().unwrap().iter().any(|v| v == "invocation"));
        assert!(out.as_array().unwrap().iter().any(|v| v == "gcp.vertex.agent"));
    }

    #[test]
    fn default_on_no_match_and_numeric_ops() {
        let j = fixture();
        let opts = JpOptions { default: Some(json!("unknown")), cmp: CmpMode::CaseSensitive };
        let jp = JsonPath::new(&j).with_options(opts.clone());
        let out = jp.query(r#"$.otel.resourceSpans[*].resource.attributes[?(@.key == "missing")].value"#);
        assert_eq!(out, json!("unknown"));

        // numeric compare + length()
        let jp2 = JsonPath::new(&j);
        let out2 = jp2.query(r#"$.otel.client_id[? (length(@) >= 0) ]"#);
        assert!(out2.is_array());
    }

    #[test]
    fn logical_ops_examples() {
        let j = fixture();
        let jp = JsonPath::new(&j);
        let path = r#"$.otel.resourceSpans[*].resource.attributes[?((@.key=="environment" && @.value=="production") || (@.key=="team" && @.value=="ai-platform"))].key"#;
        let out = jp.query(path);
        let keys: Vec<_> = out.as_array().unwrap().iter().cloned().collect();
        assert!(keys.contains(&json!("environment")) && keys.contains(&json!("team")));
    }
}

