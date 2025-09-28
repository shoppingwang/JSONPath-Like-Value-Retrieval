# JSONPath-like extraction with tiny expression language (Rust)

`json-path-like-extraction` is a small Rust library and CLI that combines:

1. A pragmatic **JSONPath-like engine** (dot/bracket, wildcards, recursive descent, filters, `length()`, slicing)
2. A tiny **expression evaluator** so you can pass a **single string** like:

```text
first(from_json("{\"otel\": {\"resourceSpans\": [{\"resource\": {\"attributes\": [{\"key\": \"service.name\", \"value\": \"nexa\"}]}}]}}", "$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value"))
```

…and get back the computed JSON value.

No extra arguments. Everything (JSON + path + helper functions) lives in that one expression.

---

## Features

* **Single-input expression**: `eval_expr("first(from_json(\"<JSON>\", \"$.path\"))")`
* **JSONPath-like queries**:

    * Root `$`
    * Dot & bracket notation: `$.a.b`, `$['a']['b']`
    * Wildcards: `*` for arrays/objects
    * Recursive descent: `..`
    * Filters: `[?(expr)]` with:

        * comparisons: `==`, `!=`, `<`, `<=`, `>`, `>=`
        * logical ops: `&&`, `||`, `!`
        * helpers: `lower()`, `upper()`, `length()`
        * `@` for current node paths like `@.key`, `@['key']`, `@[0]`
    * Array slicing: `[start:end:step]` (negatives allowed, omissions allowed)
* **Composable helpers** in expressions:

    * `from_json("<JSON>", "$.path")` → returns **array of matches** or `null`
    * `first(expr)` → first element or `null`
    * `unique(expr)` → JSON-deep dedup of arrays
    * `or_default(expr, "<JSON default>")` → fallback if `expr` is `null` or `[]`
* **Return types**: raw JSON values (no coercion)
* **No panics**: invalid JSON, invalid path, or bad expressions resolve to `null`
* **Small, dependency-light**: serde/serde\_json, itertools, clap (for CLI)

---

## Install

Add to your Rust project:

---

## Quick Start

### CLI

Run the expression directly:

```bash
cargo run -- "first(from_json(\"{\\\"otel\\\":{\\\"resourceSpans\\\":[{\\\"resource\\\":{\\\"attributes\\\":[{\\\"key\\\":\\\"service.name\\\",\\\"value\\\":\\\"nexa-agent-server\\\"}]}}]}}\",\"$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value\"))"
```

Output:

```json
"nexa-agent-server"
```

> **Tip:** When calling from shells, you’ll often need to escape quotes inside the JSON string. For complex payloads, consider reading JSON from a file and interpolating it into the expression or using single quotes outside and escaping inner single quotes accordingly.

---

## Expression Language

The library exposes a tiny expression language that supports **nested function calls** and **string literals**:

```
Expr := Call | "string" | 'string'
Call := Ident "(" ArgList? ")"
ArgList := Expr ("," Expr)*
Ident := [A-Za-z_][A-Za-z0-9_]*
```

### Built-in functions

| Function     | Signature                            | Description                                                                                                  | Returns                          |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------ | -------------------------------- |
| `from_json`  | `from_json("<JSON>", "$.path")`      | Parse JSON string and evaluate the JSONPath-like query.                                                      | **Array** of matches or **null** |
| `first`      | `first(expr)`                        | Get the first element of an **array**.                                                                       | First value or **null**          |
| `unique`     | `unique(expr)`                       | Deduplicate elements in an **array** by deep JSON equality.                                                  | Deduped array                    |
| `or_default` | `or_default(expr, "<JSON default>")` | If `expr` is **null** or **\[]**, return parsed default JSON (or treat string as plain if JSON parse fails). | `expr` or default value          |

**Examples:**

```text
first(from_json("{\"a\":[1,2,3]}", "$.a[*]"))                            -> 1
unique(from_json("{\"a\":[1,1,2,2]}", "$.a[*]"))                         -> [1,2]
or_default(from_json("{\"a\":1}", "$.missing"), "{\"fallback\":true}")    -> {"fallback": true}
```

---

## JSONPath-like Syntax

* **Root**: `$`
* **Keys**:

    * Dot: `$.otel.resourceSpans`
    * Bracket: `$['otel']['resourceSpans']`
* **Wildcards**:

    * Arrays: `$.a[*]`
    * Objects: `$.obj.*`
