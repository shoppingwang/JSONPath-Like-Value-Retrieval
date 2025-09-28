use crate::engine::JpOptions;
use crate::filter::FilterExpr;
use crate::parser::{ParseError, Parser};
use serde_json::Value;

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

pub type ParseErr = ParseError;

/// Internal engine
pub fn from_value_with_opts(data: &Value, path: &str, opts: &JpOptions) -> Value {
    match parse_path(path) {
        Ok(ast) => {
            let refs = eval_path(data, &ast);
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
    let mut p = PathParser::new(input);
    p.parse()
}

pub struct PathParser<'a> {
    parser: Parser<'a>,
}

impl<'a> PathParser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            parser: Parser::new(s),
        }
    }

    fn parse(&mut self) -> Result<Path, ParseErr> {
        let mut segments = Vec::new();
        self.parser.skip_ws();
        if !self.parser.consume_char('$') {
            return Err(ParseErr::InvalidSyntax("path must start with `$`".into()));
        }
        segments.push(Segment::Root);

        while !self.parser.eof() {
            self.parser.skip_ws();

            if let Some(segment) = self.parse_next_segment()? {
                segments.push(segment);
            } else {
                break;
            }
        }
        Ok(Path { segments })
    }

    fn parse_next_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        if self.parser.peek_str("..") {
            self.parser.consume_char('.');
            self.parser.consume_char('.');
            return Ok(Some(Segment::Recursive));
        }

        if self.parser.consume_char('.') {
            return self.parse_dot_segment();
        }

        if self.parser.consume_char('[') {
            return self.parse_bracket_segment();
        }

        Ok(None)
    }

    fn parse_dot_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        if self.parser.consume_char('*') {
            Ok(Some(Segment::Wildcard))
        } else {
            let key = self.parser.parse_identifier()?;
            Ok(Some(Segment::Key(key)))
        }
    }

    fn parse_bracket_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        self.parser.skip_ws();

        if self.parser.consume_char('*') {
            self.parser.expect(']')?;
            return Ok(Some(Segment::Wildcard));
        }

        if self.parser.peek_char() == Some('?') {
            return self.parse_filter_segment();
        }

        if matches!(self.parser.peek_char(), Some('\'') | Some('"')) {
            let key = self.parser.parse_quoted_string()?;
            self.parser.expect(']')?;
            return Ok(Some(Segment::Key(key)));
        }

        self.parse_index_or_slice_segment()
    }

    fn parse_filter_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        self.parser.consume_char('?');
        self.parser.expect('(')?;
        let expr = crate::filter::parse_filter_or(&mut self.parser)?;
        self.parser.expect(')')?;
        self.parser.expect(']')?;
        Ok(Some(Segment::Filter(Box::new(expr))))
    }

    fn parse_index_or_slice_segment(&mut self) -> Result<Option<Segment>, ParseErr> {
        let slice_content = self.parser.capture_until(']')?;
        self.parser.expect(']')?;

        if slice_content.contains(':') {
            self.parse_slice(&slice_content)
        } else {
            let mut tmp = Parser::new(&slice_content);
            let idx = tmp.parse_int()?;
            Ok(Some(Segment::Index(idx)))
        }
    }

    fn parse_slice(&self, content: &str) -> Result<Option<Segment>, ParseErr> {
        let parts: Vec<&str> = content.split(':').collect();
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

        Ok(Some(Segment::Slice { start, end, step }))
    }
}

fn eval_path<'a>(root: &'a Value, path: &Path) -> Vec<&'a Value> {
    let mut current: Vec<&Value> = vec![root];
    for seg in &path.segments {
        current = eval_segment(&current, seg, root);
    }
    current
}

fn eval_segment<'a>(
    current: &[&'a Value],
    segment: &Segment,
    root: &'a Value,
) -> Vec<&'a Value> {
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

fn eval_key_segment<'a>(current: &[&'a Value], key: &str) -> Vec<&'a Value> {
    current
        .iter()
        .filter_map(|v| match v {
            Value::Object(map) => map.get(key),
            _ => None,
        })
        .collect()
}

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

fn eval_wildcard_segment<'a>(current: &[&'a Value]) -> Vec<&'a Value> {
    current.iter().flat_map(|v| get_child_values(v)).collect()
}

fn eval_recursive_segment<'a>(current: &[&'a Value]) -> Vec<&'a Value> {
    current
        .iter()
        .flat_map(|v| {
            let mut out = Vec::new();
            recurse_collect(v, &mut out);
            out
        })
        .collect()
}

fn eval_filter_segment<'a>(
    current: &[&'a Value],
    expr: &FilterExpr,
) -> Vec<&'a Value> {
    current
        .iter()
        .flat_map(|v| get_filterable_values(v))
        .filter(|v| crate::filter::eval_filter(expr, v))
        .collect()
}

fn get_child_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(arr) => arr.iter().collect(),
        Value::Object(map) => map.values().collect(),
        _ => Vec::new(),
    }
}

fn get_filterable_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(arr) => arr.iter().collect(),
        _ => vec![value],
    }
}

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
