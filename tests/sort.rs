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
    assert_eq!(keys, ["$schema", "title", "description", "type", "properties"]);
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
