# Type Comparison Matrix (Post-Refactor)

Comprehensive analysis of all possible entry types in original vs backup,
what the code does for each case, and whether we have test coverage.

All code behavior traced against `src/compare.rs` after the symlink-handling
refactor. All behavior verified correct against `docs/symlink-handling.md`.

## Entry types

- **File** — regular file
- **Dir** — real directory
- **Sym→file** — symlink whose target resolves to a regular file
- **Sym→dir** — symlink whose target resolves to a directory
- **Sym→dangling** — symlink whose target doesn't exist
- **Special** — device, FIFO, socket (not a symlink)
- **Absent** — entry doesn't exist on that side

---

## Row: Orig = File

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `compare_entries` → `compare_file` → similarity or `DIFFERENT-FILE [SIZE/SAMPLE/HASH]:` | `basic::identical`, `basic::different_size`, `basic::different_content_hash`, `basic::different_content_sample` | Yes |
| **Dir** | `compare_entries` → `DIFFERENT-FILE [TYPE]: (file vs dir)` + `count_recursive(backup, Extra)` | `errors::file_in_original_dir_in_backup` | Yes |
| **Sym→file** | `DIFFERENT-SYMLINK-STATUS:` + different. Orig is file (not dir) → no recursive counting. | No (reverse tested in `symlinks::symlink_type_mismatch`) | Yes |
| **Sym→dir** | `DIFFERENT-SYMLINK-STATUS:` + different. Orig is file (not dir) → no recursive counting. | No | Yes |
| **Sym→dangling** | `DIFFERENT-SYMLINK-STATUS:` + different. Orig is file (not dir) → no recursive counting. | No | Yes |
| **Special** | `is_special(backup)` → `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Absent** | `MISSING-FILE:` | `basic::missing_file` | Yes |

## Row: Orig = Dir

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `compare_entries` → `DIFFERENT-FILE [TYPE]: (dir vs file)` + `count_recursive(orig, Missing)` | `errors::dir_in_original_file_in_backup`, `errors::type_mismatch_dir_orig_counts_missing_contents` | Yes |
| **Dir** | `compare_entries` → `compare_recursive` | `basic::identical`, `basic::nested`, `edge_cases::deep_identical_tree` | Yes |
| **Sym→file** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(orig, Missing)` (orig is dir) | `symlinks::symlink_status_mismatch_orig_dir` | Yes |
| **Sym→dir** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(orig, Missing)` (orig is dir) | No (same code path as Sym→file above) | Yes |
| **Sym→dangling** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(orig, Missing)` (orig is dir) | No (same code path) | Yes |
| **Special** | `is_special(backup)` → `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Absent** | `MISSING-DIR:` + `count_recursive(orig, Missing)` | `basic::nested` | Yes |

## Row: Orig = Sym→file

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-STATUS:` + different. Neither side is a real dir → no recursive counting. | `symlinks::symlink_type_mismatch` | Yes |
| **Dir** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(backup, Extra)` (backup is dir) | `symlinks::symlink_status_mismatch_backup_dir` | Yes |
| **Sym→file** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `compare_file` on resolved content. | `symlinks::matching_symlinks_are_similar`, `symlinks::file_symlink_with_follow_compares_content`, `symlinks::file_symlink_without_follow_reports_skip` | Yes |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `compare_file` on resolved content. | `symlinks::symlink_target_mismatch` | Yes |
| **Sym→dir** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `compare_entries` → `DIFFERENT-FILE [TYPE]: (file vs dir)` + `count_recursive(backup, Extra)`. | `symlinks::symlink_same_target_orig_file_backup_dir_follow` | Yes |
| **Sym→dir** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `compare_entries` → `DIFFERENT-FILE [TYPE]:` + `count_recursive(backup, Extra)`. | `symlinks::symlinks_one_resolves_to_dir_other_to_file` | Yes (note 1) |
| **Sym→dangling** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `DANGLING-SYMLINK:` (backup) + different. | No | Yes |
| **Sym→dangling** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `DANGLING-SYMLINK:` (backup). | No | Yes |
| **Special** | `is_special(backup)` → `NOT_A_FILE_OR_DIR:` | `edge_cases::symlink_vs_special_file` | Yes |
| **Absent** | `MISSING-SYMLINK:` | `symlinks::symlink_missing_from_backup` | Yes |

## Row: Orig = Sym→dir

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-STATUS:` + different. Neither side is a real dir → no recursive counting. | No (same code path as Sym→file vs File) | Yes |
| **Dir** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(backup, Extra)` (backup is dir) | No (same code path as Sym→file vs Dir) | Yes |
| **Sym→file** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `compare_entries` → `DIFFERENT-FILE [TYPE]: (dir vs file)` + `count_recursive(orig, Missing)`. | `symlinks::symlink_same_target_orig_dir_backup_file_follow`, `symlinks::symlink_same_target_dir_vs_file_no_follow` | Yes |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `compare_entries` → `DIFFERENT-FILE [TYPE]:` + `count_recursive(orig, Missing)`. | `symlinks::symlinks_one_resolves_to_dir_other_to_file` | Yes (note 1) |
| **Sym→dir** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `compare_recursive` on resolved dirs. | `symlinks::symlink_dir_no_follow`, `symlinks::symlink_dir_with_follow` | Yes |
| **Sym→dir** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `compare_recursive` on resolved dirs. | `symlinks::symlink_dir_different_targets_no_follow`, `symlinks::symlink_dir_different_targets_with_follow` | Yes |
| **Sym→dangling** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `DANGLING-SYMLINK:` (backup) + `count_recursive(orig, Missing)`. | `symlinks::dangling_backup_resolving_orig_dir_with_follow` | Yes |
| **Sym→dangling** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `DANGLING-SYMLINK:` (backup) + different + `count_recursive(orig, Missing)`. | No | Yes |
| **Special** | `is_special(backup)` → `NOT_A_FILE_OR_DIR:` | No (same code path as Sym→file vs Special) | Yes |
| **Absent** | `MISSING-SYMLINK:` | `symlinks::missing_symlink_to_dir` | Yes |

## Row: Orig = Sym→dangling

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `DIFFERENT-SYMLINK-STATUS:` + different. Neither side is a real dir → no recursive counting. | No (same code path as Sym→file vs File) | Yes |
| **Dir** | `DIFFERENT-SYMLINK-STATUS:` + different + `count_recursive(backup, Extra)` (backup is dir) | No (same code path as Sym→file vs Dir) | Yes |
| **Sym→file** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `DANGLING-SYMLINK:` (orig). Backup is file, not dir → no recursive counting. | `symlinks::dangling_orig_resolving_backup_file_with_follow` | Yes |
| **Sym→file** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `DANGLING-SYMLINK:` (orig) + different. Backup is file, not dir → no recursive counting. | No | Yes |
| **Sym→dir** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `DANGLING-SYMLINK:` (orig) + `count_recursive(backup, Extra)`. | No | Yes |
| **Sym→dir** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `DANGLING-SYMLINK:` (orig) + different + `count_recursive(backup, Extra)`. | No | Yes |
| **Sym→dangling** (same target) | Without `--follow`: `SYMLINK:` + similarity + skip. With `--follow`: `DANGLING-SYMLINK:` x2 + different. | `symlinks::dangling_symlinks_same_target`, `symlinks::dangling_symlinks_same_target_with_follow` | Yes (note 2) |
| **Sym→dangling** (diff target) | `DIFFERENT-SYMLINK-TARGET:` + different. Without `--follow`: `SYMLINK:` + skip. With `--follow`: `DANGLING-SYMLINK:` x2 (no extra different since targets already differed). | `symlinks::dangling_symlinks_different_targets` (no-follow only) | Yes |
| **Special** | `is_special(backup)` → `NOT_A_FILE_OR_DIR:` | No (same code path as Sym→file vs Special) | Yes |
| **Absent** | `MISSING-SYMLINK:` | No (same code path as other missing symlink tests) | Yes |

## Row: Orig = Special

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Dir** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | No | Yes |
| **Sym→file** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | `edge_cases::special_file_vs_symlink` | Yes |
| **Sym→dir** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | No (same code path) | Yes |
| **Sym→dangling** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | No (same code path) | Yes |
| **Special** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | `edge_cases::symlink_to_dev_dir_with_follow` (via /dev) | Yes |
| **Absent** | `is_special(orig)` → `NOT_A_FILE_OR_DIR:` | `edge_cases::special_file_missing_from_backup` | Yes |

## Row: Orig = Absent

| Backup | Code behavior | Test? | Correct? |
|--------|--------------|-------|----------|
| **File** | `EXTRA-FILE:` | `basic::extras` | Yes |
| **Dir** | `EXTRA-DIR:` + `count_recursive(backup, Extra)` | `basic::extras` | Yes |
| **Sym→file** | `EXTRA-SYMLINK:` | `symlinks::extra_symlink_in_backup` | Yes |
| **Sym→dir** | `EXTRA-SYMLINK:` | No | Yes |
| **Sym→dangling** | `EXTRA-SYMLINK:` | No | Yes |
| **Special** | `NOT_A_FILE_OR_DIR:` (no `inc_extras`) | `edge_cases::special_file_extra_in_backup` | Yes |
| **Absent** | N/A — never visited | N/A | N/A |

---

## Notes

### Note 1: Double `inc_different` with `--follow` and different targets

When both sides are symlinks with different targets and `--follow` is used,
`inc_different()` is called once for `DIFFERENT-SYMLINK-TARGET` in
`handle_both_symlinks`, and may be called again inside `compare_entries` if the
resolved types also differ (e.g., file vs dir → `DIFFERENT-FILE [TYPE]`). This
means one entry can generate two "different" counts. Both reports are
individually meaningful — the targets differ AND the content types differ —
but the summary count may be surprising.

### Note 2: Both-dangling same-target with `--follow`

When both sides are identically dangling symlinks (same target, both
unresolvable) and `--follow` is used, the code counts them as different. This
is conservative — "we were asked to verify content but neither side has
content." One could argue two identical dangling symlinks should be a
similarity, but the current behavior is intentional (tested by
`dangling_symlinks_same_target_with_follow`).

---

## Test coverage gaps

All cells marked "No" above are **untested but produce correct behavior**.
The code paths are shared with tested cases, so the risk is low. Cases that
have unique code paths are all tested.

Summary of untested cells:

| Orig | Backup | Why untested risk is low |
|------|--------|------------------------|
| File | Sym→file | Reverse direction tested; same `DIFFERENT-SYMLINK-STATUS` branch |
| File | Sym→dir | Same branch as File vs Sym→file |
| File | Sym→dangling | Same branch as File vs Sym→file |
| File | Special | `is_special` short-circuits; tested from other orig types |
| Dir | Sym→dir | Same branch as Dir vs Sym→file (tested) |
| Dir | Sym→dangling | Same branch as Dir vs Sym→file (tested) |
| Dir | Special | `is_special` short-circuits; tested from other orig types |
| Sym→file | Sym→dangling (same) | Same `follow_symlinks` dangling path as tested diff-target cases |
| Sym→file | Sym→dangling (diff) | Same `follow_symlinks` dangling path as tested cases |
| Sym→dir | File | Same branch as Sym→file vs File (tested) |
| Sym→dir | Dir | Same branch as Sym→file vs Dir (tested) |
| Sym→dir | Sym→dangling (same) | Same `follow_symlinks` path as tested diff-target case |
| Sym→dir | Special | `is_special` short-circuits; tested from Sym→file |
| Sym→dangling | File | Same branch as Sym→file vs File |
| Sym→dangling | Dir | Same branch as Sym→file vs Dir |
| Sym→dangling | Sym→file (same) | Same `follow_symlinks` dangling path |
| Sym→dangling | Sym→dir (diff) | Same `follow_symlinks` dangling path |
| Sym→dangling | Sym→dir (same) | Same `follow_symlinks` dangling path |
| Sym→dangling | Special | `is_special` short-circuits |
| Sym→dangling | Absent | Same `handle_missing` symlink branch |
| Special | File | `is_special(orig)` short-circuits identically |
| Special | Dir | `is_special(orig)` short-circuits identically |
| Special | Sym→dir | Same as tested Sym→file case |
| Special | Sym→dangling | Same as tested Sym→file case |
| Absent | Sym→dir | Same `handle_extra` symlink branch as tested Sym→file |
| Absent | Sym→dangling | Same `handle_extra` symlink branch |

**All previous TODOs from type-matrix.md have been resolved:**

- TODO 1 (Sym→dir vs Sym→dir diff targets never compared): Fixed — `handle_both_symlinks` always compares targets before dispatching.
- TODO 2 (Special missing → MISSING-FILE): Fixed — `handle_missing` checks `is_special` first → `NOT_A_FILE_OR_DIR`.
- TODO 3 (Special extra → EXTRA-FILE): Fixed — `handle_extra` checks `is_special` first → `NOT_A_FILE_OR_DIR`.
- TODO 4 (Special vs Sym → DIFFERENT-SYMLINK-TARGET): Fixed — `is_special` check fires before symlink logic.
- TODO 5 (Dir vs Sym no recursive counting): Fixed — `DIFFERENT-SYMLINK-STATUS` branch counts dir contents.
- TODO 6 (Rename DIFFERENT-SYMLINK-TARGET for status mismatches): Fixed — split into `DIFFERENT-SYMLINK-TARGET` (both symlinks, targets differ) and `DIFFERENT-SYMLINK-STATUS` (one side is symlink, other is not).
- TODO 7 (Dangling symlinks with --follow → NOT_A_FILE_OR_DIR): Fixed — `follow_symlinks` handles dangling with `DANGLING-SYMLINK`.
