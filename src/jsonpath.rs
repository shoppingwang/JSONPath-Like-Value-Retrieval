use crate::filter::FilterExpr;
use crate::parser::{ParseError, Parser};
use serde_json::Value;
use tracing::error;

/// Represents a parsed JSONPath, consisting of a sequence of segments.
#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Segment>,
}

/// Enum for each possible segment in a JSONPath expression.
#[derive(Debug, Clone)]
pub enum Segment {
    Root,        // `$` - root of the JSON document
    Key(String), // `.foo` or `['foo']` - object key access
    Wildcard,    // `.*` or `[*]` - matches all child elements
    Index(i64),  // `[0]` - array index access
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    }, // `[start:end:step]` - array slicing
    Recursive,   // `..` - recursive descent
    Filter(Box<FilterExpr>), // `[?(expr)]` - filter expression
}

pub type ParseErr = ParseError;

/// Entry point: evaluates a JSONPath string against a JSON value.
/// Returns the matched values as a JSON array, or Null if no match.
pub fn from_value(data: &Value, path: &str) -> Value {
    match parse_path(path) {
        Ok(ast) => {
            let refs = eval_path(data, &ast);
            if refs.is_empty() {
                Value::Null
            } else {
                // If exactly one match and that match itself is an array, unwrap it so we don't
                // introduce an extra level of nesting (e.g. $.departments should yield the
                // departments array, not [ departments_array ]). This matches the expectations
                // in tests where selecting an array container returns the array directly, while
                // selecting multiple elements (e.g. wildcard / recursive descent) still returns
                // a flat array of matches.
                if refs.len() == 1 {
                    if let Value::Array(_) = refs[0] {
                        return refs[0].clone();
                    }
                }
                Value::Array(refs.into_iter().cloned().collect())
            }
        }
        Err(e) => {
            let bt = std::backtrace::Backtrace::capture();
            error!(target: "jsonpath", error = ?e, backtrace = ?bt, "JSONPath parse error");
            Value::Null
        }
    }
}

/// Parses a JSONPath string into a Path AST.
fn parse_path(input: &str) -> Result<Path, ParseErr> {
    let mut p = PathParser::new(input);
    p.parse()
}

/// Parser for JSONPath strings.
pub struct PathParser<'a> {
    parser: Parser<'a>,
}

impl<'a> PathParser<'a> {
    /// Creates a new PathParser from a string slice.
    pub fn new(s: &'a str) -> Self {
        Self {
            parser: Parser::new(s),
        }
    }

    /// Parses the full path, returning a Path AST.
    fn parse(&mut self) -> Result<Path, ParseErr> {
        let mut segments = Vec::new();
        self.parser.skip_ws();
        // Path must start with `$`
        if !self.parser.consume_char('$') {
            return Err(ParseErr::InvalidSyntax("path must start with `$`".into()));
        }
        segments.push(Segment::Root);

        // Parse each segment until end of input
        while !self.parser.eof() {
            self.parser.skip_ws();

            if let Some(segment) = self.parse_next_segment(segments.last())? {
                segments.push(segment);
            } else {
                break;
            }
        }
        Ok(Path { segments })
    }

    /// Parses the next segment in the path.
    /// `prev` provides the previously parsed segment (if any) allowing context-sensitive parsing
    /// for cases like recursive descent where a bare identifier or wildcard may follow (`$..name`).
    fn parse_next_segment(&mut self, prev: Option<&Segment>) -> Result<Option<Segment>, ParseErr> {
        // Recursive descent: `..`
        if self.parser.peek_str("..") {
            self.parser.consume_char('.');
            self.parser.consume_char('.');
            return Ok(Some(Segment::Recursive));
        }

        // Dot notation: `.key` or `.*`
        if self.parser.consume_char('.') {
            return self.parse_dot_segment();
        }

        // Bracket notation: `[key]`, `[index]`, `[slice]`, `[*]`, `[?(filter)]`
        if self.parser.consume_char('[') {
            return self.parse_bracket_segment();
        }

        // Support for bare identifier or wildcard immediately following a Recursive segment
        // allowing `$..name` / `$..*` per JSONPath semantics.
        if matches!(prev, Some(Segment::Recursive)) {
            if self.parser.peek_char() == Some('*') {
                self.parser.consume_char('*');
                return Ok(Some(Segment::Wildcard));
            }
            if let Some(c) = self.parser.peek_char() {
                if c == '_' || c.is_ascii_alphanumeric() {
                    // start of identifier
                    let key = self.parser.parse_identifier()?;
                    return Ok(Some(Segment::Key(key)));
                }
            }
        }

        Ok(None)
    }

