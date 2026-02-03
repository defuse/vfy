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

    // a/ has: root + name_a/ (dir) + name_a/child.txt + name_b (file) + same.txt = 5 items
    // b/ has: root + name_a (file) + name_b/ (dir) + name_b/child.txt + same.txt = 5 items
    // name_a: TYPE mismatch, a/name_a/child.txt is missing from backup
    // name_b: TYPE mismatch, b/name_b/child.txt is extra in backup
    assert!(output.contains("Original items processed: 5"), "got:\n{}", output);
    assert!(output.contains("Backup items processed: 5"), "got:\n{}", output);
    // Missing/different: name_a (type) + name_a/child.txt (missing) + name_b (type) = 3
    assert!(output.contains("Missing/different: 3"), "got:\n{}", output);
    // Extras: name_b/child.txt = 1
    assert!(output.contains("Extras: 1"), "got:\n{}", output);
    // Similarities: root + same.txt = 2
    assert!(output.contains("Similarities: 2"), "got:\n{}", output);
}

#[test]
fn type_mismatch_dir_orig_counts_missing_contents() {
    // When original has a dir and backup has a file with the same name,
    // the directory's contents should be counted as missing.
    let (a, b) = testdata("type_mismatch");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    // name_a is a dir in a/ with child.txt, a file in b/
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "name_a"),
        "Expected type mismatch for name_a, got:\n{}",
        output
    );
    // child.txt inside the dir should be counted as missing
    assert!(
        some_line_has(&output, "MISSING-FILE:", "child.txt"),
        "Expected MISSING-FILE for child.txt inside type-mismatched dir, got:\n{}",
        output
    );
}

#[test]
fn type_mismatch_dir_backup_counts_extra_contents() {
    // When original has a file and backup has a dir with the same name,
    // the directory's contents should be counted as extras.
    let (a, b) = testdata("type_mismatch");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    // name_b is a file in a/, a dir in b/ with child.txt
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "name_b"),
        "Expected type mismatch for name_b, got:\n{}",
        output
    );
    // child.txt inside the backup dir should be counted as extra
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "child.txt"),
        "Expected EXTRA-FILE for child.txt inside type-mismatched backup dir, got:\n{}",
        output
    );
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

#[test]
fn nonexistent_backup_exits_2() {
    let (a, _) = testdata("identical");
    cmd()
        .args([&a, "/nonexistent/dir/backup"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Cannot resolve"));
}

#[test]
fn backup_is_file_not_dir() {
    let (a, _) = testdata("identical");
    let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("Cargo.toml")
        .to_str()
        .unwrap()
        .to_string();

    cmd()
        .args([&a, &file_path])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is not a directory"));
}

// ── Error counting edge cases ───────────────────────────────

#[test]
fn errors_cause_nonzero_exit() {
    // Errors should cause a non-zero exit even when there are no missing/different/extras.
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
    // Errors alone should trigger non-zero exit
    assert_eq!(
        code, 1,
        "Expected exit 1 when errors occurred, got {}",
        code
    );
}

#[test]
fn error_file_not_counted_as_similarity() {
    // When compare_file returns None (error), the file must NOT be counted as a similarity.
    // Similarities should only count items that were actually verified to match.
    let _lock = UNREADABLE_LOCK.lock().unwrap();
    setup_unreadable_file();
    let (a, b) = testdata("unreadable");
    let assert = cmd().args([&a, &b, "--all"]).assert();
    let output = stdout_of(&assert);
    teardown_unreadable_file();

    // 3 original items: root + file.txt + noperm.txt
    // noperm.txt errors → not a similarity
    // root + file.txt match → 2 similarities
    assert!(
        output.contains("Original items processed: 3"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 2"),
        "Errored file should not count as similarity, got:\n{}",
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
