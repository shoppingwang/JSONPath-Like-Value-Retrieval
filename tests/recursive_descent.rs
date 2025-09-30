use json_path_like_value_retrieval as jpl;
use serde_json::json;
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
    let result = jpl::engine::from_json(recursive_test_json(), "$..name");
    assert_eq!(
        result,
        json!(["Alice Johnson", "Bob Smith", "Carol Lee", "David Kim"])
    );
}

#[test]
fn test_recursive_descent_departments() {
    let result = jpl::engine::from_json(recursive_test_json(), "$.departments");
    assert_eq!(
        result,
        json!([{ "team":[{"name":"Alice Johnson","info":{"position":"Software Engineer","age":29,"email":"alice.johnson@example.com"}},{"name":"Bob Smith","info":{"position":"UI/UX Designer","age":34,"email":"bob.smith@example.com"}}]},{"team":[{"name":"Carol Lee","info":{"position":"Project Manager","age":41,"email":"carol.lee@example.com"}},{"name":"David Kim","info":{"position":"QA Engineer","age":27,"email":"david.kim@example.com"}}]}])
    );
}

#[test]
fn test_recursive_descent_specific_name() {
    let result = jpl::engine::from_json(recursive_test_json(), "$.departments[0].team[0].name");
    assert_eq!(result, json!(["Alice Johnson"]));
}
