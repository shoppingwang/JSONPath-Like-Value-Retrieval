# JSONPath-Like Value Retrieval (Rust)

A small Rust library and CLI for extracting values from JSON using a pragmatic JSONPath-like engine and a tiny expression language.

## Overview

This project provides:
- **JSONPath-like engine**: dot/bracket notation, wildcards, recursive descent, filters, `length()`, slicing
- **Expression evaluator**: Compose JSON queries and helpers in a single string
- **CLI and library**: Use from the command line or as a Rust crate

## Features

- **Single-input expression**: `eval_expr("first(from_json(\"<JSON>\", \"$.path\"))")`
- **JSONPath-like queries**:
    - Root `$`
    - Dot & bracket notation: `$.a.b`, `$['a']['b']`
    - Wildcards: `*` for arrays/objects
    - Recursive descent: `..`
    - Filters: `[?(expr)]` with comparisons, logical ops, helpers, and `@` for current node
    - Array slicing: `[start:end:step]` (negatives/omissions allowed)
- **Composable helpers**:
    - `from_json(<JSON>, <path>)` → array of matches or `null`
    - `first(expr)` → first element or `null`
    - `unique(expr)` → dedup array
    - `or_default(expr, <JSON default>)` → fallback if `expr` is `null` or `[]`
- **Return types**: raw JSON values
- **No panics**: invalid input resolves to `null`
- **Minimal dependencies**: serde/serde_json, itertools, clap (CLI)

## Quick Start

### CLI Usage

Run an expression directly:

```bash
cargo run -- --expr "first(from_json(\"{\\\"otel\\\":{\\\"resourceSpans\\\":[{\\\"resource\\\":{\\\"attributes\\\":[{\\\"key\\\":\\\"service.name\\\",\\\"value\\\":\\\"nexa-agent-server\\\"}]}}]}}\",\"$.otel.resourceSpans[*].resource.attributes[?(@.key==\\\"service.name\\\")].value\"))"
```
Output:
```json
"nexa-agent-server"
```

### Library Usage

```rust
use json_path_like_extraction as jple;
use serde_json::json;

let expr = r#"first(from_json("{\"otel\":{\"resourceSpans\":[{\"resource\":{\"attributes\":[{\"key\":\"service.name\",\"value\":\"nexa-agent-server\"}]}}]}}","$.otel.resourceSpans[*].resource.attributes[?(@.key==\"service.name\")].value"))"#;
let out = jple::eval(expr).unwrap();
assert_eq!(out, json!("nexa-agent-server"));
```

## Expression Language

Supports nested function calls and string literals:

```
Expr := Call | "string" | 'string'
Call := Ident ( ArgList? )
ArgList := Expr (, Expr)*
Ident := [A-Za-z_][A-Za-z0-9_]*
```

### Built-in Functions

| Function     | Signature                            | Description                                                                                                  |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `from_json`  | `from_json("<JSON>", "$.path")`      | Parse JSON and evaluate the query. Returns array of matches or `null`.                                       |
| `first`      | `first(expr)`                        | Get the first element of an array. Returns value or `null`.                                                  |
| `unique`     | `unique(expr)`                       | Deduplicate array elements by deep equality. Returns deduped array.                                          |
| `or_default` | `or_default(expr, "<JSON default>")` | If `expr` is `null` or `[]`, return parsed default JSON (or plain string if parse fails).                    |

#### Examples

```text
first(from_json("{\"a\":[1,2,3]}", "$.a[*]"))                            -> 1
unique(from_json("{\"a\":[1,1,2,2]}", "$.a[*]"))                         -> [1,2]
or_default(from_json("{\"a\":1}", "$.missing"), "{\"fallback\":true}")    -> {"fallback": true}
```

## JSONPath Syntax

- **Root**: `$`
- **Keys**: Dot (`$.otel.resourceSpans`), Bracket (`$['otel']['resourceSpans']`)
- **Wildcards**: Arrays (`$.a[*]`), Objects (`$.obj.*`)
- **Recursive descent**: `$..name` (all `name` fields at any depth)
- **Array index**: `$.a[0]`
- **Array slice**: `[start:end:step]` (e.g. `$.a[1:3]`, `$.a[::2]`, `$.a[::-1]`, `$.a[-3:]`)
- **Filters**: `[?(expr)]` with comparisons, logical ops, grouping, helpers (`lower()`, `upper()`, `length()`), and `@` for current element

## Behavior & Return Shapes

- `from_json()` returns array of matches or `null`
- `first()` returns scalar value or `null`
- `unique()` returns deduped array or value unchanged
- `or_default()` returns fallback if input is `null` or `[]`
- No type coercion; values returned as-is

## Error Handling

- No panics; invalid input resolves to `null`
- Filters comparing different types: number ↔ string attempts numeric parse, else string compare

## Performance Notes

- Operates in-memory on `serde_json::Value`
- Recursive descent (`..`) walks entire subtree

## CLI Examples

**Basic `from_json`**
```bash
cargo run -- --expr "from_json(\"{\\\"a\\\":[0,1,2,3,4]}\", \"$.a[1:4:2]\")"
# => [1,3]
```

**OTEL service name, first match only**
```bash
cargo run -- --expr "first(from_json(\"{\\\"otel\\\":{\\\"resourceSpans\\\":[{\\\"resource\\\":{\\\"attributes\\\":[{\\\"key\\\":\\\"service.name\\\",\\\"value\\\":\\\"nexa-agent-server\\\"}]}}]}}\", \"$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value\"))"
# => "nexa-agent-server"
```

**Fallback default**
```bash
cargo run -- --expr "or_default(from_json(\"{\\\"a\\\":1}\", \"$.missing\"), \"{\\\"fallback\\\":true}\")"
# => {"fallback": true}
```

## Testing

Run all tests:
```bash
cargo test
```

Covers:
- End-to-end expressions
- Wildcards, indexes, slices
- Recursive descent
- Filters (comparisons, logical ops)
- Helpers: `length()`, `lower()`, `upper()`, `unique()`, `or_default()`

## Limitations & Roadmap

- Minimal escape set in expression parser: `\"`, `\'`, `\\`, `\n`, `\t`, `\r`
- In filter `@` paths, only simple indices for `@[...]` (no slices yet)
- Recursive descent may be expensive on large documents

**Potential extensions:**
- `contains(haystack, needle)`, regex `match()`
- Richer escaping and Unicode
- Case-folding options
- `map`, `pluck`, `flatten`, and other transformations
