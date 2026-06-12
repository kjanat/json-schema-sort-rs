//! Deterministic, schema-aware sorting for JSON Schema documents.
//!
//! This crate reorders the object keys of a JSON Schema into a stable,
//! human-readable order so that diffs stay quiet. It is not a validator, a
//! schema generator, or a canonical JSON serializer.
//!
//! Object keys are ordered by JSON Schema keyword priority, with unknown keys
//! falling back to alphabetical order after the known keywords. Identifier keys
//! — the names under `properties`, `$defs`, and similar keyword maps — are
//! sorted alphabetically by default, or kept in document order via
//! [`PropertyOrdering::Preserve`]. Arrays whose order may be meaningful — such
//! as `enum`, `examples`, `prefixItems`, `allOf`, or `oneOf` — are left
//! untouched, and the literal payloads of value-bearing keywords (`const`,
//! `enum`, `default`, `examples`) are treated as opaque and never recursed
//! into.
//!
//! # Examples
//!
//! ```
//! let sorted = json_schema_sort::to_string_sorted(r#"{"type":"object","$id":"x"}"#)?;
//! assert!(sorted.starts_with("{\n  \"$id\""));
//! # Ok::<(), serde_json::Error>(())
//! ```

use std::cmp::Ordering;

use serde_json::{Map, Value};

/// Strategy for sorting JSON arrays encountered while sorting a schema.
///
/// This enum is `#[non_exhaustive]`: new strategies may be added in future
/// releases, so matches must include a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArraySorting {
    /// Leave every array in its original order.
    None,
    /// Sort only arrays whose order carries no meaning, such as the string
    /// arrays under `required`, `type`, and `dependentRequired`. Arrays where
    /// order may matter (for example `enum` or `prefixItems`) are left alone.
    SchemaSafe,
}

/// Strategy for ordering identifier keys — the user-defined names under
/// `properties`, `patternProperties`, `$defs`, and similar keyword maps.
///
/// This enum is `#[non_exhaustive]`: new strategies may be added in future
/// releases, so matches must include a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyOrdering {
    /// Sort identifier keys alphabetically. This keeps diffs stable regardless
    /// of the order properties were authored in.
    Sorted,
    /// Keep identifier keys in their original document order. Property order is
    /// often intentional (it mirrors struct fields, drives form/UI generation,
    /// or simply reads better), so this leaves it untouched while still sorting
    /// the keywords inside each subschema.
    Preserve,
}

/// Options controlling how a schema is sorted.
///
/// This struct is `#[non_exhaustive]`: new fields may be added in future
/// releases. Construct it from [`SortOptions::default`] and adjust the public
/// fields rather than using a struct literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SortOptions {
    /// How to sort arrays. Defaults to [`ArraySorting::SchemaSafe`].
    pub arrays: ArraySorting,
    /// How to order identifier keys such as property and `$defs` names.
    /// Defaults to [`PropertyOrdering::Sorted`].
    pub properties: PropertyOrdering,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            arrays: ArraySorting::SchemaSafe,
            properties: PropertyOrdering::Sorted,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Context {
    /// The keyword whose value this node is, when the parent was a schema
    /// object. `None` at the root, inside arrays, or when the parent was a
    /// keyword map (where the parent key is a user identifier, not a keyword).
    parent_key: Option<&'static str>,
    /// Whether this object's *own* keys are user-defined identifiers (property
    /// names, `$defs` names, ...) rather than schema keywords. Such keys must
    /// not be interpreted as keywords and may be order-preserved.
    keys_are_names: bool,
    in_dependent_required: bool,
}

impl Context {
    const ROOT: Self = Self {
        parent_key: None,
        keys_are_names: false,
        in_dependent_required: false,
    };
}

/// Sort a schema in place using the default [`SortOptions`].
pub fn sort_schema(value: &mut Value) {
    sort_schema_with_options(value, SortOptions::default());
}

/// Sort a schema in place using the supplied [`SortOptions`].
pub fn sort_schema_with_options(value: &mut Value, options: SortOptions) {
    sort_value(value, Context::ROOT, options);
}

/// Sort a schema and return it, using the default [`SortOptions`].
///
/// This is a convenience wrapper around [`sort_schema`] for callers that own
/// the value and prefer a by-value style.
pub fn sorted_schema(mut value: Value) -> Value {
    sort_schema(&mut value);
    value
}

