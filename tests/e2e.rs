use json_path_like_extraction as jple;
use serde_json::json;

// Examples split so each assertion stands alone
#[test]
fn test_example_service_name() {
    let expr = r#"first(from_json("{\"otel\":{\"resourceSpans\":[{\"resource\":{\"attributes\":[{\"key\":\"service.name\",\"value\":\"nexa-agent-server\"}]}}]}}","$.otel.resourceSpans[*].resource.attributes[?(@.key==\"service.name\")].value"))"#;
    let out = jple::eval(expr).unwrap();
    assert_eq!(out, json!("nexa-agent-server"));
}

#[test]
fn test_example_first_array() {
    let expr = r#"first(from_json("{\"a\":[1,2,3]}", "$.a[*]"))"#;
    let out = jple::eval(expr).unwrap();
    assert_eq!(out, json!(1));
}

#[test]
fn test_example_unique() {
    let expr = r#"unique(from_json("{\"a\":[1,1,2,2]}", "$.a[*]"))"#;
    let out = jple::eval(expr).unwrap();
    assert_eq!(out, json!([1, 2]));
}

#[test]
fn test_example_or_default() {
    let expr = r#"or_default(from_json("{\"a\":1}", "$.missing"), "{\"fallback\":true}")"#;
    let out = jple::eval(expr).unwrap();
    assert_eq!(out, json!({"fallback": true}));
}

#[test]
fn test_example_slice() {
    let expr = r#"from_json("{\"a\":[0,1,2,3,4]}", "$.a[1:4:2]")"#;
    let out = jple::eval(expr).unwrap();
    assert_eq!(out, json!([1, 3]));
}

// Unit style built-ins split
#[test]
fn test_builtin_first() {
    assert_eq!(jple::first(&json!([10, 20])), json!(10));
}

#[test]
fn test_builtin_unique() {
    assert_eq!(jple::unique(&json!([1, 1, 2, 2, 3])), json!([1, 2, 3]));
}

#[test]
fn test_builtin_or_default() {
    assert_eq!(jple::or_default(&json!(null), "{\"x\":1}"), json!({"x":1}));
}

// Property: unique is idempotent (already single assertion)
#[test]
fn unique_idempotent_smoke() {
    let once = jple::unique(&json!([1, 1, 2, 3, 3]));
    let twice = jple::unique(&once);
    assert_eq!(once, twice);
}

// Recursive descent tests split
fn recursive_test_json() -> &'static str {
    r#"{
        "departments": [
            {
                "team": [
                    {
                        "name": "Alice Johnson",
                        "info": {
                            "position": "Software Engineer",
                            "age": 29,
                            "email": "alice.johnson@example.com"
                        }
                    },
                    {
                        "name": "Bob Smith",
                        "info": {
                            "position": "UI/UX Designer",
                            "age": 34,
                            "email": "bob.smith@example.com"
                        }
                    }
                ]
            },
            {
                "team": [
                    {
                        "name": "Carol Lee",
                        "info": {
                            "position": "Project Manager",
                            "age": 41,
                            "email": "carol.lee@example.com"
                        }
                    },
                    {
                        "name": "David Kim",
                        "info": {
                            "position": "QA Engineer",
                            "age": 27,
                            "email": "david.kim@example.com"
                        }
                    }
                ]
            }
        ]
    }"#
}

#[test]
fn test_recursive_descent_all_names() {
    let result = jple::engine::from_json(recursive_test_json(), "$..name");
    assert_eq!(
        result,
        json!(["Alice Johnson", "Bob Smith", "Carol Lee", "David Kim"])
    );
}

#[test]
fn test_recursive_descent_departments() {
    let result = jple::engine::from_json(recursive_test_json(), "$.departments");
    assert_eq!(
        result,
        json!([{ "team":[{"name":"Alice Johnson","info":{"position":"Software Engineer","age":29,"email":"alice.johnson@example.com"}},{"name":"Bob Smith","info":{"position":"UI/UX Designer","age":34,"email":"bob.smith@example.com"}}]},{"team":[{"name":"Carol Lee","info":{"position":"Project Manager","age":41,"email":"carol.lee@example.com"}},{"name":"David Kim","info":{"position":"QA Engineer","age":27,"email":"david.kim@example.com"}}]}])
    );
}

#[test]
fn test_recursive_descent_specific_name() {
    let result = jple::engine::from_json(recursive_test_json(), "$.departments[0].team[0].name");
    assert_eq!(result, json!(["Alice Johnson"]));
}
