# json-schema-sort

Deterministic, schema-aware sorting for JSON Schema files.

This is not a validator, a schema generator, or canonical JSON. It sorts JSON Schema documents into a stable, human-readable order so diffs stop being noisy.

## What it sorts

- JSON object keys by JSON Schema keyword priority.
- Unknown object keys alphabetically after known schema keywords.
- Property and `$defs` names alphabetically (use `--preserve-property-order` to
  keep their authored order — it is often intentional).
- `required`, `type`, and similar schema-safe string arrays.

A key is only treated as a JSON Schema keyword when it sits in keyword position.
A property literally named `type`, `enum`, or `required` is an identifier, so it
sorts alphabetically among its siblings and its value is never reordered.

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

Keep property names in their authored order:

```sh
json-schema-sort --preserve-property-order schema.json
```

### Multiple inputs

Inputs can be files, directories, or glob patterns. Directories are searched
recursively for `*.json`. Sorting more than one file requires `--write` or
`--check` (printing several files to stdout is refused):

```sh
json-schema-sort --write schemas/                 # whole tree
json-schema-sort --check 'schemas/**/*.schema.json'
```

Files discovered by expanding a directory or glob that git ignores are skipped
(an explicitly named file is always processed). Pass `--no-ignore` to include
git-ignored files too.

## pre-commit

This repo ships [pre-commit](https://pre-commit.com) hooks. Add to your
`.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/kjanat/json-schema-sort-rs
  rev: v0.1.0
  hooks:
    - id: json-schema-sort # rewrites in place; use json-schema-sort-check to only verify
```

Both hooks default to any `schema.json` or `*.schema.json` file anywhere in the
tree; override `files:` to change the set.

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
use json_schema_sort::{ArraySorting, PropertyOrdering, SortOptions};

let mut options = SortOptions::default();
options.arrays = ArraySorting::None;
options.properties = PropertyOrdering::Preserve;
let sorted = json_schema_sort::to_string_sorted_with_options(schema_text, options)?;
```
