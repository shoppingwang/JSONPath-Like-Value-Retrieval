use json_path_like_extraction as jple;
use serde_json::json;

#[test]
fn e2e_must_cover_examples() {
    // 1
    let expr1 = r#"first(from_json("{\"otel\":{\"resourceSpans\":[{\"resource\":{\"attributes\":[{\"key\":\"service.name\",\"value\":\"nexa-agent-server\"}]}}]}}","$.otel.resourceSpans[*].resource.attributes[?(@.key==\"service.name\")].value"))"#;
    let out1 = jple::eval(expr1).unwrap();
    assert_eq!(out1, json!("nexa-agent-server"));

    // 2
    let expr2 = r#"first(from_json("{\"a\":[1,2,3]}", "$.a[*]"))"#;
    let out2 = jple::eval(expr2).unwrap();
    assert_eq!(out2, json!(1));

    // 3
    let expr3 = r#"unique(from_json("{\"a\":[1,1,2,2]}", "$.a[*]"))"#;
    let out3 = jple::eval(expr3).unwrap();
    assert_eq!(out3, json!([1,2]));

    // 4
    let expr4 = r#"or_default(from_json("{\"a\":1}", "$.missing"), "{\"fallback\":true}")"#;
    let out4 = jple::eval(expr4).unwrap();
    assert_eq!(out4, json!({"fallback": true}));

    // 5
    let expr5 = r#"from_json("{\"a\":[0,1,2,3,4]}", "$.a[1:4:2]")"#;
    let out5 = jple::eval(expr5).unwrap();
    assert_eq!(out5, json!([1,3]));
}

// A couple of unit style checks on built-ins wired through re-exports
#[test]
fn unit_builtins() {
    use serde_json::json;
    assert_eq!(jple::first(&json!([10,20])), json!(10));
    assert_eq!(jple::unique(&json!([1,1,2,2,3])), json!([1,2,3]));
    assert_eq!(jple::or_default(&json!(null), "{\"x\":1}"), json!({"x":1}));
}

// Property: unique is idempotent (basic smoke since we don't run proptest here)
#[test]
fn unique_idempotent_smoke() {
    use serde_json::json;
    let once = jple::unique(&json!([1,1,2,3,3]));
    let twice = jple::unique(&once);
    assert_eq!(once, twice);
}