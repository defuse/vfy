use super::{cmd, some_line_has, stdout_of, testdata};

#[test]
fn symlink_dir_no_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd().args([&a, &b, "-v", "-v"]).assert().success();
    let output = stdout_of(&assert);

    // Without --follow, symlink-to-dir → SYMLINK: and not traversed
    assert!(
        some_line_has(&output, "SYMLINK:", "/link_dir"),
        "Expected SYMLINK: for link_dir without --follow, got:\n{}",
        output
    );
}

#[test]
fn symlink_dir_with_follow() {
    let (a, b) = testdata("symlink_dirs");
    let assert = cmd()
        .args([&a, &b, "--follow", "-v", "-v"])
        .assert()
        .success();
    let output = stdout_of(&assert);

    assert!(
        !output.contains("SYMLINK:"),
        "Should not produce SYMLINK: with --follow"
    );
    // Files inside the symlinked dir should be traversed
    assert!(
        some_line_has(&output, "DEBUG: Comparing file", "/link_dir/"),
        "Expected traversal into link_dir with --follow, got:\n{}",
        output
    );
}

#[test]
fn symlink_target_mismatch() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // target_diff: both symlinks but point to different files
    assert!(
        some_line_has(&output, "SYMMIS:", "target_diff"),
        "Expected SYMMIS for target_diff, got:\n{}",
        output
    );
    assert!(
        output.contains("targets differ"),
        "Expected 'targets differ' message"
    );
}

#[test]
fn symlink_type_mismatch() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // type_mis: symlink in a, regular file in b
    assert!(
        some_line_has(&output, "SYMMIS:", "type_mis"),
        "Expected SYMMIS for type_mis (symlink vs regular), got:\n{}",
        output
    );
}

#[test]
fn symlink_missing_from_backup() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // missing_link: symlink in a, absent from b → MISSING-FILE
    assert!(
        some_line_has(&output, "MISSING-FILE:", "missing_link"),
        "Expected MISSING-FILE for missing_link, got:\n{}",
        output
    );
}

#[test]
fn symlink_mismatch_summary() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // a/ has: missing_link, ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 6 items
    // In backup: ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 5 backup items
    // Missing: missing_link = 1
    // Different: target_diff (SYMMIS) + type_mis (SYMMIS) = 2
    // Similarities: ok.txt + real_a.txt + real_b.txt = 3
    assert!(output.contains("Original items processed: 6"),
        "got:\n{}", output);
    assert!(output.contains("Backup items processed: 5"),
        "got:\n{}", output);
    assert!(output.contains("Similarities: 3"),
        "got:\n{}", output);

    // Verify no ERROR lines — these are all expected conditions
    assert!(
        !output.contains("ERROR:"),
        "Should have no errors, got:\n{}",
        output
    );
}