/// Parse a JSON string, sort it with the default [`SortOptions`], and
/// re-serialize it as pretty-printed JSON with a trailing newline.
///
/// # Errors
///
/// Returns the underlying [`serde_json::Error`] if `input` is not valid JSON.
pub fn to_string_sorted(input: &str) -> Result<String, serde_json::Error> {
    to_string_sorted_with_options(input, SortOptions::default())
}

/// Parse a JSON string, sort it with the supplied [`SortOptions`], and
/// re-serialize it as pretty-printed JSON with a trailing newline.
///
/// # Errors
///
/// Returns the underlying [`serde_json::Error`] if `input` is not valid JSON.
pub fn to_string_sorted_with_options(
    input: &str,
    options: SortOptions,
) -> Result<String, serde_json::Error> {
    let mut value: Value = serde_json::from_str(input)?;
    sort_schema_with_options(&mut value, options);

    serde_json::to_string_pretty(&value).map(|mut output| {
        output.push('\n');
        output
    })
}

fn sort_value(value: &mut Value, context: Context, options: SortOptions) {
    match value {
        Value::Object(map) => sort_object(map, context, options),
        Value::Array(values) => sort_array(values, context, options),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn sort_object(map: &mut Map<String, Value>, context: Context, options: SortOptions) {
    if context.keys_are_names {
        // This object is a keyword map (e.g. the value of `properties`): its
        // keys are user identifiers, and every value is a fresh schema object.
        // The key names are never keywords, so opaque/keyword handling does not
        // apply to them.
        for value in map.values_mut() {
            sort_value(value, Context::ROOT, options);
        }

        if options.properties == PropertyOrdering::Preserve {
            return;
        }

        let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        map.extend(entries);
        return;
    }

    let child_in_dependent_required = context.parent_key == Some("dependentRequired");

    for (key, value) in map.iter_mut() {
        // Keywords like `const`, `enum`, `default`, and `examples` carry
        // literal value payloads, not subschemas. Their contents are opaque:
        // reordering object keys inside them would silently rewrite the user's
        // data, so we leave the whole payload untouched.
        if is_opaque_value_key(key) {
            continue;
        }

        let child_context = Context {
            parent_key: intern_known_key(key),
            keys_are_names: is_name_map_key(key),
            in_dependent_required: child_in_dependent_required,
        };

        sort_value(value, child_context, options);
    }

    let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
    entries.sort_by(|(left, _), (right, _)| compare_schema_keys(left, right));
    map.extend(entries);
}

fn sort_array(values: &mut [Value], context: Context, options: SortOptions) {
    for value in values.iter_mut() {
        sort_value(value, Context::ROOT, options);
    }

    if options.arrays != ArraySorting::SchemaSafe {
        return;
    }

    let sortable_string_array =
        matches!(context.parent_key, Some("required" | "type")) || context.in_dependent_required;

    if sortable_string_array && values.iter().all(Value::is_string) {
        values.sort_by(|left, right| left.as_str().unwrap().cmp(right.as_str().unwrap()));
    }
}

fn compare_schema_keys(left: &str, right: &str) -> Ordering {
    match (schema_key_rank(left), schema_key_rank(right)) {
        (Some(left_rank), Some(right_rank)) => left_rank.cmp(&right_rank),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.cmp(right),
    }
}

// Keywords are bucketed by their JSON Schema 2020-12 vocabulary (function),
// the same grouping the spec authors use in their meta-schemas
// (https://json-schema.org/draft/2020-12/meta/*). The buckets are ordered for
// human readability (identity -> documentation -> constraints -> annotations
// -> applicators) so that metadata stays near the top, and within a bucket the
// order is the intuitive reading order: min before max, properties before
// additionalProperties. Legacy keywords from older drafts are slotted next to
// their modern siblings. A handful of non-standard tooling extensions are
// recognized too: the description/enum "shadows" sit beside the standard
// keyword they annotate, and the rest fall into a trailing extension bucket.
fn schema_key_rank(key: &str) -> Option<u8> {
    Some(match key {
        // Core vocabulary (.../vocab/core)
        "$id" => 0,
        "id" => 1, // draft-04 legacy identifier
        "$schema" => 2,
        "$ref" => 3,
        "$anchor" => 4,
        "$dynamicRef" => 5,
        "$recursiveRef" => 6, // draft 2019-09 legacy
        "$dynamicAnchor" => 7,
        "$recursiveAnchor" => 8, // draft 2019-09 legacy
        "$vocabulary" => 9,
        "$comment" => 10,
        "$defs" => 11,
        "definitions" => 12, // draft-07 legacy

        // Meta-data vocabulary (.../vocab/meta-data)
        "title" => 20,
        "description" => 21,
        "markdownDescription" => 22, // vscode-json-languageservice, shadows description
        "default" => 23,
        "deprecated" => 24,
        "readOnly" => 25,
        "writeOnly" => 26,
        "examples" => 27,

        // Validation vocabulary (.../vocab/validation)
        "type" => 30,
        "const" => 31,
        "enum" => 32,
        "enumDescriptions" => 33, // vscode-json-languageservice, shadows enum
        "markdownEnumDescriptions" => 34, // vscode-json-languageservice, shadows enum
        "multipleOf" => 35,
        "minimum" => 36,
        "exclusiveMinimum" => 37,
        "maximum" => 38,
        "exclusiveMaximum" => 39,
        "minLength" => 40,
        "maxLength" => 41,
        "pattern" => 42,
        "minItems" => 43,
        "maxItems" => 44,
        "uniqueItems" => 45,
        "minContains" => 46,
        "maxContains" => 47,
        "minProperties" => 48,
        "maxProperties" => 49,
        "required" => 50,
        "dependentRequired" => 51,
        "dependencies" => 52, // draft-07 legacy

        // Format vocabulary (.../vocab/format-annotation)
        "format" => 60,

        // Content vocabulary (.../vocab/content)
        "contentEncoding" => 61,
        "contentMediaType" => 62,
        "contentSchema" => 63,

        // Applicator vocabulary (.../vocab/applicator)
        "prefixItems" => 70,
        "items" => 71,
        "additionalItems" => 72, // draft-07 legacy
        "contains" => 73,
        "properties" => 74,
        "patternProperties" => 75,
        "additionalProperties" => 76,
        "propertyNames" => 77,
        "dependentSchemas" => 78,
        "if" => 79,
        "then" => 80,
        "else" => 81,
        "allOf" => 82,
        "anyOf" => 83,
        "oneOf" => 84,
        "not" => 85,

        // Unevaluated vocabulary (.../vocab/unevaluated)
        "unevaluatedItems" => 90,
        "unevaluatedProperties" => 91,

        // Non-standard tooling extensions that have no obvious standard sibling.
        // Recognized by the language servers behind SchemaStore. Ranked after
        // every standard keyword but ahead of unknown keys, grouped by the tool
        // that consumes them.
        //
        // vscode-json-languageservice
        "defaultSnippets" => 100,
        "allowTrailingCommas" => 101,
        // tamasfe/taplo
        "x-taplo" => 102,
        "x-taplo-info" => 103,
        // tombi-toml/tombi
        "x-tombi-toml-version" => 104,
        "x-tombi-array-values-order" => 105,
        "x-tombi-array-values-order-by" => 106,
        "x-tombi-table-keys-order" => 107,
        "x-tombi-string-formats" => 108,
        "x-tombi-additional-key-label" => 109,
        // IntelliJ
        "x-intellij-language-injection" => 110,
        "x-intellij-html-description" => 111,
        "x-intellij-enum-metadata" => 112,

        _ => return None,
    })
}

/// Keywords whose values are literal data payloads rather than subschemas.
/// The sorter must not recurse into these, so the structure and object-key
/// order of user-supplied values are preserved (re-serialization through
/// `serde_json` may still normalize whitespace and some scalar lexemes).
fn is_opaque_value_key(key: &str) -> bool {
    matches!(key, "const" | "enum" | "default" | "examples")
}

/// Keywords whose values are maps from user-defined identifiers to subschemas.
/// The map's own keys are names (property names, definition names, regex
/// patterns), not schema keywords, so they are ordered by [`PropertyOrdering`]
/// rather than by keyword rank, and they are never treated as keywords.
fn is_name_map_key(key: &str) -> bool {
    matches!(
        key,
        "properties"
            | "patternProperties"
            | "$defs"
            | "definitions"
            | "dependentSchemas"
            | "dependencies"
    )
}

fn intern_known_key(key: &str) -> Option<&'static str> {
    match key {
        "required" => Some("required"),
        "type" => Some("type"),
        "dependentRequired" => Some("dependentRequired"),
        _ => None,
    }
}
