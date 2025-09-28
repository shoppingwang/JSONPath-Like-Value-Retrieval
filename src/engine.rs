use crate::{ expression, jsonpath};
use itertools::Itertools;
use serde_json::{json, Value};

/// =========================
/// Public API (Expression)
/// =========================

/// Evaluate a single expression string, e.g.
///   first(from_json("<JSON>", "$.path"))
/// Returns a serde_json::Value result.
pub fn eval_expr(expr: &str) -> Value {
    match expression::parse_expr(expr) {
        Ok(ast) => expression::eval_ast(&ast),
        Err(_) => Value::Null,
    }
}

/// =========================
/// Public API (Library funcs)
/// =========================

#[derive(Default, Clone)]
pub struct JpOptions {
    pub default: Option<Value>,
}

/// Convenience: parse JSON string and evaluate `path` with default options.
/// Returns Array of matches, or null if invalid JSON or no match.
pub fn from_json(json_str: &str, path: &str) -> Value {
    let data: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Value::Null,
    };
    jsonpath::from_value_with_opts(&data, path, &JpOptions::default())
}

/// Return the first element from a result Array; else null.
pub fn first(vals: &Value) -> Value {
    match vals {
        Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// Deduplicate an array Value; identity for non-array.
pub fn unique(vals: &Value) -> Value {
    match vals {
        Value::Array(a) => {
            let dedup = a
                .iter()
                .cloned()
                .unique_by(|x| serde_json::to_string(x).unwrap_or_default())
                .collect::<Vec<_>>();
            Value::Array(dedup)
        }
        _ => vals.clone(),
    }
}

/// If `vals` is null or empty array, return parsed default JSON string; else `vals`.
pub fn or_default(vals: &Value, default_json: &str) -> Value {
    let default_val = serde_json::from_str::<Value>(default_json)
        .unwrap_or_else(|_| Value::String(default_json.to_string()));
    match vals {
        Value::Null => default_val,
        Value::Array(a) if a.is_empty() => default_val,
        _ => vals.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_json() -> &'static str {
        r#"
        {
          "otel": {
            "client_id": [1131109258, 1919201358, 556614944, 1080005413],
            "resourceSpans": [{
              "resource": {
                "attributes": [
                  { "key": "service.name", "value": "nexa-agent-server" },
                  { "key": "service.version", "value": "0.0.3" },
                  { "key": "environment", "value": "production" }
                ]
              }
            }]
          }
        }
        "#
    }

    #[test]
    fn eval_entire_expression() {
        let expr = format!(
            "first(from_json('{}', \"$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value\"))",
            sample_json().replace('"', "\\\"")
        );
        let out = eval_expr(&expr);
        assert_eq!(out, json!("nexa-agent-server"));
    }

    #[test]
    fn unique_and_default() {
        let json = r#"{"a":[1,1,2,2,3]}"#;
        let expr = format!(
            "unique(from_json('{}', '$.a[*]'))",
            json.replace('"', "\\\"")
        );
        let out = eval_expr(&expr);
        assert_eq!(out, json!([1, 2, 3]));

        let expr2 = "or_default(from_json('{\"a\":1}', '$.missing'), '{\"fallback\":true}')";
        let out2 = eval_expr(expr2);
        assert_eq!(out2, json!({"fallback": true}));
    }
}
