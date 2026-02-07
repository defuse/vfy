# vfy

`vfy` is a directory comparison tool, useful for checking if backups have been
completed or restored successfully.

By default, it compares only by file size, but it also supports checking random
samples within files (with `--samples N`) or full BLAKE3 hash-based comparison
(with `--all`).

To install, clone the repo and run `cargo install --path .` and make sure
`~/.cargo/bin` is in your `$PATH`.

```
$ vfy
CMD: vfy
Verify backup integrity by comparing directory trees. By default, only compares file sizes.

Usage: vfy [OPTIONS] <ORIGINAL> <BACKUP>

Arguments:
  <ORIGINAL>  Original directory
  <BACKUP>    Backup directory

Options:
  -v, --verbose...         Verbose output (-v for dirs, -vv for files, hashes with --all, see below)
  -s, --samples <SAMPLES>  Number of random samples to compare per file [default: 0]
  -a, --all                Full BLAKE3 hash comparison
  -f, --follow             Compare symlinked-to contents (symlink target paths are always compared, even without --follow)
  -o, --one-filesystem     Stay on one filesystem (only supported on Unix-like OSes)
  -i, --ignore <IGNORE>    Ignore one directory or file. Must exist. Ignoring one side also ignores the other.
  -h, --help               Print help

WARNING: Output behavior is currently NOT STABLE between releases.
WARNING: This release has only been tested on Linux.

Verbosity levels:
  (default)  Show differences only. For missing/extra directories, only the
             top-level directory is listed; children are counted but not shown.
  -v         Add DEBUG lines showing each directory comparison.
  -vv        Add DEBUG lines for file comparisons. Show all individual entries
             inside missing/extra directories. With --all, show BLAKE3 hashes.

Output prefixes (grep-friendly):
  MISSING-FILE:                  File in original missing from backup
  MISSING-DIR:                   Directory in original missing from backup
  MISSING-SYMLINK:               Symlink in original missing from backup
  MISSING-SPECIAL:               Special file in original missing from backup
  MISSING-ERROR:                 Something (that errored) in original missing from backup
  EXTRA-FILE:                    File in backup not in original
  EXTRA-DIR:                     Directory in backup not in original
  EXTRA-SYMLINK:                 Symlink in backup not in original
  EXTRA-SPECIAL:                 Extra special file in backup not in original
  EXTRA-ERROR:                   Extra something (that errored) in backup not in original
  DIFFERENT-FILE [reason]:       File differs (reason: first mismatch of SIZE, SAMPLE, HASH)
  FILE-DIR-MISMATCH:             One side is a file, the other is a directory
  DIFFERENT-SYMLINK-TARGET:      Both sides are symlinks but point to different targets
  DIFFERENT-SYMLINK-STATUS:      One side is a symlink, the other is not
  SPECIAL-FILE:                  Entry is a device, FIFO, socket, etc.
  SYMLINK-SKIPPED:               Symlink skipped (use --follow to compare resolved content)
  DANGLING-SYMLINK:              Symlink target does not exist (with --follow)
  DIFFERENT-FS:                  Different filesystem skipped (--one-filesystem)
  SKIP:                          Entry skipped via --ignore or error/FS/type mismatch between sides
  ERROR:                         I/O or permission error
  DEBUG:                         Verbose logging (-v dirs, -vv files and hashes)
  SUMMARY:                       Final counts (not guaranteed to add up to 100%)

Symlink handling with --follow:
  When both sides are symlinks with different targets:
    - Reports DIFFERENT-SYMLINK-TARGET as a warning
    - Continues comparing resolved contents (may find similarities)

  When one side is a symlink and the other is a regular file/directory:
    - Reports DIFFERENT-SYMLINK-STATUS as structural mismatch
    - Reports original as MISSING-*, backup symlink as EXTRA-* (or vice-versa)
    - Does NOT compare contents (structural failure means no backup exists)

  Rationale: A symlink replacing a directory is a structural failure--the backup
  tree doesn't contain the actual data. Two symlinks with different targets is
  a metadata difference--the resolved data may still be equivalent.
```

Note: The `--one-filesystem` tests assume your development environment is a
Linux system with `/dev/shm/` writable. Most of the tests are broken on Windows
due to the use of a Unix-specific filesystem library. As such, those platforms
are not officially supported, but it builds and seems to work fine.

**AI Use Disclosure:** This tool was developed with the aid of claude code.
