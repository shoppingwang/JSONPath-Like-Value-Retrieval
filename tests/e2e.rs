use json_path_like_extraction as jple;
use serde_json::json;

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