    /// Parses a dot segment: either a wildcard or a key.
    fn parse_dot_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        if self.parser.consume_char('*') {
            Ok(Some(Segment::Wildcard))
        } else {
            let key = self.parser.parse_identifier()?;
            Ok(Some(Segment::Key(key)))
        }
    }

    /// Parses a bracket segment: wildcard, filter, key, index, or slice.
    fn parse_bracket_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        self.parser.skip_ws();

        // Wildcard: `[*]`
        if self.parser.consume_char('*') {
            self.parser.expect(']')?;
            return Ok(Some(Segment::Wildcard));
        }

        // Filter: `[?(expr)]`
        if self.parser.peek_char() == Some('?') {
            return self.parse_filter_segment();
        }

        // Quoted key: `['key']` or `["key"]`
        if matches!(self.parser.peek_char(), Some('\'') | Some('"')) {
            let key = self.parser.parse_quoted_string()?;
            self.parser.expect(']')?;
            return Ok(Some(Segment::Key(key)));
        }

        // Index or slice: `[0]`, `[1:3]`, `[1:3:2]`
        self.parse_index_or_slice_segment()
    }

    /// Parses a filter segment: `[?(expr)]`
    fn parse_filter_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        self.parser.consume_char('?');
        self.parser.expect('(')?;
        let expr = crate::filter::parse_filter_or(&mut self.parser)?;
        self.parser.expect(')')?;
        self.parser.expect(']')?;
        Ok(Some(Segment::Filter(Box::new(expr))))
    }

    /// Parses an index or slice segment.
    fn parse_index_or_slice_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        let slice_content = self.parser.capture_until(']')?;
        self.parser.expect(']')?;

        // Slice: contains `:`
        if slice_content.contains(':') {
            self.parse_slice(&slice_content)
        } else {
            // Index: single integer
            let mut tmp = Parser::new(&slice_content);
            let idx = tmp.parse_int()?;
            Ok(Some(Segment::Index(idx)))
        }
    }

    /// Parses a slice segment: `[start:end:step]`
    fn parse_slice(&self, content: &str) -> Result<Option<Segment>, ParseErr> {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() > 3 {
            return Err(ParseErr::InvalidSyntax("slice too many components".into()));
        }

        // Helper to parse optional i64 values
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

        Ok(Some(Segment::Slice { start, end, step }))
    }
}

/// Evaluates a parsed Path AST against a JSON value.
/// Returns a vector of references to matched values.
fn eval_path<'a>(root: &'a Value, path: &Path) -> Vec<&'a Value> {
    let mut current: Vec<&Value> = vec![root];
    for seg in &path.segments {
        current = eval_segment(&current, seg, root);
    }
    current
}

/// Evaluates a single segment against the current set of values.
fn eval_segment<'a>(current: &[&'a Value], segment: &Segment, root: &'a Value) -> Vec<&'a Value> {
    match segment {
        Segment::Root => vec![root],
        Segment::Key(k) => eval_key_segment(current, k),
        Segment::Index(i) => eval_index_segment(current, *i),
        Segment::Slice { start, end, step } => eval_slice_segment(current, *start, *end, *step),
        Segment::Wildcard => eval_wildcard_segment(current),
        Segment::Recursive => eval_recursive_segment(current),
        Segment::Filter(expr) => eval_filter_segment(current, expr),
    }
}

/// Evaluates a key segment: gets the value for the given key from each object.
fn eval_key_segment<'a>(current: &[&'a Value], key: &str) -> Vec<&'a Value> {
    current
        .iter()
        .filter_map(|v| match v {
            Value::Object(map) => map.get(key),
            _ => None,
        })
        .collect()
}

