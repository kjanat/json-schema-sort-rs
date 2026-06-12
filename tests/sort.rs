use serde_json::json;

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
fn sorts_required_arrays() {
    let mut value = json!({
        "required": ["z", "a", "m"]
    });

    json_schema_sort::sort_schema(&mut value);

    assert_eq!(value["required"], json!(["a", "m", "z"]));
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
