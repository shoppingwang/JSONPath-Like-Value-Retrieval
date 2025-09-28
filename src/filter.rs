use serde_json::Value;
use crate::comparison::cmp_values;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    Eq(Operand, Operand),
    Ne(Operand, Operand),
    Lt(Operand, Operand),
    Lte(Operand, Operand),
    Gt(Operand, Operand),
    Gte(Operand, Operand),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Truthy(Operand),
}

#[derive(Debug, Clone)]
pub enum Operand {
    CurrentPath(Vec<PathToken>), // @.a['b'][0]
    Literal(Value),              // "abc", 123, true/false/null
    Lower(Box<Operand>),
    Upper(Box<Operand>),
    Length(Box<Operand>),
}

#[derive(Debug, Clone)]
pub enum PathToken {
    Key(String),
    Index(i64),
    Wildcard,
}

use crate::jsonpath::ParseErr;
use crate::parser::Parser;

pub fn parse_filter_or(parser: &mut Parser) -> Result<FilterExpr, ParseErr> {
    let mut left = parse_filter_and(parser)?;
    loop {
        parser.skip_ws();
        if parser.peek_str("||") {
            parser.consume_char('|');
            parser.consume_char('|');
            let right = parse_filter_and(parser)?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_filter_and(parser: &mut Parser) -> Result<FilterExpr, ParseErr> {
    let mut left = parse_filter_not(parser)?;
    loop {
        parser.skip_ws();
        if parser.peek_str("&&") {
            parser.consume_char('&');
            parser.consume_char('&');
            let right = parse_filter_not(parser)?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_filter_not(parser: &mut Parser) -> Result<FilterExpr, ParseErr> {
    parser.skip_ws();
    if parser.consume_char('!') {
        let inner = parse_filter_not(parser)?;
        Ok(FilterExpr::Not(Box::new(inner)))
    } else {
        parse_filter_compare(parser)
    }
}

fn parse_filter_compare(parser: &mut Parser) -> Result<FilterExpr, ParseErr> {
    parser.skip_ws();
    if parser.consume_char('(') {
        let inner = parse_filter_or(parser)?;
        parser.expect(')')?;
        return Ok(inner);
    }
    let left = parse_operand(parser)?;
    parser.skip_ws();
    let op = if parser.peek_str("==") {
        parser.consume_char('=');
        parser.consume_char('=');
        Some("==")
    } else if parser.peek_str("!=") {
        parser.consume_char('!');
        parser.consume_char('=');
        Some("!=")
    } else if parser.peek_str("<=") {
        parser.consume_char('<');
        parser.consume_char('=');
        Some("<=")
    } else if parser.peek_str(">=") {
        parser.consume_char('>');
        parser.consume_char('=');
        Some(">=")
    } else if parser.peek_char() == Some('<') {
        parser.consume_char('<');
        Some("<")
    } else if parser.peek_char() == Some('>') {
        parser.consume_char('>');
        Some(">")
    } else {
        None
    };
    if let Some(op) = op {
        parser.skip_ws();
        let right = parse_operand(parser)?;
        return Ok(match op {
            "==" => FilterExpr::Eq(left, right),
            "!=" => FilterExpr::Ne(left, right),
            "<" => FilterExpr::Lt(left, right),
            "<=" => FilterExpr::Lte(left, right),
            ">" => FilterExpr::Gt(left, right),
            ">=" => FilterExpr::Gte(left, right),
            _ => unreachable!(),
        });
    }
    Ok(FilterExpr::Truthy(left))
}

fn parse_operand(parser: &mut Parser) -> Result<Operand, ParseErr> {
    parser.skip_ws();
    if parser.peek_char() == Some('"') || parser.peek_char() == Some('\'') {
        return Ok(Operand::Literal(Value::String(parser.parse_quoted_string()?)));
    }
    if parser.peek_str("true") {
        for _ in 0..4 { parser.consume_char('t'); parser.consume_char('r'); parser.consume_char('u'); parser.consume_char('e'); }
        return Ok(Operand::Literal(Value::Bool(true)));
    }
    if parser.peek_str("false") {
        for _ in 0..5 { parser.consume_char('f'); parser.consume_char('a'); parser.consume_char('l'); parser.consume_char('s'); parser.consume_char('e'); }
        return Ok(Operand::Literal(Value::Bool(false)));
    }
    if parser.peek_str("null") {
        for _ in 0..4 { parser.consume_char('n'); parser.consume_char('u'); parser.consume_char('l'); parser.consume_char('l'); }
        return Ok(Operand::Literal(Value::Null));
    }

    if parser.peek_str("lower(") {
        for _ in 0..6 { parser.consume_char('l'); parser.consume_char('o'); parser.consume_char('w'); parser.consume_char('e'); parser.consume_char('r'); parser.consume_char('('); }
        let inner = parse_operand(parser)?;
        parser.expect(')')?;
        return Ok(Operand::Lower(Box::new(inner)));
    }
    if parser.peek_str("upper(") {
        for _ in 0..6 { parser.consume_char('u'); parser.consume_char('p'); parser.consume_char('p'); parser.consume_char('e'); parser.consume_char('r'); parser.consume_char('('); }
        let inner = parse_operand(parser)?;
        parser.expect(')')?;
        return Ok(Operand::Upper(Box::new(inner)));
    }
    if parser.peek_str("length(") {
        for _ in 0..7 { parser.consume_char('l'); parser.consume_char('e'); parser.consume_char('n'); parser.consume_char('g'); parser.consume_char('t'); parser.consume_char('h'); parser.consume_char('('); }
        let inner = parse_operand(parser)?;
        parser.expect(')')?;
        return Ok(Operand::Length(Box::new(inner)));
    }

    if parser.peek_char() == Some('@') {
        parser.consume_char('@');
        let mut tokens = Vec::new();
        loop {
            parser.skip_ws();
            if parser.consume_char('.') {
                if parser.consume_char('*') {
                    tokens.push(PathToken::Wildcard);
                    continue;
                }
                let k = parser.parse_identifier()?;
                tokens.push(PathToken::Key(k));
                continue;
            } else if parser.consume_char('[') {
                if parser.consume_char('*') {
                    parser.expect(']')?;
                    tokens.push(PathToken::Wildcard);
                    continue;
                }
                if parser.peek_char() == Some('"') || parser.peek_char() == Some('\'') {
                    let k = parser.parse_quoted_string()?;
                    parser.expect(']')?;
                    tokens.push(PathToken::Key(k));
                    continue;
                }
                let idx_content = parser.capture_until(']')?;
                parser.expect(']')?;
                let mut tmp = Parser::new(idx_content);
                let idx = tmp.parse_int()?;
                tokens.push(PathToken::Index(idx));
                continue;
            }
            break;
        }
        return Ok(Operand::CurrentPath(tokens));
    }

    if parser
        .peek_char()
        .map(|c| c == '-' || c.is_ascii_digit())
        .unwrap_or(false)
    {
        let n = parser.parse_number_literal()?;
        return Ok(Operand::Literal(n));
    }
    Err(ParseErr::InvalidSyntax("invalid operand".into()))
}

pub fn eval_filter(expr: &FilterExpr, current: &Value) -> bool {
    match expr {
        FilterExpr::Eq(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o == 0,
        ),
        FilterExpr::Ne(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o != 0,
        ),
        FilterExpr::Lt(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o < 0,
        ),
        FilterExpr::Lte(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o <= 0,
        ),
        FilterExpr::Gt(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o > 0,
        ),
        FilterExpr::Gte(a, b) => cmp_values(
            &eval_operand(a, current),
            &eval_operand(b, current),
            |o| o >= 0,
        ),
        FilterExpr::And(l, r) => eval_filter(l, current) && eval_filter(r, current),
        FilterExpr::Or(l, r) => eval_filter(l, current) || eval_filter(r, current),
        FilterExpr::Not(i) => !eval_filter(i, current),
        FilterExpr::Truthy(op) => truthy(&eval_operand(op, current)),
    }
}

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

fn eval_operand(op: &Operand, current: &Value) -> Value {
    match op {
        Operand::Literal(v) => v.clone(),
        Operand::Lower(inner) => {
            let v = eval_operand(inner, current);
            if let Some(s) = v.as_str() {
                Value::String(s.to_lowercase())
            } else {
                v
            }
        }
        Operand::Upper(inner) => {
            let v = eval_operand(inner, current);
            if let Some(s) = v.as_str() {
                Value::String(s.to_uppercase())
            } else {
                v
            }
        }
        Operand::Length(inner) => {
            let v = eval_operand(inner, current);
            let len = match v {
                Value::Array(a) => a.len() as i64,
                Value::Object(m) => m.len() as i64,
                Value::String(s) => s.chars().count() as i64,
                _ => 0,
            };
            Value::from(len)
        }
        Operand::CurrentPath(tokens) => {
            let mut nodes = vec![current];
            for t in tokens {
                nodes = match t {
                    PathToken::Key(k) => nodes
                        .into_iter()
                        .flat_map(|n| match n {
                            Value::Object(m) => m.get(k).into_iter().collect(),
                            _ => Vec::new(),
                        })
                        .collect(),
                    PathToken::Index(i) => {
                        if *i < 0 {
                            Vec::new()
                        } else {
                            let idx = *i as usize;
                            nodes
                                .into_iter()
                                .flat_map(|n| match n {
                                    Value::Array(a) => a.get(idx).into_iter().collect(),
                                    _ => Vec::new(),
                                })
                                .collect()
                        }
                    }
                    PathToken::Wildcard => nodes
                        .into_iter()
                        .flat_map(|n| match n {
                            Value::Array(a) => a.iter().collect(),
                            Value::Object(m) => m.values().collect(),
                            _ => Vec::new(),
                        })
                        .collect(),
                }
            }
            nodes.first().cloned().cloned().unwrap_or(Value::Null)
        }
    }
}
