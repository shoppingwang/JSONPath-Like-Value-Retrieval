# JSONPath-Like-Value-Retrieval

Support the value retrieve from JSON like the expression first(from_json("This is JSON", "$.otel.resourceSpans[*]
.resource.attributes[?(@.key=='service.name')].value"))

## Quick try

```shell
cargo run -- "$(cat <<'JSON'
{
  "_schema": "otel",
  "otel": {
    "resourceSpans": [{
      "resource": {
        "attributes": [
          { "key": "service.name", "value": "nexa-agent-server" },
          { "key": "environment", "value": "production" }
        ]
      }
    }],
    "client_id": [1,2,3,4]
  }
}
JSON
)" '$.otel.resourceSpans[*].resource.attributes[?(@.key=="service.name")].value'
```

Outputs:

```json
[
  "nexa-agent-server"
]
```

```shell
cargo run -- '{"a":[0,1,2,3,4,5]}' '$.a[1:5:2]'
```

Outputs:

```json
[
  1,
  3
]
```

```shell
cargo run -- '{"a":[1,2,3]}' '$.a[?(length(@) >= 0)]'
```

Outputs:

```json
[
  1,
  2,
  3
]
```