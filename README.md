# json-schema-sort

Deterministic, schema-aware sorting for JSON Schema files.

This is not a validator, a schema generator, or canonical JSON. It sorts JSON Schema documents into a stable, human-readable order so diffs stop being noisy.

## What it sorts

- JSON object keys by JSON Schema keyword priority.
- Unknown object keys alphabetically after known schema keywords.
- `required`, `type`, and similar schema-safe string arrays.

It deliberately does not sort arrays where order may be meaningful, such as `enum`, `examples`, `prefixItems`, `allOf`, `anyOf`, or `oneOf`.

## Installation

```sh
cargo install --git https://github.com/kjanat/json-schema-sort-rs \
  json-schema-sort
```

## CLI

Sort a schema and print it:

```sh
json-schema-sort schema.json
```

Rewrite a schema in place:

```sh
json-schema-sort --write schema.json
```

Check whether a schema is already sorted:

```sh
json-schema-sort --check schema.json
```

Disable array sorting:

```sh
json-schema-sort --no-sort-arrays schema.json
```

## Library

```rust
let sorted = json_schema_sort::to_string_sorted(schema_text)?;
```

Or sort an existing `serde_json::Value`:

```rust
let mut value: serde_json::Value = serde_json::from_str(schema_text)?;
json_schema_sort::sort_schema(&mut value);
```
