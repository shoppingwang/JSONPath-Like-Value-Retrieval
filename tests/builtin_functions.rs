use json_path_like_extraction as jple;
use serde_json::json;
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