* **Recursive descent**:

    * `$..name` (collects **all** `name` fields at any depth)
* **Array index**:

    * `$.a[0]`
* **Array slice** `[start:end:step]`:

    * `$.a[1:3]` → indices 1,2
    * `$.a[::2]` → every 2nd
    * `$.a[::-1]` → reverse (via negative step)
    * `$.a[-3:]` → last 3 items
* **Filters** `[?(expr)]`:

    * Comparisons: `==`, `!=`, `<`, `<=`, `>`, `>=`
    * Logical ops: `&&`, `||`, `!`
    * Grouping: parentheses
    * Helpers in filters:

        * `lower(x)`, `upper(x)` (for strings)
        * `length(x)` → arrays/objects/strings length (else 0)
    * `@` points to the **current element** during filtering:

        * `@.key`, `@['key']`, `@[0]`, `@.*`

**OTEL example** (primary use case):

```text
$.otel.resourceSpans[*].resource.attributes[?(@.key == "service.name")].value
```

Case-insensitive compare (via helper):

```text
$.otel.resourceSpans[*].resource.attributes[? (lower(@.key) == "service.name") ].value
```

---

## Behavior & Return Shapes

* `from_json()` returns:

    * **Array** of all match values (raw types preserved)
    * **null** when:

        * Path has no matches, or
        * Input JSON is invalid, or
        * The path/expression is invalid
* `first()` returns:

    * Scalar JSON value or **null**
* `unique()` returns:

    * Deduped array (if input is array), else returns value unchanged
* `or_default(expr, "<JSON>")`:

    * If `expr` is **null** or **\[]**, parse `"<JSON>"` (or keep as string if parsing fails) and return it

**Duplicates:** kept by default; use `unique()` for uniques.
**Type coercion:** none — values are returned as-is.

---

## Error Handling

* No panics. Invalid JSON/JSONPath/expression → **null**.
* Filters comparing different types:

    * Number ↔ string: attempts numeric parse; otherwise falls back to string comparison.

---

## Performance Notes

* Operates in-memory on `serde_json::Value`.
* `..` (recursive descent) walks the entire subtree and may collect many nodes.

---

## CLI Examples

**1) Basic `from_json`**

```bash
cargo run -- "from_json(\"{\\\"a\\\":[0,1,2,3,4]}\", \"$.a[1:4:2]\")"
# => [1,3]
```

**2) OTEL service name, first match only**

```bash
cargo run -- "first(from_json(\"{\\\"otel\\\":{\\\"resourceSpans\\\":[{\\\"resource\\\":{\\\"attributes\\\":[{\\\"key\\\":\\\"service.name\\\",\\\"value\\\":\\\"nexa-agent-server\\\"}]}}]}}\", \"$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value\"))"
# => "nexa-agent-server"
```

**3) Fallback default**

```bash
cargo run -- "or_default(from_json(\"{\\\"a\\\":1}\", \"$.missing\"), \"{\\\"fallback\\\":true}\")"
# => {"fallback": true}
```

---

## Library API Surface

Although you only need `eval_expr()` for the single-string interface, these are available internally:

```rust
pub fn eval_expr(expr: &str) -> serde_json::Value;

pub fn from_json(json_str: &str, path: &str) -> serde_json::Value;
pub fn first(vals: &serde_json::Value) -> serde_json::Value;
pub fn unique(vals: &serde_json::Value) -> serde_json::Value;
pub fn or_default(vals: &serde_json::Value, default_json: &str) -> serde_json::Value;
```

---

## Tests

```bash
cargo test
```

Covers:

* End-to-end expression (`first(from_json(...))`)
* Wildcards, indexes, slices
* Recursive descent
* Filters (comparisons & logical ops)
* `length()`, `lower()`, `upper()`
* `unique()`, `or_default()`

---

## Limitations & Roadmap

* Expression string parser supports a **minimal escape set**: `\"`, `\'`, `\\`, `\n`, `\t`, `\r`.
  (Your JSON payload string is parsed by `serde_json`, so it can use full JSON escapes.)
* In filter `@` paths, only simple indices are supported for `@[...]` (no slices yet).
* Recursive descent may be expensive on large documents.

**Potential extensions:**

* `contains(haystack, needle)` and regex `match()`
* Richer escaping and Unicode in expression strings
* Case-folding options inside expressions
* `map`, `pluck`, `flatten`, and other transformations
