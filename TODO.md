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
- [ ] 19. Original path doesn't exist → exit 2
- [ ] 20. Original path is a file, not a directory → exit 2
- [ ] 21. Same directory warning on stderr

### Symlink edge cases
- [ ] 22. Symlink target mismatch → SYMMIS (targets differ)
- [ ] 23. Symlink type mismatch (one symlink, one not) → SYMMIS (symlink mismatch)
- [ ] 24. Missing symlink (in original, absent from backup) → MISSING-FILE

### File type mismatch
- [ ] 25. Dir in original, file with same name in backup → DIFFERENT-FILE [TYPE] (dir vs file)
- [ ] 26. File in original, dir with same name in backup → DIFFERENT-FILE [TYPE] (file vs dir)

### --ignore in original tree
- [ ] 27. Ignore a subdirectory of the original tree → SKIP, correct counts

### Empty directories
- [ ] 28. Two empty directories → exit 0, 0 items, 0.00%

### Flag combinations
- [ ] 29. `-s` and `--all` together on different content → DIFFERENT-FILE [SAMPLE, HASH]
- [ ] 30. `-s` on identical content → exit 0, no differences

### Deterministic output
- [ ] 31. Output lines for same-level entries appear in sorted order

## Test Suite Consolidation

- [ ] 32. Merge `test_verbose_hashes` into `test_blake3_known_hash_values`
- [ ] 33. Merge `test_errors_on_stdout` into `test_unreadable_file_reports_error`
- [ ] 34. Merge `test_missing_file_no_false_positive` per-line check into `test_missing_file`
