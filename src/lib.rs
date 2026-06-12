use std::cmp::Ordering;

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArraySorting {
    None,
    SchemaSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOptions {
    pub arrays: ArraySorting,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            arrays: ArraySorting::SchemaSafe,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Context {
    parent_key: Option<&'static str>,
    in_dependent_required: bool,
}

impl Context {
    const ROOT: Self = Self {
        parent_key: None,
        in_dependent_required: false,
    };
}

pub fn sort_schema(value: &mut Value) {
    sort_schema_with_options(value, SortOptions::default());
}

pub fn sort_schema_with_options(value: &mut Value, options: SortOptions) {
    sort_value(value, Context::ROOT, options);
}

pub fn sorted_schema(mut value: Value) -> Value {
    sort_schema(&mut value);
    value
}

pub fn to_string_sorted(input: &str) -> Result<String, serde_json::Error> {
    to_string_sorted_with_options(input, SortOptions::default())
}

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
    let child_in_dependent_required = context.parent_key == Some("dependentRequired");

    for (key, value) in map.iter_mut() {
        let child_context = Context {
            parent_key: intern_known_key(key),
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
        sort_value(
            value,
            Context {
                parent_key: None,
                in_dependent_required: false,
            },
            options,
        );
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

fn schema_key_rank(key: &str) -> Option<u8> {
    Some(match key {
        "$schema" => 0,
        "$id" => 1,
        "$anchor" => 2,
        "$dynamicAnchor" => 3,
        "$vocabulary" => 4,
        "$comment" => 5,
        "title" => 10,
        "description" => 11,
        "default" => 12,
        "examples" => 13,
        "deprecated" => 14,
        "readOnly" => 15,
        "writeOnly" => 16,
        "type" => 20,
        "const" => 21,
        "enum" => 22,
        "format" => 30,
        "contentEncoding" => 31,
        "contentMediaType" => 32,
        "contentSchema" => 33,
        "multipleOf" => 40,
        "minimum" => 41,
        "exclusiveMinimum" => 42,
        "maximum" => 43,
        "exclusiveMaximum" => 44,
        "minLength" => 50,
        "maxLength" => 51,
        "pattern" => 52,
        "prefixItems" => 60,
        "items" => 61,
        "contains" => 62,
        "minItems" => 63,
        "maxItems" => 64,
        "uniqueItems" => 65,
        "minContains" => 66,
        "maxContains" => 67,
        "properties" => 70,
        "patternProperties" => 71,
        "additionalProperties" => 72,
        "unevaluatedProperties" => 73,
        "propertyNames" => 74,
        "required" => 75,
        "dependentRequired" => 76,
        "dependentSchemas" => 77,
        "minProperties" => 78,
        "maxProperties" => 79,
        "$defs" => 90,
        "definitions" => 91,
        "allOf" => 100,
        "anyOf" => 101,
        "oneOf" => 102,
        "not" => 103,
        "if" => 104,
        "then" => 105,
        "else" => 106,
        "$ref" => 110,
        "$dynamicRef" => 111,
        _ => return None,
    })
}

fn intern_known_key(key: &str) -> Option<&'static str> {
    match key {
        "required" => Some("required"),
        "type" => Some("type"),
        "dependentRequired" => Some("dependentRequired"),
        _ => None,
    }
}
