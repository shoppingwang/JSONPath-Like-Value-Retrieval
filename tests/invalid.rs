use json_path_like_value_retrieval as jpl;

// Invalid JSONPath slice number: uses a non-numeric token in slice component to trigger
// the slice parser's `bad slice number` error path. JSONPath parsing errors inside
// from_json are coerced to Value::Null (they do not bubble as EvalError), so we assert Null.
#[test]
fn test_invalid_jsonpath_slice_bad_number() {
    // $.a[1:x] -> 'x' cannot be parsed as i64, triggering ParseErr::InvalidSyntax("bad slice number") internally.
    let expr = r#"from_json("{\"a\":[0,1,2,3]}", "$.a[1:x]")"#;
    let out = jpl::eval(expr).unwrap();
    assert!(
        out.is_null(),
        "Expected Null result for invalid slice number, got: {out}"
    );
}
