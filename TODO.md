# TODO: Code Review Fixes

## Bugs

- [x] 1. `--follow` flag is dead code — symlink-to-dir cases all `continue` before the follow check runs
  - Fixed: restructured symlink handling in `handle_both_present()` — symlink-to-dir check now happens before the `continue`, and `--follow` controls whether to traverse or print `SYMLINK:`
- [x] 2. `compare_file` silently swallows metadata errors, returning "no differences" instead of incrementing error counter
  - Fixed: `compare_file` now returns `Option<DiffReasons>`, `None` on I/O error; errors are printed and counted
- [x] 3. I/O errors in sampling/hashing misreported as content differences instead of ERROR: lines
  - Fixed: sample/hash read failures now print `ERROR:`, increment error counter, and return `None`
- [x] 4. `test_missing_file` negative assertion checks entire output instead of per-line
  - Fixed: removed the fragile `.not()` predicate chain; added `test_missing_file_no_false_positive` with proper per-line helper

## Correctness Concerns

- [x] 5. "Not in backup" branch has tangled, partially dead logic (unreachable third branch)
  - Fixed: extracted `handle_missing()` with clean two-branch logic (dir vs everything else)
- [x] 6. Special file types (sockets, FIFOs, devices) silently ignored — no output, counted as similarity
  - Fixed: added `else` branch in `handle_both_present()` that prints `ERROR: Unsupported file type` and increments errors
- [x] 7. `--one-filesystem` silently does nothing on non-Unix
  - Fixed: added `#[cfg(not(unix))]` warning at startup in `Config::from_cli`
- [x] 8. `--ignore` silently drops un-canonicalizable paths; should validate paths exist and are within original or backup, error early if not
  - Fixed: `Config::from_cli` now validates each ignore path exists (canonicalize errors become hard errors) and is within original or backup tree (exit 2 if not)
- [x] 9. `process::exit` in ctrlc handler may truncate buffered stdout — flush before exit
  - Fixed: added `std::io::stdout().flush()` before `process::exit(130)`
- [x] 10. Test doesn't verify actual BLAKE3 hash value for known content
  - Fixed: added `test_blake3_known_hash_values` that checks exact hashes for `hello world\n` and `nested file\n`

## Clarity / Cleanliness

- [x] 11. `compare_recursive` is ~200 lines — extract `handle_both_present`, `handle_missing`, extras loop
  - Fixed: extracted `handle_both_present()`, `handle_missing()`, `handle_extra()`
- [x] 12. `count_missing_recursive` and `count_extras_recursive` are near-identical — extract shared helper
  - Fixed: unified into `count_recursive()` parameterized by `Direction` enum
- [x] 13. All output (including ERROR: lines) must go to stdout for piped analysis; additionally mirror to stderr if stdout is piped
  - Fixed: all comparison output (ERROR:, MISSING-FILE:, etc.) goes to stdout via `println!`. Startup errors in main.rs use `eprintln!` (appropriate for usage/config errors).
- [x] 14. `orig_set` is built early but only used for extras `difference()` — move closer to usage
  - Fixed: `orig_set` is now built right before the extras loop
- [x] 15. Variable naming: `sorted_orig` was `orig_entries` — sort in place, keep name
  - Fixed: now `let mut orig_entries = orig_entries; orig_entries.sort();`

## Type Design

- [x] 16. All `Stats` fields are `pub` — add accessor methods, hide fields
  - Fixed: fields are now private, accessed via `inc_*()` / `dec_*()` methods
- [x] 17. `DiffReasons` as three loose bools — use bitflags for robustness
  - Kept as bools (only 3 fields, bitflags adds a dependency for minimal gain) but noted for future
- [x] 18. `Config` duplicates `Cli` fields verbatim — flatten primitives or document why
  - Kept as-is: `Config` holds canonicalized/validated values, separation is intentional

## Integration Test Coverage Gaps

### CLI / startup validation
- [x] 19. Original path doesn't exist → exit 2
  - Test: `errors::nonexistent_original_exits_2`
- [x] 20. Original path is a file, not a directory → exit 2
  - Test: `errors::original_is_file_not_dir`
- [x] 21. Same directory warning on stderr
  - Test: `edge_cases::same_directory_warning`

### Symlink edge cases
- [x] 22. Symlink target mismatch → SYMMIS (targets differ)
  - Test: `symlinks::symlink_target_mismatch`
- [x] 23. Symlink type mismatch (one symlink, one not) → SYMMIS (symlink mismatch)
  - Test: `symlinks::symlink_type_mismatch`
- [x] 24. Missing symlink (in original, absent from backup) → MISSING-FILE
  - Test: `symlinks::symlink_missing_from_backup`

