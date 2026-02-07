//! Shared test infrastructure for programmatic test directory setup.
//!
//! Provides the `case!` macro and supporting types for declarative integration tests.
//! Each `case!` invocation generates a test that:
//!   1. Creates filesystem entries in temp orig/backup dirs
//!   2. Runs `vfy` and checks output lines + summary counts
//!   3. Automatically runs the reversed direction (orig↔backup swapped)
//!
//! Entry types: File, Dir, Sym (symlink), Fifo (named pipe),
//!              FileUnreadable, DirUnreadable, FileSized.
//!
//! Optional debug_contains/debug_excludes fields allow checking DEBUG output lines.

use std::path::Path;
use std::process::Command as StdCommand;

#[allow(unused_imports)]
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Entry DSL
// ---------------------------------------------------------------------------

/// Filesystem entry for test setup.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Variants used in future phases
pub enum Entry {
    /// Regular file with name and content: `File("name.txt", "content")`
    File(&'static str, &'static str),
    /// Empty directory: `Dir("dirname")`
    Dir(&'static str),
    /// Symlink: `Sym("link", "target")`
    Sym(&'static str, &'static str),
    /// Named pipe (FIFO): `Fifo("name")`
    Fifo(&'static str),
    /// File that becomes unreadable (chmod 0o000): `FileUnreadable("name", "content")`
    FileUnreadable(&'static str, &'static str),
    /// Directory that becomes unreadable (chmod 0o000): `DirUnreadable("name")`
    DirUnreadable(&'static str),
    /// File with N bytes of 'x': `FileSized("name", 1024)`
    FileSized(&'static str, usize),
}

#[allow(unused_imports)]
pub use Entry::*;

// ---------------------------------------------------------------------------
// Expected counts
// ---------------------------------------------------------------------------

/// Expected summary values from vfy output.
#[derive(Clone, Debug, Default)]
pub struct Counts {
    pub original_processed: u64,
    pub backup_processed: u64,
    pub missing: u64,
    pub different: u64,
    pub extras: u64,
    pub special_files: u64,
    pub similarities: u64,
    pub skipped: u64,
    pub errors: u64,
}

/// Debug line assertions for verbose output checking.
#[derive(Clone, Debug, Default)]
pub struct DebugChecks<'a> {
    /// Substrings that must appear in at least one DEBUG line
    pub contains: &'a [&'a str],
    /// Substrings that must NOT appear in any DEBUG line
    pub excludes: &'a [&'a str],
}

/// Output assertions for checking arbitrary substrings in full output.
#[derive(Clone, Debug, Default)]
pub struct OutputChecks<'a> {
    /// Substrings that must appear somewhere in the output
    pub contains: &'a [&'a str],
    /// Substrings that must NOT appear anywhere in the output
    pub excludes: &'a [&'a str],
}

// ---------------------------------------------------------------------------
// case! macro
// ---------------------------------------------------------------------------