/// Evaluates an index segment: gets the value at the given index from each array.
fn eval_index_segment<'a>(current: &[&'a Value], index: i64) -> Vec<&'a Value> {
    if index < 0 {
        return Vec::new();
    }

    let idx = index as usize;
    current
        .iter()
        .filter_map(|v| match v {
            Value::Array(arr) => arr.get(idx),
            _ => None,
        })
        .collect()
}

/// Evaluates a slice segment: gets a slice of values from each array.
fn eval_slice_segment<'a>(
    current: &[&'a Value],
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
) -> Vec<&'a Value> {
    current
        .iter()
        .flat_map(|v| match v {
            Value::Array(arr) => slice_array(arr, start, end, step),
            _ => Vec::new(),
        })
        .collect()
}

/// Evaluates a wildcard segment: gets all child values from each object or array.
fn eval_wildcard_segment<'a>(current: &[&'a Value]) -> Vec<&'a Value> {
    current.iter().flat_map(|v| get_child_values(v)).collect()
}

/// Evaluates a recursive segment: collects all descendant nodes that can be searched.
/// This implements proper JSONPath recursive descent semantics.
fn eval_recursive_segment<'a>(current: &[&'a Value]) -> Vec<&'a Value> {
    let mut result = Vec::new();

    for &value in current {
        // Only collect nodes that can meaningfully have keys applied to them
        collect_searchable_nodes(value, &mut result);
    }

    result
}

/// Collects all descendant nodes that could be targets for subsequent path segments.
/// This includes the current node and all nested objects and arrays, but excludes
/// primitive values that cannot have keys applied to them.
fn collect_searchable_nodes<'a>(value: &'a Value, result: &mut Vec<&'a Value>) {
    match value {
        Value::Object(_) => {
            // Objects can have keys applied to them
            result.push(value);
            // Recurse into object values
            if let Value::Object(obj) = value {
                for child_value in obj.values() {
                    collect_searchable_nodes(child_value, result);
                }
            }
        }
        Value::Array(arr) => {
            // Arrays can have indices applied, but we also need to search their contents
            result.push(value);
            // Recurse into array elements
            for child_value in arr.iter() {
                collect_searchable_nodes(child_value, result);
            }
        }
        _ => {
            // Primitive values (strings, numbers, booleans, null) cannot have
            // keys applied to them, so we don't include them in recursive descent
        }
    }
}

/// Evaluates a filter segment: filters values using the filter expression.
fn eval_filter_segment<'a>(current: &[&'a Value], expr: &FilterExpr) -> Vec<&'a Value> {
    current
        .iter()
        .flat_map(|v| get_filterable_values(v))
        .filter(|v| crate::filter::eval_filter(expr, v))
        .collect()
}

/// Gets all child values of an object or array.
fn get_child_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(arr) => arr.iter().collect(),
        Value::Object(map) => map.values().collect(),
        _ => Vec::new(),
    }
}

/// Gets values that can be filtered: array elements or the value itself.
fn get_filterable_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(arr) => arr.iter().collect(),
        _ => vec![value],
    }
}

/// Slices an array according to start, end, and step parameters.
fn slice_array(
    arr: &Vec<Value>,
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
) -> Vec<&Value> {
    let n = arr.len() as i64;
    let step = step.unwrap_or(1);
    if step == 0 {
        return Vec::new();
    }

    // Normalizes an index, handling negative values.
    let normalize_index = |i: i64| -> i64 {
        if i < 0 {
            (n + i).clamp(0, n)
        } else {
            i.clamp(0, n)
        }
    };

    let (lo, hi) = (
        normalize_index(start.unwrap_or(0)),
        normalize_index(end.unwrap_or(n)),
    );

    if step > 0 {
        slice_forward(arr, lo, hi, step)
    } else {
        slice_backward(arr, lo, hi, step, n)
    }
}

/// Slices an array forward (step > 0).
fn slice_forward(arr: &Vec<Value>, lo: i64, hi: i64, step: i64) -> Vec<&Value> {
    let mut out = Vec::new();
    let mut i = lo;
    while i < hi {
        if let Some(v) = arr.get(i as usize) {
            out.push(v);
        }
        i += step;
    }
    out
}

/// Slices an array backward (step < 0).
fn slice_backward(arr: &Vec<Value>, lo: i64, hi: i64, step: i64, n: i64) -> Vec<&Value> {
    let mut out = Vec::new();
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
    out
}
