//! End-to-end snapshot test. `schema.unsorted.json` is the input and
//! `schema.sorted.json` is the expected output of sorting it. Together they
//! pin the full ordering behavior in one place.
//!
//! To regenerate the expected snapshot after an intentional change, run:
//!
//! ```sh
//! UPDATE_FIXTURES=1 cargo test --test snapshot
//! ```

use std::{fs, path::PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn sorts_full_schema_to_snapshot() {
    let input = fs::read_to_string(fixture_path("schema.unsorted.json")).unwrap();
    let actual = json_schema_sort::to_string_sorted(&input).unwrap();

    let expected_path = fixture_path("schema.sorted.json");

    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        fs::write(&expected_path, &actual).unwrap();
    }

    let expected = fs::read_to_string(&expected_path).unwrap();
    assert_eq!(
        actual, expected,
        "sorted output does not match snapshot; rerun with UPDATE_FIXTURES=1 to update"
    );
}

#[test]
fn sorting_the_snapshot_is_idempotent() {
    let expected = fs::read_to_string(fixture_path("schema.sorted.json")).unwrap();
    let actual = json_schema_sort::to_string_sorted(&expected).unwrap();
    assert_eq!(
        actual, expected,
        "sorting an already-sorted schema changed it"
    );
}
