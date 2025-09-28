use crate::{expression, jsonpath};
use itertools::Itertools;
use serde_json::Value;

/// =========================
/// Public API (Expression)
/// =========================

/// Evaluates a single expression string.
/// Example: first(from_json("<JSON>", "$.path"))
/// Returns the result as a serde_json::Value.
/// If parsing fails, returns Value::Null.
pub fn eval_expr(expr: &str) -> Value {
    match expression::parse_expr(expr) {
        Ok(ast) => expression::eval_ast(&ast), // Evaluate parsed AST
        Err(_) => Value::Null, // Return Null on parse error
    }
}

/// =========================
/// Public API (Library funcs)
/// =========================

/// Parses a JSON string and evaluates a JSONPath expression.
/// Returns an array of matches, or Null if JSON is invalid or no match found.
pub fn from_json(json_str: &str, path: &str) -> Value {
    let data: Value = match serde_json::from_str(json_str) {
        Ok(v) => v, // Successfully parsed JSON
        Err(_) => return Value::Null, // Return Null on parse error
    };
    jsonpath::from_value(&data, path) // Apply JSONPath to parsed data
}

/// Returns the first element from a result array.
/// If input is not an array or is empty, returns Null.
pub fn first(vals: &Value) -> Value {
    match vals {
        Value::Array(a) => a.first().cloned().unwrap_or(Value::Null), // First element or Null
        _ => Value::Null, // Not an array
    }
}

/// Deduplicates an array Value using string serialization for comparison.
/// Returns the deduplicated array, or the original value if not an array.
pub fn unique(vals: &Value) -> Value {
    match vals {
        Value::Array(a) => {
            let dedup = a
                .iter()
                .cloned()
                // Use string representation for uniqueness
                .unique_by(|x| serde_json::to_string(x).unwrap_or_default())
                .collect::<Vec<_>>();
            Value::Array(dedup)
        }
        _ => vals.clone(), // Non-array values are returned unchanged
    }
}

/// Returns a default value if input is Null or an empty array.
/// The default is parsed from a JSON string, or used as a string if parsing fails.
/// Otherwise, returns the original value.
pub fn or_default(vals: &Value, default_json: &str) -> Value {
    let default_val = serde_json::from_str::<Value>(default_json)
        .unwrap_or_else(|_| Value::String(default_json.to_string())); // Fallback to string if not valid JSON
    match vals {
        Value::Null => default_val, // Use default if Null
        Value::Array(a) if a.is_empty() => default_val, // Use default if empty array
        _ => vals.clone(), // Otherwise, return original value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Returns a sample JSON string for testing.
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

    /// Tests evaluating a full expression string.
    #[test]
    fn eval_entire_expression() {
        let expr = format!(
            "first(from_json('{}', \"$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value\"))",
            sample_json().replace('"', "\\\"")
        );
        let out = eval_expr(&expr);
        assert_eq!(out, json!("nexa-agent-server"));
    }

    /// Tests unique and or_default functions.
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
