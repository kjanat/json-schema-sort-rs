use json_schema_sort::{PropertyOrdering, SortOptions};
use serde_json::json;

fn preserve_options() -> SortOptions {
    let mut options = SortOptions::default();
    options.properties = PropertyOrdering::Preserve;
    options
}

#[test]
fn sorts_schema_keywords_before_unknown_keys() {
    let mut value = json!({
        "properties": {},
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "description": "User object",
        "title": "User",
        "type": "object"
    });

    json_schema_sort::sort_schema(&mut value);

    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        ["$schema", "title", "description", "type", "properties"]
    );
}

#[test]
fn sorts_property_keys_alphabetically() {
    let mut value = json!({
        "type": "object",
        "properties": {
            "z": { "type": "string" },
            "a": { "type": "number" }
        }
    });

    json_schema_sort::sort_schema(&mut value);

    let properties = value.get("properties").unwrap().as_object().unwrap();
    let keys: Vec<_> = properties.keys().cloned().collect();
    assert_eq!(keys, ["a", "z"]);
}

#[test]
fn preserves_property_order_when_requested() {
    let mut value = json!({
        "type": "object",
        "properties": {
            "z": { "type": "string" },
            "a": { "type": "number" }
        }
    });

    json_schema_sort::sort_schema_with_options(&mut value, preserve_options());

    let properties = value.get("properties").unwrap().as_object().unwrap();
    let keys: Vec<_> = properties.keys().cloned().collect();
    assert_eq!(
        keys,
        ["z", "a"],
        "property names should keep document order"
    );
}

#[test]
fn preserve_still_sorts_keywords_inside_each_property() {
    let mut value = json!({
        "properties": {
            "z": { "type": "string", "$id": "x", "title": "Z" }
        }
    });

    json_schema_sort::sort_schema_with_options(&mut value, preserve_options());

    let inner = value["properties"]["z"].as_object().unwrap();
    let keys: Vec<_> = inner.keys().cloned().collect();
    assert_eq!(keys, ["$id", "title", "type"]);
}

#[test]
fn preserves_defs_name_order() {
    let mut value = json!({
        "$defs": {
            "zebra": { "type": "string" },
            "apple": { "type": "number" }
        }
    });

    json_schema_sort::sort_schema_with_options(&mut value, preserve_options());

    let defs = value["$defs"].as_object().unwrap();
    let keys: Vec<_> = defs.keys().cloned().collect();
    assert_eq!(keys, ["zebra", "apple"]);
}

#[test]
fn property_named_like_a_keyword_is_treated_as_a_property() {
    // A property literally named `type` is an identifier, not the `type`
    // keyword: it must sort alphabetically among siblings (not by keyword
    // rank) and its array value must not be reordered as a schema-safe array.
    let mut value = json!({
        "properties": {
            "type": { "enum": ["b", "a"] },
            "id": { "type": "string" }
        }
    });

    json_schema_sort::sort_schema(&mut value);

    let properties = value["properties"].as_object().unwrap();
    let keys: Vec<_> = properties.keys().cloned().collect();
    assert_eq!(keys, ["id", "type"]);
    // The value under the `type` property is a real subschema, and its `enum`
    // payload stays opaque/untouched.
    assert_eq!(value["properties"]["type"]["enum"], json!(["b", "a"]));
}

#[test]
fn sorts_required_arrays() {
    let mut value = json!({
        "required": ["z", "a", "m"]
    });

    json_schema_sort::sort_schema(&mut value);

    assert_eq!(value["required"], json!(["a", "m", "z"]));
}

#[test]
fn groups_applicators_and_validation_by_vocabulary() {
    // Array applicators (prefixItems/items/contains) belong to the applicator
    // vocabulary and sort together, separate from the array-size constraints
    // (minItems/maxItems) which belong to the validation vocabulary.
    let mut value = json!({
        "minItems": 1,
        "allOf": [],
        "type": "array",
        "items": { "type": "string" },
        "title": "Tags"
    });

    json_schema_sort::sort_schema(&mut value);

    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["title", "type", "minItems", "items", "allOf"]);
}

#[test]
fn orders_min_before_max_and_properties_before_additional() {
    let mut value = json!({
        "maximum": 10,
        "minimum": 1,
        "additionalProperties": false,
        "properties": {},
        "maxItems": 5,
        "minItems": 0
    });

    json_schema_sort::sort_schema(&mut value);

    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "minimum",
            "maximum",
            "minItems",
            "maxItems",
            "properties",
            "additionalProperties"
        ]
    );
}

#[test]
fn ranks_known_tooling_extensions_after_standard_keywords() {
    let mut value = json!({
        "x-taplo": {},
        "markdownDescription": "**hi**",
        "zzz-unknown": true,
        "description": "hi",
        "type": "string"
    });

    json_schema_sort::sort_schema(&mut value);

    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    // markdownDescription is a shadow of description and sorts next to it;
    // x-taplo has no standard sibling and falls into the trailing extension
    // bucket; truly-unknown keys sort alphabetically last.
    assert_eq!(
        keys,
        [
            "description",
            "markdownDescription",
            "type",
            "x-taplo",
            "zzz-unknown"
        ]
    );
}

#[test]
fn does_not_sort_enum_arrays() {
    let mut value = json!({
        "enum": ["bronze", "silver", "gold"]
    });

    json_schema_sort::sort_schema(&mut value);

    assert_eq!(value["enum"], json!(["bronze", "silver", "gold"]));
}

#[test]
fn does_not_reorder_object_keys_inside_opaque_values() {
    let input = r#"{
  "enum": [
    { "z": 1, "a": 2 }
  ],
  "default": {
    "z": 1,
    "a": 2
  },
  "examples": [
    { "z": 1, "a": 2 }
  ],
  "const": {
    "z": 1,
    "a": 2
  }
}"#;

    let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
    json_schema_sort::sort_schema(&mut value);

    let enum_keys: Vec<_> = value["enum"][0]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let default_keys: Vec<_> = value["default"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let example_keys: Vec<_> = value["examples"][0]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let const_keys: Vec<_> = value["const"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();

    assert_eq!(enum_keys, ["z", "a"]);
    assert_eq!(default_keys, ["z", "a"]);
    assert_eq!(example_keys, ["z", "a"]);
    assert_eq!(const_keys, ["z", "a"]);
}

#[test]
fn still_sorts_schema_keys_around_opaque_values() {
    let mut value = json!({
        "type": "object",
        "default": { "z": 1, "a": 2 },
        "$id": "https://example.com/widget"
    });

    json_schema_sort::sort_schema(&mut value);

    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["$id", "default", "type"]);
}
