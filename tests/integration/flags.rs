use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};
use predicates::prelude::*;

// ── Verbosity ────────────────────────────────────────────────

#[test]
fn verbose_dirs_only() {
    let (a, b) = testdata("identical");
    let assert = cmd().args([&a, &b, "-v"]).assert().success();
    let output = stdout_of(&assert);

    assert!(output.contains("DEBUG: Comparing"));
    assert!(
        !output.contains("DEBUG: Comparing file"),
        "File-level DEBUG requires -vv"
    );
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn verbose_files() {
    let (a, b) = testdata("identical");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    assert!(output.contains("DEBUG: Comparing file"));
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn verbose_blake3_known_hashes() {
    let (a, b) = testdata("identical");
    let assert = cmd()
        .args([&a, &b, "-v", "-v", "--all"])
        .assert()
        .success();
    let output = stdout_of(&assert);

    // BLAKE3("hello world\n")
    let hello_hash = "dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355";
    // BLAKE3("nested file\n")
    let nested_hash = "4e3d3cb1a85c88eed97f72907ee8cd68b467b4dd6abb914f4c0c859aa13843e2";

    assert!(
        some_line_has(&output, "BLAKE3", hello_hash),
        "Expected known BLAKE3 hash for hello.txt"
    );
    assert!(
        some_line_has(&output, "BLAKE3", nested_hash),
        "Expected known BLAKE3 hash for nested.txt"
    );

    // Verify format: 64-char hex
    let has_valid_format = output.lines().any(|line| {
        if line.contains("DEBUG: BLAKE3") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 3
                && parts[2].len() == 64
                && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    });
    assert!(has_valid_format, "BLAKE3 lines must contain 64-char hex");

    assert!(output.contains("Similarities: 3"));
}

// ── --all and -s combinations ────────────────────────────────

#[test]
fn sample_and_hash_combined() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "-s", "10", "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SAMPLE, HASH]:")
                .and(predicate::str::contains("Missing/different: 1 (100.00%)")),
        );
}

#[test]
fn sample_on_identical_content() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("Similarities: 3")),
        );
}

// ── --ignore ─────────────────────────────────────────────────

#[test]
fn ignore_nonexistent_path_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist").or(
            predicate::str::contains("not found").or(predicate::str::contains("Cannot resolve")),
        ));
}

#[test]
fn ignore_path_outside_trees_errors() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b, "-i", "/tmp"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not within"));
}

#[test]
fn ignore_works_in_backup_tree() {
    let (a, b) = testdata("extras");
    let base = testdata_base("extras");
    let ignore_path = base.join("b").join("extra_dir").to_str().unwrap().to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "EXTRA-DIR:", "extra_dir"),
        "extra_dir should be skipped via --ignore"
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for ignored extra_dir, got:\n{}",
        output
    );
}

#[test]
fn ignore_works_in_original_tree() {
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    // Ignore sub3 in the original tree — it's a missing dir with nested contents
    let ignore_path = base.join("a").join("sub3").to_str().unwrap().to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped via --ignore"
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for ignored sub3, got:\n{}",
        output
    );
    // sub3/ had 3 missing items (sub3/ + deep/ + file.txt); ignoring it should reduce counts
    // Original: sub1/ + ok.txt + missing.txt = 3 (sub3 skipped)
    // Missing: missing.txt = 1
    assert!(
        output.contains("Original items processed: 3"),
        "Expected 3 original items with sub3 ignored, got:\n{}",
        output
    );
}

#[test]
fn ignore_a_file_not_directory() {
    // --ignore on a specific file (not a directory)
    let (a, b) = testdata("missing");
    let base = testdata_base("missing");
    let ignore_path = base
        .join("a")
        .join("also_here.txt")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd().args([&a, &b, "-i", &ignore_path]).assert();
    let output = stdout_of(&assert);

    // also_here.txt should be skipped, not reported as MISSING-FILE
    assert!(
        !some_line_has(&output, "MISSING-FILE:", "also_here.txt"),
        "also_here.txt should be skipped via --ignore, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1, got:\n{}",
        output
    );
    // Only exists.txt remains as an original item
    assert!(
        output.contains("Original items processed: 1"),
        "got:\n{}",
        output
    );
}

#[test]
fn ignore_multiple_paths() {
    // -i path1 -i path2
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    let ignore1 = base
        .join("a")
        .join("sub3")
        .to_str()
        .unwrap()
        .to_string();
    let ignore2 = base
        .join("b")
        .join("sub2")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd()
        .args([&a, &b, "-i", &ignore1, "-i", &ignore2])
        .assert();
    let output = stdout_of(&assert);

    assert!(
        !some_line_has(&output, "MISSING-DIR:", "sub3"),
        "sub3 should be skipped, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "EXTRA-DIR:", "sub2"),
        "sub2 should be skipped, got:\n{}",
        output
    );
    assert!(
        output.contains("Skipped: 2"),
        "Expected Skipped: 2, got:\n{}",
        output
    );
}

#[test]
fn all_with_ignore_skips_hashing() {
    // --all combined with --ignore: ignored entries should not produce BLAKE3 lines
    let (a, b) = testdata("nested");
    let base = testdata_base("nested");
    let ignore_path = base
        .join("a")
        .join("sub3")
        .to_str()
        .unwrap()
        .to_string();

    let assert = cmd()
        .args([&a, &b, "--all", "-v", "-v", "-i", &ignore_path])
        .assert();
    let output = stdout_of(&assert);

    // sub3 should not appear in any BLAKE3 line
    assert!(
        !some_line_has(&output, "BLAKE3", "sub3"),
        "sub3 should be skipped, no BLAKE3 hashing, got:\n{}",
        output
    );
    assert!(
        output.contains("SKIP:"),
        "Expected SKIP: for ignored path, got:\n{}",
        output
    );
}
