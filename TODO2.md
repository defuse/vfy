# TODO: Full Review Findings

## Bugs

- [x] B1. `--ignore` on extra FILES in backup tree is silently ignored
  - Fixed: removed `meta.is_dir()` gate on ignore check in `handle_extra`; ignore now applies to all extra entry types
  - Also fixed: `count_recursive` now checks ignore list per entry, so ignored subdirs inside missing/extra dirs are skipped
  - Tests: `flags::ignore_extra_file_in_backup`, `flags::ignore_subdir_inside_missing_dir`

- [x] B2. Type mismatch messages are misleading for special file types
  - Fixed: added `NOT_A_FILE_OR_DIR:` check before dir/file branching for both non-symlink and symlink `--follow` paths
  - Special files (FIFO, socket, device, etc.) are now detected early and reported as differences
  - Test: `edge_cases::special_file_via_symlink_follow`

- [x] B3. Sample size off-by-one vs Ruby — wontfix
  - Ruby (vfy.rb:135): `length = [aBytes, start + SampleSize].min - start + 1` reads up to SampleSize+1 = 33 bytes
  - Rust (compare.rs:409): `sample_size = 32`, reads exactly 32 bytes
  - Very low probability of impact; 32 bytes is a reasonable sample size

## Missing Test Coverage

- [ ] T1. Nonexistent backup directory -> exit 2
  - main.rs:30-33 handles this case
  - No test exists (only `nonexistent_original_exits_2` is tested)

- [ ] T2. Backup path is a file, not a directory -> exit 2
  - main.rs:30-33 handles this
  - No test (only `original_is_file_not_dir` is tested)

- [ ] T3. Cannot read symlink targets -> ERROR
  - compare.rs:200-203 handles this
  - No test — hard to trigger without a race condition

- [ ] T4. Both symlinks, one resolves to dir, other resolves to file
  - compare.rs falls to target comparison (line 168); targets will likely differ -> SYMMIS
  - No specific test; behavior is reasonable

- [x] T5. `--ignore` on extra file in backup tree (related to B1)
  - Test: `flags::ignore_extra_file_in_backup`

## Correctness Concerns

- [ ] C1. Root dir counted before ignore check
  - compare.rs:14-19 counts root as original+backup+similarity unconditionally, then compare_recursive checks ignore
  - If someone did `--ignore /orig` (the root itself), root would still be counted
  - Fine in practice because `--ignore` is validated to be within the tree, not the tree root itself
  - Ruby (vfy.rb:280): Same — `$itemCount += 1` before `compareDirs`

- [x] C2. If both dirs unreadable, only orig error reported — wontfix
  - compare_recursive:34-50: if orig dir read fails, returns immediately; backup dir read failure never checked
  - The item was already counted as similarity (from parent recurse)
  - Ruby (vfy.rb:245-251): only catches EACCES/EINVAL for original dir read — same behavior

- [x] C3. `--ignore` path canonicalization follows symlinks
  - Fixed: ignore path resolution now canonicalizes only the parent directory and appends the final component, preserving the symlink's own name
  - Existence verified via `symlink_metadata` (doesn't follow symlinks)
  - Tests: `flags::ignore_symlink_to_file`, `flags::ignore_symlink_to_dir_with_follow`

- [x] C4. Ignore paths not checked inside count_recursive
  - Fixed: `count_recursive` now checks ignore list per entry; skipped entries get `SKIP:` output and increment skipped counter
  - Test: `flags::ignore_subdir_inside_missing_dir`

- [x] C5. `handle_extra` stat failure: item not counted at all
  - Prints ERROR and increments error count — user sees the failure
  - Not counted as backup_items/extras, but error count is sufficient
  - No test: requires race condition (file deleted between readdir and stat), untestable without fault injection

## Design Observations

- [x] D1. `-f` short flag for `--follow` missing
  - Fixed: added `#[arg(short, long)]` to `follow` in cli.rs

- [x] D2. No `--machine` output format — wontfix
  - Ruby has `-m`/`--machine` for machine-readable summary (vfy.rb:43-45)
  - Not planned for Rust port

- [x] D3. `handle_extra` increment-then-undo pattern
  - Fixed: moved ignore check before counter increments in `handle_extra`; removed unused `dec_extras()` and `dec_backup_items()` from Stats

- [x] D4. `Ordering::Relaxed` is correct for all atomics
  - Single-threaded comparison; only ctrlc handler reads from another thread and only needs a best-effort snapshot
