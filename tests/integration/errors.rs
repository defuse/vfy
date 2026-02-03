use std::os::unix::fs::PermissionsExt;

use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};
use predicates::prelude::*;

/// Set up the unreadable fixture at runtime (git doesn't preserve 000 perms).
fn setup_unreadable() {
    let path = testdata_base("unreadable").join("b").join("noperm.txt");
    let perms = std::fs::Permissions::from_mode(0o000);
    std::fs::set_permissions(&path, perms).expect("failed to chmod noperm.txt");
}

/// Restore readable perms (cleanup for other tests / re-runs).
fn teardown_unreadable() {
    let path = testdata_base("unreadable").join("b").join("noperm.txt");
    let perms = std::fs::Permissions::from_mode(0o644);
    let _ = std::fs::set_permissions(&path, perms);
}

#[test]
fn unreadable_file_reports_error_not_diff() {
    setup_unreadable();
    let (a, b) = testdata("unreadable");
    let assert = cmd().args([&a, &b, "--all"]).assert();
    let output = stdout_of(&assert);
    teardown_unreadable();

    // ERROR: on stdout for piped analysis
    assert!(
        output.contains("ERROR:"),
        "Expected ERROR: for unreadable file, got:\n{}",
        output
    );
    assert!(
        output.contains("Errors: 1"),
        "Expected Errors: 1, got:\n{}",
        output
    );
    // NOT reported as a content difference
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "noperm.txt"),
        "Unreadable file should be ERROR not DIFFERENT-FILE"
    );
}

// ── Type mismatch ────────────────────────────────────────────

#[test]
fn dir_in_original_file_in_backup() {
    let (a, b) = testdata("type_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // name_a is a dir in a/, a file in b/
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "name_a"),
        "Expected DIFFERENT-FILE [TYPE] for name_a (dir vs file), got:\n{}",
        output
    );
    assert!(output.contains("dir vs file"), "Expected 'dir vs file'");
}

#[test]
fn file_in_original_dir_in_backup() {
    let (a, b) = testdata("type_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // name_b is a file in a/, a dir in b/
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "name_b"),
        "Expected DIFFERENT-FILE [TYPE] for name_b (file vs dir), got:\n{}",
        output
    );
    assert!(output.contains("file vs dir"), "Expected 'file vs dir'");
}

#[test]
fn type_mismatch_summary() {
    let (a, b) = testdata("type_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // a/ has: name_a/ (dir), name_b (file), same.txt (file) = 3 items
    // All 3 exist in backup
    // name_a: TYPE mismatch (different), name_b: TYPE mismatch (different), same.txt: match
    assert!(output.contains("Original items processed: 3"), "got:\n{}", output);
    assert!(output.contains("Backup items processed: 3"), "got:\n{}", output);
    assert!(output.contains("Missing/different: 2"), "got:\n{}", output);
    assert!(output.contains("Similarities: 1"), "got:\n{}", output);
}

// ── CLI validation ───────────────────────────────────────────

#[test]
fn nonexistent_original_exits_2() {
    cmd()
        .args(["/nonexistent/dir/orig", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Cannot resolve"));
}

#[test]
fn original_is_file_not_dir() {
    // Use an existing file as the "original" argument
    let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("Cargo.toml")
        .to_str()
        .unwrap()
        .to_string();
    let (_, b) = testdata("identical");

    cmd()
        .args([&file_path, &b])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is not a directory"));
}
