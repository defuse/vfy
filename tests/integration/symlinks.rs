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
    // Symlink-to-dir should be counted as skipped
    assert!(
        output.contains("Skipped: 1"),
        "Expected Skipped: 1 for symlink-to-dir without --follow, got:\n{}",
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

    // a/ has: root + missing_link, ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 7 items
    // In backup: root + ok.txt, real_a.txt, real_b.txt, target_diff, type_mis = 6 backup items
    // Missing: missing_link = 1
    // Different: target_diff (SYMMIS) + type_mis (SYMMIS) = 2
    // Similarities: root + ok.txt + real_a.txt + real_b.txt = 4
    assert!(output.contains("Original items processed: 7"),
        "got:\n{}", output);
    assert!(output.contains("Backup items processed: 6"),
        "got:\n{}", output);
    assert!(output.contains("Similarities: 4"),
        "got:\n{}", output);

    // Verify no ERROR lines — these are all expected conditions
    assert!(
        !output.contains("ERROR:"),
        "Should have no errors, got:\n{}",
        output
    );
}

// ── Matching symlinks ───────────────────────────────────────

#[test]
fn matching_symlinks_are_similar() {
    // Both sides have a symlink pointing to the same target → should count as similarity
    let (a, b) = testdata("symlink_matching");
    let assert = cmd().args([&a, &b]).assert().success();
    let output = stdout_of(&assert);

    assert!(
        !output.contains("SYMMIS:"),
        "Matching symlinks should not produce SYMMIS, got:\n{}",
        output
    );
    assert!(
        !output.contains("DIFFERENT-FILE"),
        "Matching symlinks should not produce DIFFERENT-FILE, got:\n{}",
        output
    );
    // 3 items: root + real.txt + link
    assert!(
        output.contains("Original items processed: 3"),
        "got:\n{}",
        output
    );
    assert!(
        output.contains("Similarities: 3"),
        "got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

// ── --follow with differences inside symlinked dir ──────────

#[test]
fn symlink_follow_finds_differences() {
    let (a, b) = testdata("symlink_follow_diff");
    let assert = cmd()
        .args([&a, &b, "--follow", "-v", "-v"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    // Traversal should find missing file inside the symlinked dir
    assert!(
        some_line_has(&output, "MISSING-FILE:", "only_orig.txt"),
        "Expected MISSING-FILE for only_orig.txt inside followed symlink dir, got:\n{}",
        output
    );
    // No SYMLINK: lines since we used --follow
    assert!(
        !output.contains("SYMLINK:"),
        "Should not produce SYMLINK: with --follow, got:\n{}",
        output
    );
}

// ── Dangling symlinks ───────────────────────────────────────

#[test]
fn dangling_symlinks_same_target() {
    // Both sides have dangling symlinks to the same nonexistent target
    let (a, b) = testdata("dangling_symlink");
    let assert = cmd().args([&a, &b]).assert();
    let output = stdout_of(&assert);

    // Both dangling symlinks point to same target → no SYMMIS
    assert!(
        !some_line_has(&output, "SYMMIS:", "dangling"),
        "Dangling symlinks with same target should not SYMMIS, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

#[test]
fn dangling_symlinks_different_targets() {
    // Dangling symlinks point to different targets
    let (a, b) = testdata("dangling_symlink_diff");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    assert!(
        some_line_has(&output, "SYMMIS:", "dangling"),
        "Dangling symlinks with different targets should SYMMIS, got:\n{}",
        output
    );
    assert!(
        output.contains("targets differ"),
        "Expected 'targets differ' message, got:\n{}",
        output
    );
}

// ── Extra symlink in backup ─────────────────────────────────

#[test]
fn extra_symlink_in_backup() {
    let (a, b) = testdata("extra_symlink");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // Extra symlink should be reported as EXTRA-FILE (symlink_metadata.is_dir() = false)
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "extra_link"),
        "Expected EXTRA-FILE for extra symlink, got:\n{}",
        output
    );
    assert!(output.contains("Extras: 1"), "got:\n{}", output);
}

// ── File symlinks with --follow ─────────────────────────────

#[test]
fn file_symlink_with_follow_compares_content() {
    // Both sides have link -> target.txt (same symlink target),
    // but the actual file content at target.txt differs.
    // With --follow, the symlink should compare resolved content too.
    let tmp = std::env::temp_dir().join("bv_test_file_symlink_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("target.txt"), "original content\n").unwrap();
    std::fs::write(b.join("target.txt"), "different backup content\n").unwrap();

    std::os::unix::fs::symlink("target.txt", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "--follow"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // target.txt is caught as different (regular file comparison)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "target.txt"),
        "Expected DIFFERENT-FILE for target.txt, got:\n{}",
        output
    );
    // With --follow, link should ALSO be caught as different (resolved content differs)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "/link"),
        "Expected DIFFERENT-FILE for link with --follow, got:\n{}",
        output
    );
}

#[test]
fn file_symlink_without_follow_checks_target_only() {
    // Same setup but WITHOUT --follow.
    // link should be similarity (targets match), only target.txt different.
    let tmp = std::env::temp_dir().join("bv_test_file_symlink_no_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    std::fs::write(a.join("target.txt"), "original content\n").unwrap();
    std::fs::write(b.join("target.txt"), "different backup content\n").unwrap();

    std::os::unix::fs::symlink("target.txt", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // target.txt is caught as different (regular file)
    assert!(
        some_line_has(&output, "DIFFERENT-FILE", "target.txt"),
        "Expected DIFFERENT-FILE for target.txt, got:\n{}",
        output
    );
    // Without --follow, link should NOT be different (targets match → similarity)
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "/link"),
        "link should be similarity without --follow (targets match), got:\n{}",
        output
    );
}

// ── Symlink type resolution mismatch ────────────────────────

#[test]
fn symlinks_one_resolves_to_dir_other_to_file() {
    // T4: Both sides are symlinks, but one resolves to a directory and the
    // other to a file. They must have different targets → SYMMIS.
    let tmp = std::env::temp_dir().join("bv_test_sym_dir_vs_file");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create targets: a directory and a file
    let dir_target = tmp.join("target_dir");
    std::fs::create_dir_all(&dir_target).unwrap();
    std::fs::write(tmp.join("target_file.txt"), "content\n").unwrap();

    // a/entry -> directory, b/entry -> file
    std::os::unix::fs::symlink(&dir_target, a.join("entry")).unwrap();
    std::os::unix::fs::symlink(tmp.join("target_file.txt"), b.join("entry")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Different targets → SYMMIS
    assert!(
        some_line_has(&output, "SYMMIS:", "entry"),
        "Expected SYMMIS for symlinks with dir vs file targets, got:\n{}",
        output
    );
    assert!(
        output.contains("targets differ"),
        "Expected 'targets differ' message, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

#[test]
fn symlink_same_target_dir_vs_file_no_follow() {
    // T4: Both symlinks have the same relative target (../foo), but on the
    // original side it resolves to a directory and on the backup side to a file.
    // Without --follow, targets match → counted as similarity.
    let tmp = std::env::temp_dir().join("bv_test_sym_same_target_no_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/../foo is a directory with a file inside
    std::fs::create_dir_all(tmp.join("a_foo")).unwrap();
    std::fs::write(tmp.join("a_foo").join("inside.txt"), "content\n").unwrap();
    // b/../foo is a regular file
    std::fs::write(tmp.join("b_foo"), "I'm a file\n").unwrap();

    // Both symlinks have the same target name but resolve differently
    std::os::unix::fs::symlink("../a_foo", a.join("link")).unwrap();
    std::os::unix::fs::symlink("../b_foo", b.join("link")).unwrap();

    // Wait — these have different targets. We need the SAME target string.
    // Use a shared name: both point to "../foo" but a/../foo is a dir, b/../foo is a file.
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create a/../foo as a directory
    std::fs::create_dir_all(tmp.join("foo_dir")).unwrap();
    std::fs::write(tmp.join("foo_dir").join("inside.txt"), "content\n").unwrap();
    // We can't have ../foo be both a dir and a file at the same path.
    // Instead, use separate parent-level targets with the same basename via symlinks.
    // Actually: a/link -> target, b/link -> target
    // a/target is a dir, b/target is a file
    std::fs::create_dir_all(a.join("target")).unwrap();
    std::fs::write(a.join("target").join("inside.txt"), "content\n").unwrap();
    std::fs::write(b.join("target"), "I'm a file\n").unwrap();

    std::os::unix::fs::symlink("target", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert();
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Without --follow, symlink targets match ("target") → similarity for the symlink.
    // But "target" itself exists as a non-symlink entry and will be compared directly
    // (dir vs file type mismatch).
    assert!(
        !some_line_has(&output, "SYMMIS:", "link"),
        "Symlinks with same target should not SYMMIS without --follow, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

#[test]
fn symlink_same_target_orig_dir_backup_file_follow() {
    // T4 with --follow: both symlinks point to "target". On original side,
    // target is a directory (with files inside). On backup side, target is a file.
    // With --follow, the resolved type mismatch should be detected.
    let tmp = std::env::temp_dir().join("bv_test_sym_dir_file_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/target is a directory with a file inside
    std::fs::create_dir_all(a.join("target")).unwrap();
    std::fs::write(a.join("target").join("inside.txt"), "content\n").unwrap();
    // b/target is a regular file
    std::fs::write(b.join("target"), "I'm a file\n").unwrap();

    std::os::unix::fs::symlink("target", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "-v", "-v"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The symlink resolves to dir on orig, file on backup → TYPE mismatch
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "link"),
        "Expected DIFFERENT-FILE [TYPE] for symlink resolving to dir vs file, got:\n{}",
        output
    );
    // inside.txt should be counted as missing (it's in the original dir but backup is a file)
    assert!(
        some_line_has(&output, "MISSING-FILE:", "inside.txt"),
        "Expected MISSING-FILE for inside.txt in type-mismatched symlink dir, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}

#[test]
fn symlink_same_target_orig_file_backup_dir_follow() {
    // T4 with --follow: both symlinks point to "target". On original side,
    // target is a file. On backup side, target is a directory (with files inside).
    let tmp = std::env::temp_dir().join("bv_test_sym_file_dir_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/target is a regular file
    std::fs::write(a.join("target"), "I'm a file\n").unwrap();
    // b/target is a directory with a file inside
    std::fs::create_dir_all(b.join("target")).unwrap();
    std::fs::write(b.join("target").join("inside.txt"), "content\n").unwrap();

    std::os::unix::fs::symlink("target", a.join("link")).unwrap();
    std::os::unix::fs::symlink("target", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "-v", "-v"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // The symlink resolves to file on orig, dir on backup → TYPE mismatch
    assert!(
        some_line_has(&output, "DIFFERENT-FILE [TYPE]:", "link"),
        "Expected DIFFERENT-FILE [TYPE] for symlink resolving to file vs dir, got:\n{}",
        output
    );
    // inside.txt should be counted as extra (it's in the backup dir but original is a file)
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "inside.txt"),
        "Expected EXTRA-FILE for inside.txt in type-mismatched symlink backup dir, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}
