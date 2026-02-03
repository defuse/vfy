use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("backup-verify").unwrap()
}

fn testdata(scenario: &str) -> (String, String) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join(scenario);
    (
        base.join("a").to_str().unwrap().to_string(),
        base.join("b").to_str().unwrap().to_string(),
    )
}

/// Helper: check that no line matching `prefix` also contains `needle`.
fn no_line_has(output: &str, prefix: &str, needle: &str) -> bool {
    !output.lines().any(|l| l.contains(prefix) && l.contains(needle))
}

/// Helper: check that at least one line matches both `prefix` and `needle`.
fn some_line_has(output: &str, prefix: &str, needle: &str) -> bool {
    output.lines().any(|l| l.contains(prefix) && l.contains(needle))
}

#[test]
fn test_identical() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("MISSING-FILE").not()
                .and(predicate::str::contains("MISSING-DIR").not())
                .and(predicate::str::contains("EXTRA-FILE").not())
                .and(predicate::str::contains("EXTRA-DIR").not())
                .and(predicate::str::contains("DIFFERENT-FILE").not())
                .and(predicate::str::contains("ERROR").not())
                .and(predicate::str::contains("Original items processed: 3"))
                .and(predicate::str::contains("Backup items processed: 3"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 3"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_missing_file() {
    let (a, b) = testdata("missing");
    cmd()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("MISSING-FILE:")
                .and(predicate::str::contains("also_here.txt"))
                .and(predicate::str::contains("Original items processed: 2"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 1 (50.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 1"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_extras() {
    let (a, b) = testdata("extras");
    cmd()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("EXTRA-FILE:")
                .and(predicate::str::contains("extra.txt"))
                .and(predicate::str::contains("EXTRA-DIR:"))
                .and(predicate::str::contains("extra_dir"))
                .and(predicate::str::contains("MISSING-FILE").not())
                .and(predicate::str::contains("DIFFERENT-FILE").not())
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 4"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 3"))
                .and(predicate::str::contains("Similarities: 1"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_different_size() {
    let (a, b) = testdata("different_size");
    cmd()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SIZE]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("SAMPLE").not())
                .and(predicate::str::contains("HASH").not())
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 1 (100.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 0"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_different_content_no_check() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE").not()
                .and(predicate::str::contains("MISSING-FILE").not())
                .and(predicate::str::contains("ERROR").not())
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 1"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_different_content_hash() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [HASH]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 1 (100.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 0"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_different_content_sample() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SAMPLE]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 1 (100.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 0"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_different_size_and_hash() {
    let (a, b) = testdata("different_size");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SIZE, HASH]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Original items processed: 1"))
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 1 (100.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 0"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn test_verbose_dirs_only() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Should contain DEBUG for directory comparison
    assert!(output.contains("DEBUG: Comparing"), "Expected DEBUG: Comparing for dirs");

    // Should NOT contain DEBUG for file comparison (that requires -vv)
    assert!(!output.contains("DEBUG: Comparing file"), "Should not contain file-level DEBUG at -v");

    // Summary should be same as identical
    assert!(output.contains("Original items processed: 3"));
    assert!(output.contains("Backup items processed: 3"));
    assert!(output.contains("Missing/different: 0 (0.00%)"));
    assert!(output.contains("Extras: 0"));
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn test_verbose_files() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v", "-v"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Should contain DEBUG for directories
    assert!(output.contains("DEBUG: Comparing"), "Expected DEBUG: Comparing for dirs");

    // Should contain DEBUG for file comparison
    assert!(output.contains("DEBUG: Comparing file"), "Expected DEBUG: Comparing file at -vv");

    // Summary
    assert!(output.contains("Original items processed: 3"));
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn test_verbose_hashes() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v", "-v", "--all"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Should contain BLAKE3 hash lines (64-char hex strings)
    assert!(output.contains("DEBUG: BLAKE3"), "Expected BLAKE3 hash output");

    // The BLAKE3 hash of "hello world\n" is known
    // Let's just verify there are 64-char hex-looking strings
    let has_hash = output.lines().any(|line| {
        if line.contains("DEBUG: BLAKE3") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 3 && parts[2].len() == 64 && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    });
    assert!(has_hash, "Expected 64-char hex hash in BLAKE3 output");

    // Summary same as identical
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn test_nested() {
    let (a, b) = testdata("nested");
    let assert = cmd()
        .args([&a, &b])
        .assert()
        .code(1);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Check expected output lines
    assert!(output.contains("MISSING-DIR:") && output.contains("sub3"),
        "Expected MISSING-DIR for sub3");
    assert!(output.contains("MISSING-FILE:") && output.contains("missing.txt"),
        "Expected MISSING-FILE for missing.txt");
    assert!(output.contains("EXTRA-DIR:") && output.contains("sub2"),
        "Expected EXTRA-DIR for sub2");

    // At default verbosity, should NOT list contents of missing/extra dirs individually
    // sub3/deep/file.txt should not appear as MISSING-FILE (it's inside a MISSING-DIR)
    let missing_file_lines: Vec<&str> = output.lines()
        .filter(|l| l.contains("MISSING-FILE:"))
        .collect();
    let has_deep_file = missing_file_lines.iter().any(|l| l.contains("deep/file.txt"));
    assert!(!has_deep_file, "Should NOT list sub3/deep/file.txt as MISSING-FILE at default verbosity");

    let extra_file_lines: Vec<&str> = output.lines()
        .filter(|l| l.contains("EXTRA-FILE:"))
        .collect();
    let has_extra_inner = extra_file_lines.iter().any(|l| l.contains("sub2"));
    assert!(!has_extra_inner, "Should NOT list files inside sub2/ as EXTRA-FILE at default verbosity");

    // Summary checks
    assert!(output.contains("Original items processed: 6"), "Expected 6 original items, got: {}", output);
    assert!(output.contains("Backup items processed: 4"), "Expected 4 backup items, got: {}", output);
    assert!(output.contains("Missing/different: 4 (66.67%)"), "Expected 4 missing/different");
    assert!(output.contains("Extras: 2"), "Expected 2 extras");
    assert!(output.contains("Similarities: 2"), "Expected 2 similarities");
    assert!(output.contains("Skipped: 0"));
    assert!(output.contains("Errors: 0"));
}

#[test]
fn test_nested_vv() {
    let (a, b) = testdata("nested");
    let assert = cmd()
        .args([&a, &b, "-v", "-v"])
        .assert()
        .code(1);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // At -vv, should list contents of missing dirs
    assert!(output.contains("MISSING-DIR:") && output.contains("sub3"),
        "Expected MISSING-DIR for sub3");

    // Check for deep contents being listed at -vv
    let has_deep_missing = output.lines().any(|l| l.contains("MISSING-DIR:") && l.contains("deep"));
    assert!(has_deep_missing, "Expected MISSING-DIR for sub3/deep at -vv");

    let has_deep_file = output.lines().any(|l| l.contains("MISSING-FILE:") && l.contains("file.txt") && l.contains("deep"));
    assert!(has_deep_file, "Expected MISSING-FILE for sub3/deep/file.txt at -vv");

    // Extra dir contents
    let has_extra_file = output.lines().any(|l| l.contains("EXTRA-FILE:") && l.contains("extra.txt") && l.contains("sub2"));
    assert!(has_extra_file, "Expected EXTRA-FILE for sub2/extra.txt at -vv");

    // Summary should be same counts as non-verbose
    assert!(output.contains("Original items processed: 6"));
    assert!(output.contains("Backup items processed: 4"));
    assert!(output.contains("Missing/different: 4 (66.67%)"));
    assert!(output.contains("Extras: 2"));
    assert!(output.contains("Similarities: 2"));
}

// ============================================================
// Bug #1: --follow flag must actually traverse symlink-to-dirs
// ============================================================

#[test]
fn test_symlink_dir_no_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd()
        .args([&a, &b, "-v", "-v"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Without --follow, symlink-to-dir entries should produce SYMLINK: and not be traversed
    assert!(some_line_has(&output, "SYMLINK:", "/link_dir"),
        "Expected SYMLINK: for link_dir without --follow, got:\n{}", output);
}

#[test]
fn test_symlink_dir_with_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd()
        .args([&a, &b, "--follow", "-v", "-v"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // With --follow, symlink dirs should be traversed, NOT produce SYMLINK:
    assert!(!output.contains("SYMLINK:"),
        "Should not produce SYMLINK: with --follow, got:\n{}", output);

    // The contents inside the symlinked dir should be traversed
    // We should see a DEBUG: Comparing for files inside link_dir/
    // (check for "/link_dir/" to avoid matching the "symlink_dirs" scenario name)
    assert!(some_line_has(&output, "DEBUG: Comparing file", "/link_dir/"),
        "Expected traversal into link_dir with --follow, got:\n{}", output);
}

// ============================================================
// Bug #2: compare_file must report errors instead of swallowing them
// ============================================================

#[test]
fn test_unreadable_file_reports_error() {
    let (a, b) = testdata("unreadable");
    let assert = cmd()
        .args([&a, &b, "--all"])
        .assert();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // An unreadable backup file during hash comparison should produce ERROR:, not silence
    assert!(output.contains("ERROR:"),
        "Expected ERROR: for unreadable file, got:\n{}", output);
    assert!(output.contains("Errors: 1"),
        "Expected Errors: 1, got:\n{}", output);

    // It should NOT be reported as DIFFERENT-FILE (it's an error, not a difference)
    assert!(!some_line_has(&output, "DIFFERENT-FILE", "noperm.txt"),
        "Unreadable file should be ERROR not DIFFERENT-FILE, got:\n{}", output);
}

// ============================================================
// Bug #4: test_missing_file negative assertion should be per-line
// (This replaces the old test_missing_file — the old one is kept
//  but we add a stricter version here)
// ============================================================

#[test]
fn test_missing_file_no_false_positive() {
    // Verify that exists.txt never appears on a MISSING-FILE: line,
    // even if it appears elsewhere in the output (e.g. at higher verbosity)
    let (a, b) = testdata("missing");
    let assert = cmd()
        .args([&a, &b, "-v", "-v"])
        .assert()
        .code(1);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // exists.txt should NOT appear on any MISSING-FILE: line
    assert!(no_line_has(&output, "MISSING-FILE:", "exists.txt"),
        "exists.txt must not appear on a MISSING-FILE: line, got:\n{}", output);

    // also_here.txt SHOULD appear on a MISSING-FILE: line
    assert!(some_line_has(&output, "MISSING-FILE:", "also_here.txt"),
        "also_here.txt must appear on a MISSING-FILE: line, got:\n{}", output);
}

// ============================================================
// Bug #8: --ignore must validate paths exist and are within orig/backup
// ============================================================

#[test]
fn test_ignore_nonexistent_path_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist").or(predicate::str::contains("not found").or(predicate::str::contains("Cannot resolve"))));
}

#[test]
fn test_ignore_path_outside_trees_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not within"));
}

#[test]
fn test_ignore_works_in_backup_tree() {
    // --ignore should accept a path in either the original or backup tree
    let (a, b) = testdata("extras");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("extras");
    let ignore_path = base.join("b").join("extra_dir").to_str().unwrap().to_string();

    let assert = cmd()
        .args([&a, &b, "-i", &ignore_path])
        .assert();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // extra_dir should be skipped, not reported as EXTRA-DIR
    assert!(!some_line_has(&output, "EXTRA-DIR:", "extra_dir"),
        "extra_dir should be skipped via --ignore, got:\n{}", output);
    assert!(output.contains("SKIP:") || output.contains("Skipped: 1"),
        "Expected SKIP or Skipped: 1 for ignored extra_dir, got:\n{}", output);
}

// ============================================================
// Bug #10: test must verify actual BLAKE3 hash values
// ============================================================

#[test]
fn test_blake3_known_hash_values() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v", "-v", "--all"])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // BLAKE3 of "hello world\n" = dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355
    let hello_hash = "dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355";
    assert!(some_line_has(&output, "BLAKE3", hello_hash),
        "Expected known BLAKE3 hash {} for hello.txt, got:\n{}", hello_hash, output);

    // BLAKE3 of "nested file\n" = 4e3d3cb1a85c88eed97f72907ee8cd68b467b4dd6abb914f4c0c859aa13843e2
    let nested_hash = "4e3d3cb1a85c88eed97f72907ee8cd68b467b4dd6abb914f4c0c859aa13843e2";
    assert!(some_line_has(&output, "BLAKE3", nested_hash),
        "Expected known BLAKE3 hash {} for nested.txt, got:\n{}", nested_hash, output);
}

// ============================================================
// Bug #13: ERROR: lines must appear on stdout (and also stderr when piped)
// ============================================================

#[test]
fn test_errors_on_stdout() {
    let (a, b) = testdata("unreadable");
    let assert = cmd()
        .args([&a, &b, "--all"])
        .assert();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // ERROR: lines from comparison must always appear on stdout for analysis
    assert!(stdout.contains("ERROR:"),
        "Expected ERROR: on stdout, got stdout:\n{}", stdout);
}