/// Generate a test function that creates temp directories, runs vfy, and checks output.
///
/// Also runs the reversed test (orig↔backup swapped) automatically.
///
/// Basic form (no debug checks):
/// ```ignore
/// case!(test_name {
///     orig: [...],
///     backup: [...],
///     flags: [...],
///     lines: [...],
///     original_processed: N,
///     ...
/// });
/// ```
///
/// Extended form with debug checks:
/// ```ignore
/// case!(test_name {
///     orig: [...],
///     backup: [...],
///     flags: [...],
///     lines: [...],
///     debug_contains: ["Comparing"],      // DEBUG lines must contain these
///     debug_excludes: ["Comparing file"], // DEBUG lines must NOT contain these
///     original_processed: N,
///     ...
/// });
/// ```
#[macro_export]
macro_rules! case {
    // Full form with debug_contains, debug_excludes, output_contains, output_excludes, and symmetric
    ($name:ident {
        orig: [ $($orig:expr),* $(,)? ],
        backup: [ $($backup:expr),* $(,)? ],
        flags: [ $($flag:expr),* $(,)? ],
        lines: [ $($line:expr),* $(,)? ],
        debug_contains: [ $($dc:expr),* $(,)? ],
        debug_excludes: [ $($de:expr),* $(,)? ],
        output_contains: [ $($oc:expr),* $(,)? ],
        output_excludes: [ $($oe:expr),* $(,)? ],
        original_processed: $op:expr,
        backup_processed: $bp:expr,
        missing: $mis:expr,
        different: $diff:expr,
        extras: $ext:expr,
        special_files: $nfd:expr,
        similarities: $sim:expr,
        skipped: $skip:expr,
        errors: $err:expr,
        symmetric: $sym:expr $(,)?
    }) => {
        #[test]
        fn $name() {
            $crate::harness::check_full(
                stringify!($name),
                &[$($orig),*],
                &[$($backup),*],
                &[$($flag),*],
                &[$($line),*],
                $crate::harness::DebugChecks {
                    contains: &[$($dc),*],
                    excludes: &[$($de),*],
                },
                $crate::harness::OutputChecks {
                    contains: &[$($oc),*],
                    excludes: &[$($oe),*],
                },
                $crate::harness::Counts {
                    original_processed: $op,
                    backup_processed: $bp,
                    missing: $mis,
                    different: $diff,
                    extras: $ext,
                    special_files: $nfd,
                    similarities: $sim,
                    skipped: $skip,
                    errors: $err,
                },
                $sym,
            );
        }
    };
    // Full form with debug_contains, debug_excludes, output_contains, output_excludes (symmetric default true)
    ($name:ident {
        orig: [ $($orig:expr),* $(,)? ],
        backup: [ $($backup:expr),* $(,)? ],
        flags: [ $($flag:expr),* $(,)? ],
        lines: [ $($line:expr),* $(,)? ],
        debug_contains: [ $($dc:expr),* $(,)? ],
        debug_excludes: [ $($de:expr),* $(,)? ],
        output_contains: [ $($oc:expr),* $(,)? ],
        output_excludes: [ $($oe:expr),* $(,)? ],
        original_processed: $op:expr,
        backup_processed: $bp:expr,
        missing: $mis:expr,
        different: $diff:expr,
        extras: $ext:expr,
        special_files: $nfd:expr,
        similarities: $sim:expr,
        skipped: $skip:expr,
        errors: $err:expr $(,)?
    }) => {
        #[test]
        fn $name() {
            $crate::harness::check_full(
                stringify!($name),
                &[$($orig),*],
                &[$($backup),*],
                &[$($flag),*],
                &[$($line),*],
                $crate::harness::DebugChecks {
                    contains: &[$($dc),*],
                    excludes: &[$($de),*],
                },
                $crate::harness::OutputChecks {
                    contains: &[$($oc),*],
                    excludes: &[$($oe),*],
                },
                $crate::harness::Counts {
                    original_processed: $op,
                    backup_processed: $bp,
                    missing: $mis,
                    different: $diff,
                    extras: $ext,
                    special_files: $nfd,
                    similarities: $sim,
                    skipped: $skip,
                    errors: $err,
                },
                true,
            );
        }
    };
    // Extended form with debug_contains and debug_excludes (no output checks)
    ($name:ident {
        orig: [ $($orig:expr),* $(,)? ],
        backup: [ $($backup:expr),* $(,)? ],
        flags: [ $($flag:expr),* $(,)? ],
        lines: [ $($line:expr),* $(,)? ],
        debug_contains: [ $($dc:expr),* $(,)? ],
        debug_excludes: [ $($de:expr),* $(,)? ],
        original_processed: $op:expr,
        backup_processed: $bp:expr,
        missing: $mis:expr,
        different: $diff:expr,
        extras: $ext:expr,
        special_files: $nfd:expr,
        similarities: $sim:expr,
        skipped: $skip:expr,
        errors: $err:expr $(,)?
    }) => {
        #[test]
        fn $name() {
            $crate::harness::check_with_debug(
                stringify!($name),
                &[$($orig),*],
                &[$($backup),*],
                &[$($flag),*],
                &[$($line),*],
                $crate::harness::DebugChecks {
                    contains: &[$($dc),*],
                    excludes: &[$($de),*],
                },
                $crate::harness::Counts {
                    original_processed: $op,
                    backup_processed: $bp,
                    missing: $mis,
                    different: $diff,
                    extras: $ext,
                    special_files: $nfd,
                    similarities: $sim,
                    skipped: $skip,
                    errors: $err,
                },
                true,
            );
        }
    };
    // Basic form (no debug or output checks)
    ($name:ident {
        orig: [ $($orig:expr),* $(,)? ],
        backup: [ $($backup:expr),* $(,)? ],
        flags: [ $($flag:expr),* $(,)? ],
        lines: [ $($line:expr),* $(,)? ],
        original_processed: $op:expr,
        backup_processed: $bp:expr,
        missing: $mis:expr,
        different: $diff:expr,
        extras: $ext:expr,
        special_files: $nfd:expr,
        similarities: $sim:expr,
        skipped: $skip:expr,
        errors: $err:expr $(,)?
    }) => {
        #[test]
        fn $name() {
            $crate::harness::check(
                stringify!($name),
                &[$($orig),*],
                &[$($backup),*],
                &[$($flag),*],
                &[$($line),*],
                $crate::harness::Counts {
                    original_processed: $op,
                    backup_processed: $bp,
                    missing: $mis,
                    different: $diff,
                    extras: $ext,
                    special_files: $nfd,
                    similarities: $sim,
                    skipped: $skip,
                    errors: $err,
                },
                true,
            );
        }
    };
}


// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

/// Tracks paths that need permission restoration during cleanup.
pub(crate) struct PermissionGuard {
    paths: Vec<std::path::PathBuf>,
}

impl PermissionGuard {
    fn new() -> Self {
        Self { paths: Vec::new() }
    }

    fn add(&mut self, path: std::path::PathBuf) {
        self.paths.push(path);
    }
}

impl Drop for PermissionGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        for path in &self.paths {
            // Restore permissions so the directory can be cleaned up
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
        }
    }
}

/// Create filesystem entries in the given directory.
pub fn create_entries(dir: &Path, entries: &[Entry]) -> PermissionGuard {
    let mut guard = PermissionGuard::new();

    for entry in entries {
        match entry {
            Entry::File(name, content) => {
                let path = dir.join(name);
                if let Some(parent) = path.parent() {
                    if parent != dir {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                }
                std::fs::write(&path, content).unwrap();
            }
            Entry::Dir(name) => {
                std::fs::create_dir_all(dir.join(name)).unwrap();
            }
            Entry::Sym(name, target) => {
                std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
            }
            Entry::Fifo(name) => {
                let path = dir.join(name);
                StdCommand::new("mkfifo")
                    .arg(&path)
                    .status()
                    .expect("mkfifo failed");
            }
            Entry::FileUnreadable(name, content) => {
                let path = dir.join(name);
                if let Some(parent) = path.parent() {
                    if parent != dir {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                }
                std::fs::write(&path, content).unwrap();
                // Make unreadable
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
                guard.add(path);
            }
            Entry::DirUnreadable(name) => {
                let path = dir.join(name);
                std::fs::create_dir_all(&path).unwrap();
                // Make unreadable
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
                guard.add(path);
            }
            Entry::FileSized(name, size) => {
                let path = dir.join(name);
                if let Some(parent) = path.parent() {
                    if parent != dir {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                }
                let content = vec![b'x'; *size];
                std::fs::write(&path, content).unwrap();
            }
        }
    }

    guard
}

/// Transform an expected line for the reversed (orig↔backup swapped) test.
///
/// Rules:
/// - MISSING-X ↔ EXTRA-X  (prefix swap + path a/↔b/)
/// - DANGLING-SYMLINK:     (path a/↔b/ only)
/// - Everything else:      unchanged (DIFFERENT-*, SPECIAL-FILE, SYMLINK-SKIPPED
///   always report the orig path, which is always a/)
pub fn reverse_expected_line(line: &str) -> String {
    // Split "PREFIX: path" at the first ": "
    let (prefix, path) = match line.split_once(": ") {
        Some((p, r)) => (p, r),
        None => return line.to_string(),
    };

    let (new_prefix, swap_path) = match prefix {
        "MISSING-FILE" => ("EXTRA-FILE", true),
        "MISSING-DIR" => ("EXTRA-DIR", true),
        "MISSING-SYMLINK" => ("EXTRA-SYMLINK", true),
        "MISSING-SPECIAL" => ("EXTRA-SPECIAL", true),
        "MISSING-ERROR" => ("EXTRA-ERROR", true),
        "EXTRA-FILE" => ("MISSING-FILE", true),
        "EXTRA-DIR" => ("MISSING-DIR", true),
        "EXTRA-SYMLINK" => ("MISSING-SYMLINK", true),
        "EXTRA-SPECIAL" => ("MISSING-SPECIAL", true),
        "EXTRA-ERROR" => ("MISSING-ERROR", true),
        "DANGLING-SYMLINK" => ("DANGLING-SYMLINK", true),
        "SPECIAL-FILE" => ("SPECIAL-FILE", true),
        _ => (prefix, false),
    };

    let new_path = if swap_path {
        if let Some(rest) = path.strip_prefix("a/") {
            format!("b/{}", rest)
        } else if let Some(rest) = path.strip_prefix("b/") {
            format!("a/{}", rest)
        } else {
            panic!(
                "Expected line path must start with a/ or b/, got: {:?}",
                line
            );
        }
    } else {
        path.to_string()
    };

    format!("{}: {}", new_prefix, new_path)
}

/// Return true if this output line is a "diagnostic" line (not summary/CMD/DEBUG).
pub fn is_diagnostic_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // Summary block: "SUMMARY:" header and indented lines
    if line.starts_with("SUMMARY:") || line.starts_with("    ") {
        return false;
    }
    // CMD line at the top
    if line.starts_with("CMD:") {
        return false;
    }
    if line.starts_with("COMPARISON FINISHED!") {
        return false;
    }
    // DEBUG lines (verbose output)
    if line.starts_with("DEBUG:") {
        return false;
    }
    true
}

/// Run vfy and check output lines, summary counts, DEBUG lines, and output substrings.
pub fn run_and_check_full(
    label: &str,
    tmp: &Path,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    debug_checks: Option<&DebugChecks>,
    output_checks: Option<&OutputChecks>,
    counts: &Counts,
) {
    let a = tmp.join("a");
    let b = tmp.join("b");

    // Clean and recreate
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let _guard_a = create_entries(&a, orig_entries);
    let _guard_b = create_entries(&b, backup_entries);

    let a_str = a.to_str().unwrap();
    let b_str = b.to_str().unwrap();

    let mut args: Vec<&str> = vec![a_str, b_str];
    args.extend_from_slice(flags);

    let output = assert_cmd::cargo_bin_cmd!("vfy")
        .args(&args)
        .output()
        .expect("failed to run vfy");

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Collect diagnostic lines (non-DEBUG, non-summary)
    let mut diag_lines: Vec<&str> = stdout.lines().filter(|l| is_diagnostic_line(l)).collect();

    // Collect DEBUG lines for separate checking
    let debug_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("DEBUG:"))
        .collect();

    // Match expected lines (each must match exactly one diagnostic line).
    // Expected format: "PREFIX: relative/path" — checks that the output line
    // starts with PREFIX and contains the relative path.
    let mut unmatched_expected: Vec<&str> = Vec::new();
    for expected in expected_lines {
        let matches = |line: &&str| -> bool {
            if let Some((prefix, path)) = expected.split_once(": ") {
                line.starts_with(prefix) && line.contains(path)
            } else {
                line.contains(expected)
            }
        };
        if let Some(pos) = diag_lines.iter().position(matches) {
            diag_lines.remove(pos);
        } else {
            unmatched_expected.push(expected);
        }
    }

    if !unmatched_expected.is_empty() || !diag_lines.is_empty() {
        panic!(
            "[{}] Line mismatch.\n\
             Expected lines not found: {:?}\n\
             Unexpected lines in output: {:?}\n\
             Full output:\n{}",
            label, unmatched_expected, diag_lines, stdout
        );
    }

    // Check DEBUG line assertions if provided
    if let Some(checks) = debug_checks {
        // Check debug_contains: each pattern must appear in at least one DEBUG line
        for pattern in checks.contains {
            let found = debug_lines.iter().any(|line| line.contains(pattern));
            if !found {
                panic!(
                    "[{}] DEBUG check failed: expected DEBUG output to contain {:?}.\n\
                     DEBUG lines:\n{}\n\
                     Full output:\n{}",
                    label,
                    pattern,
                    debug_lines.join("\n"),
                    stdout
                );
            }
        }

        // Check debug_excludes: each pattern must NOT appear in any DEBUG line
        for pattern in checks.excludes {
            let found = debug_lines.iter().any(|line| line.contains(pattern));
            if found {
                panic!(
                    "[{}] DEBUG check failed: expected DEBUG output to NOT contain {:?}.\n\
                     DEBUG lines:\n{}\n\
                     Full output:\n{}",
                    label,
                    pattern,
                    debug_lines.join("\n"),
                    stdout
                );
            }
        }
    }

    // Check output substring assertions if provided
    if let Some(checks) = output_checks {
        // Check output_contains: each pattern must appear somewhere in output
        for pattern in checks.contains {
            if !stdout.contains(pattern) {
                panic!(
                    "[{}] Output check failed: expected output to contain {:?}.\n\
                     Full output:\n{}",
                    label, pattern, stdout
                );
            }
        }

        // Check output_excludes: each pattern must NOT appear anywhere in output
        for pattern in checks.excludes {
            if stdout.contains(pattern) {
                panic!(
                    "[{}] Output check failed: expected output to NOT contain {:?}.\n\
                     Full output:\n{}",
                    label, pattern, stdout
                );
            }
        }
    }

    // Check summary counts
    let expect_exit_0 = counts.missing == 0
        && counts.different == 0
        && counts.extras == 0
        && counts.special_files == 0
        && counts.errors == 0;

    let checks = [
        format!("Original items processed: {}", counts.original_processed),
        format!("Backup items processed: {}", counts.backup_processed),
        format!("Missing: {}", counts.missing),
        format!("Different: {}", counts.different),
        format!("Extras: {}", counts.extras),
        format!("Special files: {}", counts.special_files),
        format!("Similarities: {}", counts.similarities),
        format!("Skipped: {}", counts.skipped),
        format!("Errors: {}", counts.errors),
    ];

    for check_str in &checks {
        if !stdout.contains(check_str) {
            panic!(
                "[{}] Summary mismatch: expected {:?} in output.\nFull output:\n{}",
                label, check_str, stdout
            );
        }
    }

    // Check exit code
    let actual_exit = output.status.code().unwrap_or(-1);
    let expected_exit = if expect_exit_0 { 0 } else { 1 };
    if actual_exit != expected_exit {
        panic!(
            "[{}] Exit code mismatch: expected {}, got {}.\nFull output:\n{}",
            label, expected_exit, actual_exit, stdout
        );
    }
}

/// Main entry point for case! macro: runs forward and reversed tests.
pub fn check(
    name: &str,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    counts: Counts,
    symmetric: bool,
) {
    check_internal(name, orig_entries, backup_entries, flags, expected_lines, None, None, counts, symmetric);
}

/// Entry point for case! macro with DEBUG checking: runs forward and reversed tests.
pub fn check_with_debug(
    name: &str,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    debug_checks: DebugChecks,
    counts: Counts,
    symmetric: bool,
) {
    check_internal(
        name,
        orig_entries,
        backup_entries,
        flags,
        expected_lines,
        Some(debug_checks),
        None,
        counts,
        symmetric,
    );
}

/// Entry point for case! macro with all checking options: runs forward and reversed tests.
pub fn check_full(
    name: &str,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    debug_checks: DebugChecks,
    output_checks: OutputChecks,
    counts: Counts,
    symmetric: bool,
) {
    check_internal(
        name,
        orig_entries,
        backup_entries,
        flags,
        expected_lines,
        Some(debug_checks),
        Some(output_checks),
        counts,
        symmetric,
    );
}

/// Create test directories from Entry arrays for legacy manual tests.
/// Returns (TempDir, orig_path_string, backup_path_string).
/// The TempDir must be kept alive for the duration of the test.
pub fn setup_legacy_test_dirs(
    orig_entries: &[Entry],
    backup_entries: &[Entry],
) -> (tempfile::TempDir, String, String) {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    create_entries(&a, orig_entries);
    create_entries(&b, backup_entries);
    (
        tmp,
        a.to_str().unwrap().to_string(),
        b.to_str().unwrap().to_string(),
    )
}

fn check_internal(
    name: &str,
    orig_entries: &[Entry],
    backup_entries: &[Entry],
    flags: &[&str],
    expected_lines: &[&str],
    debug_checks: Option<DebugChecks>,
    output_checks: Option<OutputChecks>,
    counts: Counts,
    symmetric: bool,
) {
    let tmp = std::env::temp_dir().join(format!("bv_harness_{}", name));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Forward direction
    run_and_check_full(
        &format!("{} (forward)", name),
        &tmp,
        orig_entries,
        backup_entries,
        flags,
        expected_lines,
        debug_checks.as_ref(),
        output_checks.as_ref(),
        &counts,
    );

    // Reversed direction: swap orig↔backup, swap missing↔extras,
    // swap original_processed↔backup_processed, transform MISSING↔EXTRA in lines
    if symmetric {
        let reversed_lines: Vec<String> = expected_lines
            .iter()
            .map(|l| reverse_expected_line(l))
            .collect();
        let reversed_line_refs: Vec<&str> = reversed_lines.iter().map(|s| s.as_str()).collect();

        let reversed_counts = Counts {
            original_processed: counts.backup_processed,
            backup_processed: counts.original_processed,
            missing: counts.extras,
            different: counts.different,
            extras: counts.missing,
            special_files: counts.special_files,
            similarities: counts.similarities,
            skipped: counts.skipped,
            errors: counts.errors,
        };

        // Note: DEBUG and output checks are only done on forward direction, since the
        // patterns typically reference a/ and b/ paths which swap in reversed mode
        run_and_check_full(
            &format!("{} (reversed)", name),
            &tmp,
            backup_entries,
            orig_entries,
            flags,
            &reversed_line_refs,
            None, // Skip debug checks for reversed
            None, // Skip output checks for reversed
            &reversed_counts,
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}
