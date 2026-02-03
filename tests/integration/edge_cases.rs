use super::{cmd, some_line_has, stdout_of, testdata, testdata_base};
use predicates::prelude::*;

#[test]
fn empty_directories() {
    // Git can't track empty directories, so create them at runtime
    let base = testdata_base("empty");
    let a = base.join("a");
    let b = base.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    // Remove any .gitkeep files that might exist
    let _ = std::fs::remove_file(a.join(".gitkeep"));
    let _ = std::fs::remove_file(b.join(".gitkeep"));

    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Original items processed: 1")
                .and(predicate::str::contains("Backup items processed: 1"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 1"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn same_directory_warning() {
    let (a, _) = testdata("identical");
    cmd()
        .args([&a, &a])
        .assert()
        .success()
        .stderr(predicate::str::contains("same directory"));
}

#[test]
fn output_is_sorted() {
    // sorted/ has alpha.txt, bravo.txt, charlie.txt in a/ but only charlie.txt in b/
    // MISSING-FILE lines should appear in alphabetical order
    let (a, b) = testdata("sorted");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    let missing_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.contains("MISSING-FILE:"))
        .collect();

    assert_eq!(
        missing_lines.len(),
        2,
        "Expected 2 MISSING-FILE lines, got: {:?}",
        missing_lines
    );

    // alpha.txt must come before bravo.txt
    let alpha_pos = output.find("alpha.txt").expect("alpha.txt not in output");
    let bravo_pos = output.find("bravo.txt").expect("bravo.txt not in output");
    assert!(
        alpha_pos < bravo_pos,
        "alpha.txt should appear before bravo.txt in sorted output"
    );
}

// ── Zero-byte files ─────────────────────────────────────────

#[test]
fn zero_byte_files_with_all() {
    let (a, b) = testdata("empty_files");
    let assert = cmd().args([&a, &b, "--all", "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    // Two identical empty files should match
    assert!(
        !output.contains("DIFFERENT-FILE"),
        "Empty files should match, got:\n{}",
        output
    );
    assert!(output.contains("Similarities: 2"), "got:\n{}", output);
    assert!(output.contains("Errors: 0"), "got:\n{}", output);

    // Verify the known BLAKE3 hash of empty input
    // BLAKE3("") = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
    let empty_hash = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
    assert!(
        some_line_has(&output, "BLAKE3", empty_hash),
        "Expected known BLAKE3 hash for empty file, got:\n{}",
        output
    );
}

#[test]
fn zero_byte_files_with_sampling() {
    let (a, b) = testdata("empty_files");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("Similarities: 2"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

// ── Extras with zero original items ─────────────────────────

#[test]
fn extras_with_zero_originals() {
    // a/ is empty, b/ has files → percentage 0.00% but exit 1 due to extras
    let base = testdata_base("extras_only");
    let a = base.join("a");
    let b = base.join("b");
    // Ensure empty dir exists (git can't track empty dirs)
    std::fs::create_dir_all(&a).unwrap();
    let _ = std::fs::remove_file(a.join(".gitkeep"));

    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    assert!(
        output.contains("Original items processed: 1"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Missing/different: 0 (0.00%)"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Extras: 3"), "got:\n{}", output);
}

// ── Mixed results in one directory ──────────────────────────

#[test]
fn mixed_results_per_file() {
    let (a, b) = testdata("mixed_results");
    let assert = cmd().args([&a, &b, "-s", "10", "--all"]).assert().code(1);
    let output = stdout_of(&assert);

    // match.txt should not appear as DIFFERENT-FILE
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "match.txt"),
        "match.txt should not differ, got:\n{}",
        output
    );
    // size_diff.txt should differ by SIZE
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "size_diff.txt"),
        "Expected DIFFERENT-FILE for size_diff.txt, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "SIZE", "size_diff.txt"),
        "Expected SIZE reason for size_diff.txt, got:\n{}",
        output
    );
    // content_diff.txt should differ by SAMPLE and/or HASH
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "content_diff.txt"),
        "Expected DIFFERENT-FILE for content_diff.txt, got:\n{}",
        output
    );

    // Summary: 4 items (root + 3), 2 different, 2 similarities (root + match.txt)
    assert!(
        output.contains("Original items processed: 4"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Backup items processed: 4"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Missing/different: 2"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Similarities: 2"), "got:\n{}", output);
}

// ── Deeply nested identical trees ───────────────────────────

#[test]
fn deep_identical_tree() {
    let (a, b) = testdata("deep_identical");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("MISSING").not())
                .and(predicate::str::contains("EXTRA").not())
                .and(predicate::str::contains("ERROR").not())
                // root + l1/ + l2/ + l3/ + deep.txt + mid.txt + top.txt = 7
                .and(predicate::str::contains("Original items processed: 7"))
                .and(predicate::str::contains("Backup items processed: 7"))
                .and(predicate::str::contains("Similarities: 7"))
                .and(predicate::str::contains("Errors: 0")),
        );
}
