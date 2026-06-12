# json-schema-sort

Deterministic, schema-aware sorting for JSON Schema files.

This is not a validator, a schema generator, or canonical JSON. It sorts JSON Schema documents into a stable, human-readable order so diffs stop being noisy.

## What it sorts

- JSON object keys by JSON Schema keyword priority.
- Unknown object keys alphabetically after known schema keywords.
- `required`, `type`, and similar schema-safe string arrays.

Known keywords are grouped by their JSON Schema 2020-12 vocabulary (core,
meta-data, validation, format, content, applicator, unevaluated), and within
each group they follow the order used in that vocabulary's meta-schema. The
groups themselves are arranged for readability, so identity and documentation
keywords stay near the top.

It deliberately does not sort arrays where order may be meaningful, such as `enum`, `examples`, `prefixItems`, `allOf`, `anyOf`, or `oneOf`.

The literal value payloads of `const`, `enum`, `default`, and `examples` are treated as opaque: the sorter does not recurse into them, so the structure and key order of any objects inside them are preserved. (The string APIs still round-trip through `serde_json`, so insignificant whitespace and some scalar lexemes may be normalized.)

## Installation

```sh
cargo install --git https://github.com/kjanat/json-schema-sort-rs \
  json-schema-sort
```

<!--<details><summary>powershell</summary>-->

```pwsh
# powershell
cargo install --git 'https://github.com/kjanat/json-schema-sort-rs' `
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

`SortOptions` is `#[non_exhaustive]`. Construct it from `Default` and adjust
the fields you care about:

```rust
use json_schema_sort::{ArraySorting, SortOptions};

let mut options = SortOptions::default();
options.arrays = ArraySorting::None;
let sorted = json_schema_sort::to_string_sorted_with_options(schema_text, options)?;
```