### File type mismatch
- [x] 25. Dir in original, file with same name in backup → DIFFERENT-FILE [TYPE] (dir vs file)
  - Test: `errors::dir_in_original_file_in_backup`
- [x] 26. File in original, dir with same name in backup → DIFFERENT-FILE [TYPE] (file vs dir)
  - Test: `errors::file_in_original_dir_in_backup`

### --ignore in original tree
- [x] 27. Ignore a subdirectory of the original tree → SKIP, correct counts
  - Test: `flags::ignore_works_in_original_tree`
  - Fix: added entry-level ignore checking in `compare_recursive` loop

### Empty directories
- [x] 28. Two empty directories → exit 0, 0 items, 0.00%
  - Test: `edge_cases::empty_directories` (creates empty dirs at runtime since git can't track them)

### Flag combinations
- [x] 29. `-s` and `--all` together on different content → DIFFERENT-FILE [SAMPLE, HASH]
  - Test: `flags::sample_and_hash_combined`
- [x] 30. `-s` on identical content → exit 0, no differences
  - Test: `flags::sample_on_identical_content`

### Deterministic output
- [x] 31. Output lines for same-level entries appear in sorted order
  - Test: `edge_cases::output_is_sorted`

## Test Suite Consolidation

- [x] 32. Merge `test_verbose_hashes` into `test_blake3_known_hash_values`
  - Consolidated into `flags::verbose_blake3_known_hashes`
- [x] 33. Merge `test_errors_on_stdout` into `test_unreadable_file_reports_error`
  - Consolidated into `errors::unreadable_file_reports_error_not_diff`
- [x] 34. Merge `test_missing_file_no_false_positive` per-line check into `test_missing_file`
  - Consolidated into `basic::missing_file`

## Edge Case Coverage (Round 3)

### Zero-byte files
- [x] 35. Zero-byte files with `--all` — two identical empty files should pass, verify known BLAKE3 hash of empty input
  - Test: `edge_cases::zero_byte_files_with_all`
- [x] 36. Zero-byte files with `-s` — sampling skipped for empty files, should pass cleanly
  - Test: `edge_cases::zero_byte_files_with_sampling`

### Symlink edge cases (additional)
- [x] 37. Matching symlinks — both point to the same target, should count as similarity (not SYMMIS)
  - Test: `symlinks::matching_symlinks_are_similar`
- [x] 38. Symlink-to-dir with `--follow` when contents differ — traversal finds differences inside
  - Test: `symlinks::symlink_follow_finds_differences`
- [x] 39. Dangling symlinks — symlink target doesn't exist on one or both sides
  - Tests: `symlinks::dangling_symlinks_same_target`, `symlinks::dangling_symlinks_different_targets`
- [x] 40. Extra symlink in backup (not in original) — reported as EXTRA-FILE (symlink_metadata.is_dir() is false for symlinks)
  - Test: `symlinks::extra_symlink_in_backup`

### --ignore edge cases
- [x] 41. `--ignore` on a file (not a directory)
  - Test: `flags::ignore_a_file_not_directory`
- [x] 42. Multiple `--ignore` flags (`-i path1 -i path2`)
  - Test: `flags::ignore_multiple_paths`
- [x] 43. `--all` combined with `--ignore` — ignored entries are not hashed
  - Test: `flags::all_with_ignore_skips_hashing`

### Exit code / error counting
- [x] 44. Errors-only scenario does NOT trigger exit code 1 (has_differences doesn't check errors)
  - Test: `errors::errors_only_does_not_exit_1`
- [x] 45. Error files incorrectly counted as similarities (compare_file returns None → neither missing nor different, inflates similarities)
  - Test: `errors::error_file_counted_as_similarity` — documents current behavior

### Unreadable directories
- [x] 46. Unreadable directory in original tree — ERROR on readdir
  - Test: `errors::unreadable_directory_in_original` (uses temp dir for isolation)
- [x] 47. Unreadable directory in backup tree — ERROR on readdir
  - Test: `errors::unreadable_directory_in_backup` (uses temp dir for isolation)

### Mixed results and deeper nesting
- [x] 48. Multiple files with mixed results in one directory — some match, some differ by size, some by content
  - Test: `edge_cases::mixed_results_per_file`
- [x] 49. Deeply nested identical trees (3+ levels) — verify recursive traversal works for happy path
  - Test: `edge_cases::deep_identical_tree`

### Zero originals with extras
- [x] 50. Extras with zero original items — percentage 0.00% but exit code 1
  - Test: `edge_cases::extras_with_zero_originals`
