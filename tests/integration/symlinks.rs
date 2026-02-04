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

// ── Sym→dir with different targets (BUG: targets never compared) ──

#[test]
fn symlink_dir_different_targets_no_follow() {
    // Both sides have a symlink that resolves to a directory, but they point
    // to different directories. The targets differ, so this must report
    // DIFFERENT-SYMLINK-TARGET.
    let tmp = std::env::temp_dir().join("bv_test_symdir_diff_target_nofollow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create two different target directories
    let dir1 = tmp.join("dir1");
    let dir2 = tmp.join("dir2");
    std::fs::create_dir_all(&dir1).unwrap();
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir1.join("file.txt"), "from dir1\n").unwrap();
    std::fs::write(dir2.join("file.txt"), "from dir2\n").unwrap();

    // a/link -> dir1, b/link -> dir2 (different targets, both resolve to dirs)
    std::os::unix::fs::symlink(&dir1, a.join("link")).unwrap();
    std::os::unix::fs::symlink(&dir2, b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Should report the targets differ
    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
        "Expected DIFFERENT-SYMLINK-TARGET for dir symlinks with different targets, got:\n{}",
        output
    );
    // Without --follow, should also emit SYMLINK: + skip (content not verified)
    assert!(
        some_line_has(&output, "SYMLINK:", "link"),
        "Expected SYMLINK: skip after target mismatch without --follow, got:\n{}",
        output
    );
}

#[test]
fn symlink_dir_different_targets_with_follow() {
    // Both sides have a symlink that resolves to a directory, but they point
    // to different directories. With --follow, this should report
    // DIFFERENT-SYMLINK-TARGET and then recursively compare the contents,
    // finding missing/extra/different files inside.
    let tmp = std::env::temp_dir().join("bv_test_symdir_diff_target_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create two different target directories with overlapping but distinct contents
    let dir1 = tmp.join("dir1");
    let dir2 = tmp.join("dir2");
    std::fs::create_dir_all(&dir1).unwrap();
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir1.join("shared.txt"), "same content\n").unwrap();
    std::fs::write(dir2.join("shared.txt"), "same content\n").unwrap();
    std::fs::write(dir1.join("only_orig.txt"), "only in dir1\n").unwrap();
    std::fs::write(dir2.join("only_backup.txt"), "only in dir2\n").unwrap();

    // a/link -> dir1, b/link -> dir2 (different targets, both resolve to dirs)
    std::os::unix::fs::symlink(&dir1, a.join("link")).unwrap();
    std::os::unix::fs::symlink(&dir2, b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Should report the targets differ
    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
        "Expected DIFFERENT-SYMLINK-TARGET for dir symlinks with different targets, got:\n{}",
        output
    );
    // Should also recursively compare and find differences inside
    assert!(
        some_line_has(&output, "MISSING-FILE:", "only_orig.txt"),
        "Expected MISSING-FILE for only_orig.txt inside followed symlink dir, got:\n{}",
        output
    );
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "only_backup.txt"),
        "Expected EXTRA-FILE for only_backup.txt inside followed symlink dir, got:\n{}",
        output
    );
    // shared.txt exists in both dirs with same content — should be a similarity
    // root(1) + link dir(1) + shared.txt(1) = 3 similarities
    assert!(
        output.contains("Similarities: 3"),
        "shared.txt inside followed symlink dir should count as similarity, got:\n{}",
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
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "target_diff"),
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
        some_line_has(&output, "DIFFERENT-SYMLINK-STATUS:", "type_mis"),
        "Expected DIFFERENT-SYMLINK-STATUS for type_mis (symlink vs regular), got:\n{}",
        output
    );
}

// ── DIFFERENT-SYMLINK-STATUS with directory on one side ──────

#[test]
fn symlink_status_mismatch_orig_dir() {
    // Original has a real directory with children, backup has a symlink.
    // Should report DIFFERENT-SYMLINK-STATUS and count dir contents as missing.
    let tmp = std::env::temp_dir().join("bv_test_symstatus_orig_dir");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/entry is a real directory with files inside
    std::fs::create_dir_all(a.join("entry")).unwrap();
    std::fs::write(a.join("entry/child.txt"), "content\n").unwrap();
    // b/entry is a symlink to some file
    std::fs::write(b.join("target.txt"), "target\n").unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("entry")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-STATUS:", "entry"),
        "Expected DIFFERENT-SYMLINK-STATUS for dir vs symlink, got:\n{}",
        output
    );
    // Directory contents should be counted as missing
    assert!(
        some_line_has(&output, "MISSING-FILE:", "child.txt"),
        "Expected MISSING-FILE for child.txt inside dir that was replaced by symlink, got:\n{}",
        output
    );
}

