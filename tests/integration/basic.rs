use predicates::prelude::*;

use super::{cmd, no_line_has, some_line_has, stdout_of, testdata};

#[test]
fn identical() {
    let (a, b) = testdata("identical");
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("MISSING-FILE")
                .not()
                .and(predicate::str::contains("MISSING-DIR").not())
                .and(predicate::str::contains("EXTRA-FILE").not())
                .and(predicate::str::contains("EXTRA-DIR").not())
                .and(predicate::str::contains("DIFFERENT-FILE").not())
                .and(predicate::str::contains("ERROR").not())
                .and(predicate::str::contains("Original items processed: 4"))
                .and(predicate::str::contains("Backup items processed: 4"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 0"))
                .and(predicate::str::contains("Similarities: 4"))
                .and(predicate::str::contains("Skipped: 0"))
                .and(predicate::str::contains("Errors: 0")),
        );
}

#[test]
fn missing_file() {
    let (a, b) = testdata("missing");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    // Per-line: also_here.txt IS on a MISSING-FILE: line
    assert!(
        some_line_has(&output, "MISSING-FILE:", "also_here.txt"),
        "Expected MISSING-FILE for also_here.txt"
    );
    // Per-line: exists.txt is NOT on any MISSING-FILE: line
    assert!(
        no_line_has(&output, "MISSING-FILE:", "exists.txt"),
        "exists.txt must not appear on a MISSING-FILE: line"
    );

    assert!(output.contains("Original items processed: 3"));
    assert!(output.contains("Backup items processed: 2"));
    assert!(output.contains("Missing/different: 1 (33.33%)"));
    assert!(output.contains("Extras: 0"));
    assert!(output.contains("Similarities: 2"));
}

#[test]
fn extras() {
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
                .and(predicate::str::contains("Original items processed: 2"))
                .and(predicate::str::contains("Backup items processed: 5"))
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Extras: 3"))
                .and(predicate::str::contains("Similarities: 2")),
        );
}

#[test]
fn different_size() {
    let (a, b) = testdata("different_size");
    cmd()
        .args([&a, &b])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SIZE]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Original items processed: 2"))
                .and(predicate::str::contains("Missing/different: 1 (50.00%)"))
                .and(predicate::str::contains("Similarities: 1")),
        );
}

#[test]
fn different_content_no_check() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("DIFFERENT-FILE")
                .not()
                .and(predicate::str::contains("Missing/different: 0 (0.00%)"))
                .and(predicate::str::contains("Similarities: 2")),
        );
}

#[test]
fn different_content_hash() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [HASH]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Missing/different: 1 (50.00%)"))
                .and(predicate::str::contains("Similarities: 1")),
        );
}

#[test]
fn different_content_sample() {
    let (a, b) = testdata("different_content");
    cmd()
        .args([&a, &b, "-s", "10"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SAMPLE]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Missing/different: 1 (50.00%)"))
                .and(predicate::str::contains("Similarities: 1")),
        );
}

#[test]
fn different_size_and_hash() {
    let (a, b) = testdata("different_size");
    cmd()
        .args([&a, &b, "--all"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("DIFFERENT-FILE [SIZE]:")
                .and(predicate::str::contains("file.txt"))
                .and(predicate::str::contains("Missing/different: 1 (50.00%)")),
        );
}

#[test]
fn hash_catches_single_byte_difference() {
    // 1 MB files identical except for one byte near the end.
    // -s 1 is overwhelmingly likely to miss the difference (samples 32 bytes
    // out of 1 MB), but --all must catch it via BLAKE3.
    let tmp = std::env::temp_dir().join("bv_test_hash_1mb");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let size = 1_000_000;
    let mut data = vec![0u8; size];
    std::fs::write(a.join("file.bin"), &data).unwrap();
    // Flip one byte near the end
    data[size - 37] = 0xFF;
    std::fs::write(b.join("file.bin"), &data).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "-s", "1", "--all"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Hash must catch it — sampling alone almost certainly won't
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [HASH]:", "file.bin"),
        "Expected HASH to catch single-byte difference, got:\n{}",
        output
    );
    // Should NOT be SIZE (same length)
    assert!(
        no_line_has(&output, "DIFFERENT-FILE", "SIZE"),
        "Files are same size, should not report SIZE, got:\n{}",
        output
    );
}

#[test]
fn nested() {
    let (a, b) = testdata("nested");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    assert!(some_line_has(&output, "MISSING-DIR:", "sub3"));
    assert!(some_line_has(&output, "MISSING-FILE:", "missing.txt"));
    assert!(some_line_has(&output, "EXTRA-DIR:", "sub2"));

    // At default verbosity, contents inside missing/extra dirs are NOT listed
    assert!(!some_line_has(&output, "MISSING-FILE:", "deep/file.txt"));
    assert!(!some_line_has(&output, "EXTRA-FILE:", "sub2"));

    assert!(output.contains("Original items processed: 7"));
    assert!(output.contains("Backup items processed: 5"));
    assert!(output.contains("Missing/different: 4 (57.14%)"));
    assert!(output.contains("Extras: 2"));
    assert!(output.contains("Similarities: 3"));
}

#[test]
fn nested_vv() {
    let (a, b) = testdata("nested");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    // At -vv, contents of missing/extra dirs ARE listed
    assert!(output
        .lines()
        .any(|l| l.contains("MISSING-DIR:") && l.contains("deep")));
    assert!(output
        .lines()
        .any(|l| l.contains("MISSING-FILE:") && l.contains("deep") && l.contains("file.txt")));
    assert!(output
        .lines()
        .any(|l| l.contains("EXTRA-FILE:") && l.contains("sub2") && l.contains("extra.txt")));

    // Summary counts unchanged by verbosity
    assert!(output.contains("Original items processed: 7"));
    assert!(output.contains("Missing/different: 4 (57.14%)"));
    assert!(output.contains("Extras: 2"));
    assert!(output.contains("Similarities: 3"));
}
