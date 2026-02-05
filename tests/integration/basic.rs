//! Basic integration tests using the case! macro infrastructure.
//!
//! These tests cover fundamental scenarios: identical files, missing files,
//! extras, different sizes/content, and nested directories.

use super::harness::Entry::*;
use crate::case;

// testdata/identical: hello.txt + sub/ + sub/nested.txt (identical both sides)
// Items: root dir + hello.txt + sub/ + sub/nested.txt = 4
case!(identical {
    orig: [
        File("hello.txt", "hello world\n"),
        Dir("sub"),
        File("sub/nested.txt", "nested file\n"),
    ],
    backup: [
        File("hello.txt", "hello world\n"),
        Dir("sub"),
        File("sub/nested.txt", "nested file\n"),
    ],
    flags: [],
    lines: [],
    original_processed: 4,
    backup_processed: 4,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 4,
    skipped: 0,
    errors: 0,
});

// testdata/missing: orig has exists.txt + also_here.txt, backup only has exists.txt
// Items: root dir + files = 3 orig, 2 backup
case!(missing_file {
    orig: [
        File("exists.txt", "I exist\n"),
        File("also_here.txt", "me too\n"),
    ],
    backup: [
        File("exists.txt", "I exist\n"),
    ],
    flags: [],
    lines: [
        "MISSING-FILE: a/also_here.txt",
    ],
    original_processed: 3,
    backup_processed: 2,
    missing: 1,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// testdata/extras: orig has base.txt, backup has base.txt + extra.txt + extra_dir/
// extra_dir/ contains file.txt
// Items: orig = root + base.txt = 2
//        backup = root + base.txt + extra.txt + extra_dir + extra_dir/file.txt = 5
case!(extras {
    orig: [
        File("base.txt", "base content\n"),
    ],
    backup: [
        File("base.txt", "base content\n"),
        File("extra.txt", "I'm extra\n"),
        Dir("extra_dir"),
        File("extra_dir/file.txt", "extra dir file\n"),
    ],
    flags: [],
    lines: [
        "EXTRA-FILE: b/extra.txt",
        "EXTRA-DIR: b/extra_dir",
    ],
    original_processed: 2,
    backup_processed: 5,
    missing: 0,
    different: 0,
    extras: 3,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// testdata/different_size: same filename, different content lengths
// "short" (5 bytes) vs "this is a longer string" (23 bytes)
case!(different_size {
    orig: [
        File("file.txt", "short"),
    ],
    backup: [
        File("file.txt", "this is a longer string"),
    ],
    flags: [],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/file.txt",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// testdata/different_content: same size (4 bytes), different content
// "aaaa" vs "bbbb" - without --all or -s, no difference detected (size-only check)
case!(different_content_no_check {
    orig: [
        File("file.txt", "aaaa"),
    ],
    backup: [
        File("file.txt", "bbbb"),
    ],
    flags: [],
    lines: [],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 0,
    extras: 0,
    special_files: 0,
    similarities: 2,
    skipped: 0,
    errors: 0,
});

// With --all flag, hash comparison detects the difference
case!(different_content_hash {
    orig: [
        File("file.txt", "aaaa"),
    ],
    backup: [
        File("file.txt", "bbbb"),
    ],
    flags: ["--all"],
    lines: [
        "DIFFERENT-FILE [HASH]: a/file.txt",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// With -s (sample) flag, sampling comparison detects the difference
case!(different_content_sample {
    orig: [
        File("file.txt", "aaaa"),
    ],
    backup: [
        File("file.txt", "bbbb"),
    ],
    flags: ["-s", "10"],
    lines: [
        "DIFFERENT-FILE [SAMPLE]: a/file.txt",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// With --all on different sizes, SIZE is reported (not HASH)
case!(different_size_and_hash {
    orig: [
        File("file.txt", "short"),
    ],
    backup: [
        File("file.txt", "this is a longer string"),
    ],
    flags: ["--all"],
    lines: [
        "DIFFERENT-FILE [SIZE]: a/file.txt",
    ],
    original_processed: 2,
    backup_processed: 2,
    missing: 0,
    different: 1,
    extras: 0,
    special_files: 0,
    similarities: 1,
    skipped: 0,
    errors: 0,
});

// testdata/nested structure:
// orig: sub1/ok.txt, sub1/missing.txt, sub3/deep/file.txt
// backup: sub1/ok.txt, sub2/extra.txt
//
// Items in orig: root + sub1 + sub1/ok.txt + sub1/missing.txt + sub3 + sub3/deep + sub3/deep/file.txt = 7
// Items in backup: root + sub1 + sub1/ok.txt + sub2 + sub2/extra.txt = 5
//
// Missing: sub1/missing.txt (1) + sub3 (dir with deep/file.txt counted = 3) = 4 missing
// Extras: sub2 (dir + extra.txt inside = 2) = 2 extras
// Similarities: root + sub1 + sub1/ok.txt = 3
case!(nested {
    orig: [
        Dir("sub1"),
        File("sub1/ok.txt", "ok\n"),
        File("sub1/missing.txt", "missing\n"),
        Dir("sub3"),
        Dir("sub3/deep"),
        File("sub3/deep/file.txt", "deep\n"),
    ],
    backup: [
        Dir("sub1"),
        File("sub1/ok.txt", "ok\n"),
        Dir("sub2"),
        File("sub2/extra.txt", "extra\n"),
    ],
    flags: [],
    lines: [
        "MISSING-FILE: a/sub1/missing.txt",
        "MISSING-DIR: a/sub3",
        "EXTRA-DIR: b/sub2",
    ],
    original_processed: 7,
    backup_processed: 5,
    missing: 4,
    different: 0,
    extras: 2,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// At -vv, contents inside missing/extra dirs ARE listed
case!(nested_vv {
    orig: [
        Dir("sub1"),
        File("sub1/ok.txt", "ok\n"),
        File("sub1/missing.txt", "missing\n"),
        Dir("sub3"),
        Dir("sub3/deep"),
        File("sub3/deep/file.txt", "deep\n"),
    ],
    backup: [
        Dir("sub1"),
        File("sub1/ok.txt", "ok\n"),
        Dir("sub2"),
        File("sub2/extra.txt", "extra\n"),
    ],
    flags: ["-vv"],
    lines: [
        "MISSING-FILE: a/sub1/missing.txt",
        "MISSING-DIR: a/sub3",
        "MISSING-DIR: a/sub3/deep",
        "MISSING-FILE: a/sub3/deep/file.txt",
        "EXTRA-DIR: b/sub2",
        "EXTRA-FILE: b/sub2/extra.txt",
    ],
    original_processed: 7,
    backup_processed: 5,
    missing: 4,
    different: 0,
    extras: 2,
    special_files: 0,
    similarities: 3,
    skipped: 0,
    errors: 0,
});

// Special test: 1 MB files identical except for one byte near the end.
// Sampling is overwhelmingly likely to miss the difference, but --all catches it via BLAKE3.
// This test cannot use case! because it needs custom file generation (not static content).
#[test]
fn hash_catches_single_byte_difference() {
    use super::{cmd, no_line_has, some_line_has, stdout_of};

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