#[test]
fn symlink_status_mismatch_backup_dir() {
    // Original has a symlink, backup has a real directory with children.
    // Should report DIFFERENT-SYMLINK-STATUS and count dir contents as extra.
    let tmp = std::env::temp_dir().join("bv_test_symstatus_backup_dir");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/entry is a symlink to some file
    std::fs::write(a.join("target.txt"), "target\n").unwrap();
    std::os::unix::fs::symlink("target.txt", a.join("entry")).unwrap();
    // b/entry is a real directory with files inside
    std::fs::create_dir_all(b.join("entry")).unwrap();
    std::fs::write(b.join("entry/child.txt"), "content\n").unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "-v", "-v"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-STATUS:", "entry"),
        "Expected DIFFERENT-SYMLINK-STATUS for symlink vs dir, got:\n{}",
        output
    );
    // Directory contents should be counted as extra
    assert!(
        some_line_has(&output, "EXTRA-FILE:", "child.txt"),
        "Expected EXTRA-FILE for child.txt inside dir that replaced a symlink, got:\n{}",
        output
    );
}

#[test]
fn symlink_missing_from_backup() {
    let (a, b) = testdata("symlink_mismatch");
    let assert = cmd().args([&a, &b]).assert().code(1);
    let output = stdout_of(&assert);

    // missing_link: symlink in a, absent from b → MISSING-SYMLINK
    assert!(
        some_line_has(&output, "MISSING-SYMLINK:", "missing_link"),
        "Expected MISSING-SYMLINK for missing_link, got:\n{}",
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
        !output.contains("DIFFERENT-SYMLINK-TARGET:"),
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
        !some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "dangling"),
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
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "dangling"),
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

    // Extra symlink should be reported as EXTRA-SYMLINK
    assert!(
        some_line_has(&output, "EXTRA-SYMLINK:", "extra_link"),
        "Expected EXTRA-SYMLINK for extra symlink, got:\n{}",
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
fn file_symlink_without_follow_reports_skip() {
    // Both sides have matching file symlinks. Without --follow, the symlink
    // should be reported as SYMLINK: + skipped (content not verified),
    // not silently counted as a similarity.
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
    // Without --follow, link should be SYMLINK: + skipped, not a similarity
    assert!(
        some_line_has(&output, "SYMLINK:", "link"),
        "Expected SYMLINK: for file symlink without --follow, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "DIFFERENT-FILE", "/link"),
        "link should not be compared without --follow, got:\n{}",
        output
    );
}

// ── Dangling symlinks with --follow ──────────────────────────

#[test]
fn dangling_symlinks_same_target_with_follow() {
    // Both sides have dangling symlinks to the same nonexistent target.
    // With --follow, the targets can't be resolved. Should report
    // DANGLING-SYMLINK: to indicate we couldn't follow, not NOT_A_FILE_OR_DIR.
    let tmp = std::env::temp_dir().join("bv_test_dangling_follow");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Both sides point to the same nonexistent target
    std::os::unix::fs::symlink("nonexistent_target", a.join("dangling")).unwrap();
    std::os::unix::fs::symlink("nonexistent_target", b.join("dangling")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "--follow"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Should NOT be NOT_A_FILE_OR_DIR — these are dangling symlinks, not special files
    assert!(
        !some_line_has(&output, "NOT_A_FILE_OR_DIR:", "dangling"),
        "Dangling symlinks should not be NOT_A_FILE_OR_DIR, got:\n{}",
        output
    );
    // Should report DANGLING-SYMLINK: to indicate --follow couldn't resolve the target
    assert!(
        some_line_has(&output, "DANGLING-SYMLINK:", "dangling"),
        "Expected DANGLING-SYMLINK: for unresolvable symlinks with --follow, got:\n{}",
        output
    );
}

#[test]
fn dangling_orig_resolving_backup_file_with_follow() {
    // Orig symlink is dangling, backup symlink resolves to a file.
    // With --follow, the backup content is extra since orig can't provide it.
    let tmp = std::env::temp_dir().join("bv_test_dangling_orig_file_backup");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/link points to nonexistent target (dangling)
    std::os::unix::fs::symlink("nonexistent", a.join("link")).unwrap();
    // b/link points to a real file
    std::fs::write(b.join("target.txt"), "content\n").unwrap();
    std::os::unix::fs::symlink("target.txt", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str, "--follow"]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Targets differ → DIFFERENT-SYMLINK-TARGET
    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
        "Expected DIFFERENT-SYMLINK-TARGET, got:\n{}",
        output
    );
    // Orig is dangling → DANGLING-SYMLINK
    assert!(
        some_line_has(&output, "DANGLING-SYMLINK:", "link"),
        "Expected DANGLING-SYMLINK: for dangling orig, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "NOT_A_FILE_OR_DIR:", "link"),
        "Should not be NOT_A_FILE_OR_DIR, got:\n{}",
        output
    );
}

#[test]
fn dangling_backup_resolving_orig_dir_with_follow() {
    // Orig symlink resolves to a directory with children, backup symlink is
    // dangling. With --follow, the orig dir contents are missing from backup.
    let tmp = std::env::temp_dir().join("bv_test_dangling_backup_dir_orig");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // a/link points to a real directory with files
    let target_dir = tmp.join("real_dir");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("child.txt"), "content\n").unwrap();
    std::os::unix::fs::symlink(&target_dir, a.join("link")).unwrap();
    // b/link points to nonexistent target (dangling)
    std::os::unix::fs::symlink("nonexistent", b.join("link")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd()
        .args([&a_str, &b_str, "--follow", "-v", "-v"])
        .assert()
        .code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Targets differ → DIFFERENT-SYMLINK-TARGET
    assert!(
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
        "Expected DIFFERENT-SYMLINK-TARGET, got:\n{}",
        output
    );
    // Backup is dangling → DANGLING-SYMLINK
    assert!(
        some_line_has(&output, "DANGLING-SYMLINK:", "link"),
        "Expected DANGLING-SYMLINK: for dangling backup, got:\n{}",
        output
    );
    // Orig dir contents should be counted as missing
    assert!(
        some_line_has(&output, "MISSING-FILE:", "child.txt"),
        "Expected MISSING-FILE for child.txt inside orig dir with dangling backup, got:\n{}",
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
        some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "entry"),
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
        !some_line_has(&output, "DIFFERENT-SYMLINK-TARGET:", "link"),
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

// ── Symlink-to-directory missing from backup ────────────────

#[test]
fn missing_symlink_to_dir() {
    // A symlink that points to a directory should be reported as MISSING-SYMLINK,
    // not MISSING-DIR or MISSING-FILE.
    let tmp = std::env::temp_dir().join("bv_test_missing_symlink_to_dir");
    let _ = std::fs::remove_dir_all(&tmp);
    let a = tmp.join("a");
    let b = tmp.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    // Create a real directory target
    let target_dir = tmp.join("real_dir");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("inside.txt"), "content\n").unwrap();

    // a has a symlink to that directory; b does not
    std::os::unix::fs::symlink(&target_dir, a.join("link_to_dir")).unwrap();

    let a_str = a.to_str().unwrap().to_string();
    let b_str = b.to_str().unwrap().to_string();
    let assert = cmd().args([&a_str, &b_str]).assert().code(1);
    let output = stdout_of(&assert);

    let _ = std::fs::remove_dir_all(&tmp);

    // Should be MISSING-SYMLINK, not MISSING-DIR or MISSING-FILE
    assert!(
        some_line_has(&output, "MISSING-SYMLINK:", "link_to_dir"),
        "Expected MISSING-SYMLINK for symlink-to-directory, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "MISSING-DIR:", "link_to_dir"),
        "Symlink-to-dir should NOT be MISSING-DIR, got:\n{}",
        output
    );
    assert!(
        !some_line_has(&output, "MISSING-FILE:", "link_to_dir"),
        "Symlink-to-dir should NOT be MISSING-FILE, got:\n{}",
        output
    );
    assert!(output.contains("Errors: 0"), "got:\n{}", output);
}
