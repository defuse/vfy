use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};
use predicates::prelude::*;

/// Mutex to serialize tests that share the `unreadable` fixture's noperm.txt.
/// Tests run in parallel by default; without this lock, one test's teardown
/// can restore permissions while another test is between setup and execution.
static UNREADABLE_LOCK: Mutex<()> = Mutex::new(());

/// Set up the unreadable file fixture at runtime (git doesn't preserve 000 perms).
/// Also cleans up any stale dirs from unreadable-dir tests to avoid interference.
fn setup_unreadable_file() {
    let base = testdata_base("unreadable");
    // Clean up any stale unreadable dirs that other tests might have left
    for side in &["a", "b"] {
        for name in &["noread_dir", "noread_dir_b"] {
            let path = base.join(side).join(name);
            if path.exists() {
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
    let path = base.join("b").join("noperm.txt");
    let perms = std::fs::Permissions::from_mode(0o000);
    std::fs::set_permissions(&path, perms).expect("failed to chmod noperm.txt");
}

/// Restore readable perms (cleanup for other tests / re-runs).
fn teardown_unreadable_file() {
    let path = testdata_base("unreadable").join("b").join("noperm.txt");
    let perms = std::fs::Permissions::from_mode(0o644);
    let _ = std::fs::set_permissions(&path, perms);
}

#[test]
fn unreadable_file_reports_error_not_diff() {
    let _lock = UNREADABLE_LOCK.lock().unwrap();
    setup_unreadable_file();
    let (a, b) = testdata("unreadable");
    let assert = cmd().args([&a, &b, "--all"]).assert();
    let output = stdout_of(&assert);
    teardown_unreadable_file();

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

// ── Error counting edge cases ───────────────────────────────

#[test]
fn errors_only_does_not_exit_1() {
    // When only errors occur (no missing/different/extras), exit code should be 0
    // because has_differences() only checks missing/different/extras, not errors.
    let _lock = UNREADABLE_LOCK.lock().unwrap();
    setup_unreadable_file();
    let (a, b) = testdata("unreadable");
    let assert = cmd().args([&a, &b, "--all"]).assert();
    let output = stdout_of(&assert);
    let code = assert.get_output().status.code().unwrap();
    teardown_unreadable_file();

    assert!(
        output.contains("Errors: 1"),
        "Expected Errors: 1, got:\n{}",
        output
    );
    // file.txt matches, noperm.txt errors → no missing/different/extras
    assert!(
        output.contains("Missing/different: 0"),
        "Expected no differences, got:\n{}",
        output
    );
    assert!(
        output.contains("Extras: 0"),
        "Expected no extras, got:\n{}",
        output
    );
    // Exit code 0 because has_differences doesn't count errors
    assert_eq!(
        code, 0,
        "Expected exit 0 with errors-only (no differences), got {}",
        code
    );
}

#[test]
fn error_file_counted_as_similarity() {
    // When compare_file returns None (error), the file is neither missing nor different,
    // so it inflates the similarities count. This test documents that behavior.
    let _lock = UNREADABLE_LOCK.lock().unwrap();
    setup_unreadable_file();
    let (a, b) = testdata("unreadable");
    let assert = cmd().args([&a, &b, "--all"]).assert();
    let output = stdout_of(&assert);
    teardown_unreadable_file();

    // 2 original items: file.txt + noperm.txt
    // noperm.txt errors, file.txt matches
    // similarities = original - missing - different = 2 - 0 - 0 = 2
    // This means the errored file is counted as a "similarity" in the summary
    assert!(
        output.contains("Original items processed: 2"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 2"),
        "Errored file inflates similarities (orig - missing - different), got:\n{}",
        output
    );
}

// ── Unreadable directories ──────────────────────────────────

#[test]
fn unreadable_directory_in_original() {
    // Use a temporary directory to avoid interfering with other tests
    let tmp = std::env::temp_dir().join("bv_test_unreadable_dir_orig");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("noread_dir")).unwrap();
    std::fs::create_dir_all(b.join("noread_dir")).unwrap();
    std::fs::write(a.join("noread_dir").join("hidden.txt"), "hidden\n").unwrap();
    std::fs::write(b.join("noread_dir").join("hidden.txt"), "hidden\n").unwrap();
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    // Make a's noread_dir unreadable
    let perms = std::fs::Permissions::from_mode(0o000);
    std::fs::set_permissions(a.join("noread_dir"), perms).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert();
    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::set_permissions(
        a.join("noread_dir"),
        std::fs::Permissions::from_mode(0o755),
    );
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "ERROR:", "noread_dir"),
        "Expected ERROR for unreadable directory in original, got:\n{}",
        output
    );
}

#[test]
fn unreadable_directory_in_backup() {
    // Use a temporary directory to avoid interfering with other tests
    let tmp = std::env::temp_dir().join("bv_test_unreadable_dir_backup");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(a.join("noread_dir")).unwrap();
    std::fs::create_dir_all(b.join("noread_dir")).unwrap();
    std::fs::write(a.join("noread_dir").join("file.txt"), "test\n").unwrap();
    std::fs::write(b.join("noread_dir").join("file.txt"), "test\n").unwrap();
    std::fs::write(a.join("ok.txt"), "ok\n").unwrap();
    std::fs::write(b.join("ok.txt"), "ok\n").unwrap();

    // Make only b's noread_dir unreadable
    let perms = std::fs::Permissions::from_mode(0o000);
    std::fs::set_permissions(b.join("noread_dir"), perms).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert();
    let output = stdout_of(&assert);

    // Cleanup
    let _ = std::fs::set_permissions(
        b.join("noread_dir"),
        std::fs::Permissions::from_mode(0o755),
    );
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "ERROR:", "noread_dir"),
        "Expected ERROR for unreadable backup directory, got:\n{}",
        output
    );
}
