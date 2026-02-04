# Symlink Handling: Intended Behavior

## Special files take priority everywhere

Before any symlink, dir, or file logic, check if either side is a special file
(device, FIFO, socket — not a regular file, not a directory, not a symlink).
If so, report `NOT_A_FILE_OR_DIR:` regardless of what the other side is.

This applies in all three code paths:

- `handle_both_present` — either side is special → `NOT_A_FILE_OR_DIR:`
- `handle_missing` — special file in orig, absent from backup → `NOT_A_FILE_OR_DIR:`
- `handle_extra` — special file in backup, absent from orig → `NOT_A_FILE_OR_DIR:`

## Entry point (`handle_both_present`)

After the special file check above, if either side is a symlink (via
`symlink_metadata`), the symlink handling logic below applies. If neither side
is a symlink, the existing dir/file comparison logic runs as normal.

## One side is a symlink, the other is not

Report `DIFFERENT-SYMLINK-STATUS:` and count as different.

Then, if the non-symlink side is a directory, recursively count its contents:

- Orig is dir, backup is symlink → `count_recursive(orig, Missing)`
- Orig is symlink, backup is dir → `count_recursive(backup, Extra)`

Otherwise, done.

## Both sides are symlinks

### 1. Compare targets

Read both symlink targets with `read_link`. If the targets differ, report
`DIFFERENT-SYMLINK-TARGET:` and count as different. (This happens regardless
of `--follow`.)

### 2. With `--follow`: compare resolved content

Resolve both symlinks. If either side is dangling (target doesn't exist),
report `DANGLING-SYMLINK:` for each dangling side and count as different.

If only one side is dangling, the resolved side's content is effectively
missing or extra:

- Orig resolves, backup dangling → orig content is missing from backup
  - orig is dir → `count_recursive(orig, Missing)`
  - orig is file → count as missing
- Orig dangling, backup resolves → backup content is extra
  - backup is dir → `count_recursive(backup, Extra)`
  - backup is file → count as extra

If both sides are dangling, nothing further to compare.

Otherwise (both resolve), compare using the existing non-symlink logic:

- dir vs dir → `compare_recursive`
- dir vs file → `DIFFERENT-FILE [TYPE]` + `count_recursive(orig, Missing)`
- file vs dir → `DIFFERENT-FILE [TYPE]` + `count_recursive(backup, Extra)`
- file vs file → `compare_file`
- special → `NOT_A_FILE_OR_DIR`

This runs whether or not the targets matched. If the targets differed, the user
sees both the target mismatch and any content differences inside.

### 3. Without `--follow`: skip

Report `SYMLINK:` and count as skipped. This applies to all symlinks with
matching targets, not just directory symlinks. The user is warned that the
actual content behind the symlink was not verified.

## Decision tree

```
either side is special (device/FIFO/socket) → NOT_A_FILE_OR_DIR
neither side is symlink → existing dir/file logic
one side not symlink → DIFFERENT-SYMLINK-STATUS + different
├── orig is dir → count_recursive(orig, Missing)
├── backup is dir → count_recursive(backup, Extra)
└── else → done
both symlinks:
├── read_link, compare targets
│   └── targets differ → DIFFERENT-SYMLINK-TARGET + different
├── --follow:
│   ├── both dangling → DANGLING-SYMLINK: + different
│   ├── orig resolves, backup dangling → DANGLING-SYMLINK: + missing
│   │   └── orig is dir → count_recursive(orig, Missing)
│   ├── orig dangling, backup resolves → DANGLING-SYMLINK: + extra
│   │   └── backup is dir → count_recursive(backup, Extra)
│   └── both resolve → compare resolved types using existing dir/file/special logic
└── no --follow → SYMLINK: + skip
```
