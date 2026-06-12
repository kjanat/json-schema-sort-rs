//! End-to-end CLI tests: directory/glob expansion, `--write`/`--check`
//! aggregation across many files, and the `--preserve-property-order` flag.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const BIN: &str = env!("CARGO_BIN_EXE_json-schema-sort");
const UNSORTED: &str = r#"{"type":"object","$id":"x"}"#;
const SORTED: &str = "{\n  \"$id\": \"x\",\n  \"type\": \"object\"\n}\n";

/// Fresh, unique working directory per test under the cargo-provided tmp dir.
fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

#[test]
fn write_recurses_into_a_directory() {
    let dir = workdir("write_dir");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).unwrap();
    write(&dir.join("a.json"), UNSORTED);
    write(&nested.join("b.json"), UNSORTED);

    let status = Command::new(BIN).arg("--write").arg(&dir).status().unwrap();
    assert!(status.success());

    assert_eq!(fs::read_to_string(dir.join("a.json")).unwrap(), SORTED);
    assert_eq!(fs::read_to_string(nested.join("b.json")).unwrap(), SORTED);
}

#[test]
fn check_fails_and_names_each_unsorted_file_via_glob() {
    let dir = workdir("check_glob");
    write(&dir.join("bad.json"), UNSORTED);
    write(&dir.join("good.json"), SORTED);

    let output = Command::new(BIN)
        .arg("--check")
        .arg(dir.join("*.json"))
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "check should fail on unsorted file"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bad.json"), "stderr: {stderr}");
    assert!(!stderr.contains("good.json"), "stderr: {stderr}");
}

#[test]
fn refuses_to_print_multiple_files() {
    let dir = workdir("multi_print");
    write(&dir.join("a.json"), SORTED);
    write(&dir.join("b.json"), SORTED);

    let output = Command::new(BIN).arg(&dir).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--write or --check"), "stderr: {stderr}");
}

#[test]
fn preserve_property_order_flag_via_stdin() {
    let input = r#"{"properties":{"z":{"type":"string"},"a":{"type":"number"}}}"#;

    let mut child = Command::new(BIN)
        .arg("--preserve-property-order")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let z = stdout.find("\"z\"").unwrap();
    let a = stdout.find("\"a\"").unwrap();
    assert!(
        z < a,
        "property order should be preserved (z before a):\n{stdout}"
    );
}

#[test]
fn errors_when_no_files_match() {
    let dir = workdir("no_match");
    let output = Command::new(BIN).arg(dir.join("*.json")).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no files matched"), "stderr: {stderr}");
}
