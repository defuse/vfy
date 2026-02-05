# Test Coverage TODO

## Requested Tests

### Error Handling - Other Side Counted as Missing/Extra

- [x] Unreadable dir in original → backup contents counted as extras
  - **Implemented by:** `errors::unreadable_dir_in_original`
- [x] Unreadable dir in backup → original contents counted as missing
  - **Implemented by:** `errors::unreadable_dir_in_backup`
- [x] Unreadable file in original → backup file NOT counted as extra (safe behavior)
  - **Implemented by:** `errors::unreadable_file_in_original`
  - **Design:** Asymmetric - don't suggest deletion of potentially valid backup
- [x] Unreadable file in backup → original file counted as missing
  - **Implemented by:** `errors::unreadable_file_in_backup`, `errors::unreadable_file_in_backup_counts_missing`
  - **Design:** Conservative - alert user to investigate backup issue
- [x] Permission error on file during sampling → original counted as missing
  - **Implemented by:** `errors::unreadable_file_with_sampling`
  - **Design:** Conservative - alert user to investigate backup issue
- [x] Unreadable dir in original → backup contents NOT counted as extras (safe behavior)
  - **Implemented by:** `errors::unreadable_dir_in_original`
  - **Design:** Asymmetric - don't suggest deletion of potentially valid backup
- [x] I/O error during read → other side handling
  - **Moved to:** `docs/future-test-todos.md` (requires fault injection)

### Matrix Tests - Dirs with Children

- [x] `file_x_dir` has child in dir
  - **Implemented by:** `matrix::file_x_dir` and `matrix::file_x_dir_vv`
- [x] `dir_x_dir` has children both sides
  - **Implemented by:** `matrix::dir_x_dir`
- [x] `dir_x_fifo` has child in dir
  - **Implemented by:** `matrix::dir_x_fifo`
- [ ] `dir_x_absent` - verify it has children to test recursive counting
- [ ] `dir_x_symlink_to_file` - add children to test counting
- [ ] `dir_x_symlink_to_dir` - add children to test counting
- [ ] `dir_x_symlink_dangling` - add children to test counting
- [ ] All dir_x_* tests should systematically have children

### Symlink Loops

- [x] Simple loop: `a -> b -> a` with --follow reports ERROR gracefully
  - **Implemented by:** `symlink_loops::two_link_loop_with_follow`
- [x] Self-referential: `a -> a` with --follow reports ERROR
  - **Implemented by:** `symlink_loops::self_loop_with_follow`
- [x] Longer chain loop: `a -> b -> c -> a` with --follow
  - **Implemented by:** `symlink_loops::three_link_chain_loop_with_follow`
- [x] Loop in subdirectory during traversal
  - **Implemented by:** `symlink_loops::loop_in_subdir_with_follow`
- [x] Loop detection increments error count correctly
  - **Verified by:** all `*_with_follow` tests check Errors count
- [x] Loop in one tree, valid path in other tree
  - **Implemented by:** `symlink_loops::loop_in_orig_valid_in_backup`, `symlink_loops::valid_in_orig_loop_in_backup`
- [x] No error without --follow (symlinks compared by target only)
  - **Implemented by:** `symlink_loops::self_loop_no_follow`, `symlink_loops::two_link_loop_no_follow`

### Nested Symlinks (Symlink Chains)

- [x] Symlink to symlink to file (2-level chain)
  - **Implemented by:** `symlinks_nested::chain_to_file_no_follow`, `symlinks_nested::chain_to_file_with_follow`
- [x] Symlink to symlink to dir (2-level chain)
  - **Implemented by:** `symlinks_nested::chain_to_dir_no_follow`, `symlinks_nested::chain_to_dir_with_follow`
- [x] Symlink to symlink to dangling (chain ending in dangling)
  - **Implemented by:** `symlinks_nested::chain_to_dangling_no_follow`, `symlinks_nested::chain_to_dangling_with_follow`
- [x] Deep symlink chain (3+ levels)
  - **Implemented by:** `symlinks_nested::deep_chain_three_levels_no_follow`, `symlinks_nested::deep_chain_three_levels_with_follow`
- [x] Mixed chain: symlink -> symlink -> dir with files inside
  - **Implemented by:** `symlinks_nested::chain_to_dir_with_follow`, `symlinks_nested::chain_to_dir_with_differences_follow`
- [x] Chain with --follow vs without --follow behavior
  - **Implemented by:** All `symlinks_nested::*_no_follow` and `*_with_follow` test pairs
- [x] Symlink to directory containing symlinks (--follow traverses into dir, finds more symlinks)
  - **Implemented by:** `symlinks_nested::symlink_to_dir_containing_symlink_with_follow`

### Inside Missing/Extra Directories

#### FIFOs Inside Missing/Extra Dirs
- [x] FIFO inside missing directory (orig has dir with FIFO, backup missing dir)
  - **Implemented by:** `inside_missing_extra::fifo_inside_missing_dir`, `inside_missing_extra::fifo_inside_missing_dir_vv`
- [x] FIFO inside extra directory (backup has dir with FIFO, orig missing dir)
  - **Implemented by:** symmetric tests via case! macro
- [x] Multiple FIFOs inside missing/extra dir
  - **Implemented by:** `inside_missing_extra::multiple_fifos_inside_missing_dir`
- [x] FIFO nested deeply inside missing/extra dir
  - **Implemented by:** `inside_missing_extra::fifo_nested_deeply_inside_missing_dir`, `inside_missing_extra::fifo_nested_deeply_inside_missing_dir_vv`

#### Errors Inside Missing/Extra Dirs
- [x] Unreadable file inside missing directory
  - **Implemented by:** `inside_missing_extra::unreadable_file_inside_missing_dir`, `inside_missing_extra::unreadable_file_inside_missing_dir_vv`
  - **Note:** No ERROR when dir is missing (only stat, not read)
- [x] Unreadable file inside extra directory
  - **Implemented by:** symmetric tests via case! macro
- [x] Unreadable subdir inside missing directory
  - **Implemented by:** `inside_missing_extra::unreadable_subdir_inside_missing_dir`, `inside_missing_extra::unreadable_subdir_inside_missing_dir_vv`
- [x] Unreadable subdir inside extra directory
  - **Implemented by:** symmetric tests via case! macro

#### Dangling Symlinks Inside Missing/Extra Dirs
- [x] Dangling symlink inside missing directory (no --follow)
  - **Implemented by:** `inside_missing_extra::dangling_symlink_inside_missing_dir_no_follow`, `inside_missing_extra::dangling_symlink_inside_missing_dir_no_follow_vv`
- [x] Dangling symlink inside missing directory (with --follow)
  - **Implemented by:** `inside_missing_extra::dangling_symlink_inside_missing_dir_with_follow`, `inside_missing_extra::dangling_symlink_inside_missing_dir_with_follow_vv`
- [x] Dangling symlink inside extra directory (no --follow)
  - **Implemented by:** symmetric tests via case! macro
- [x] Dangling symlink inside extra directory (with --follow)
  - **Implemented by:** symmetric tests via case! macro

#### Counting Inside Missing/Extra Dirs
- [x] Basic counting without --follow
  - **Implemented by:** `basic::nested`, `basic::nested_vv`
- [x] Counting with --follow when symlinks inside missing/extra dir
  - **Implemented by:** `inside_missing_extra::counting_with_follow_symlink_inside_missing_dir`, `inside_missing_extra::counting_with_follow_symlink_inside_missing_dir_vv`
- [x] Mixed content (files, dirs, symlinks, FIFOs) inside missing dir - verify counts
  - **Implemented by:** `inside_missing_extra::mixed_content_inside_missing_dir`, `inside_missing_extra::mixed_content_inside_missing_dir_vv`, `inside_missing_extra::mixed_content_inside_missing_dir_follow`
- [x] Mixed content inside extra dir - verify counts
  - **Implemented by:** symmetric tests via case! macro

#### Verbosity for Missing/Extra Dir Contents
- [x] No -v: top-level missing/extra shown, children not listed individually
  - **Implemented by:** `basic::nested`, `errors::type_mismatch_combined`
- [x] -vv: all children listed individually
  - **Implemented by:** `basic::nested_vv`, `errors::type_mismatch_combined_vv`
- [x] -v (single): behavior for missing/extra dir contents (should match no -v?)
  - **Implemented by:** `inside_missing_extra::single_v_missing_dir_contents`
- [x] Verify FIFO inside missing dir output at each verbosity level
  - **Implemented by:** `inside_missing_extra::fifo_verbosity_no_v`, `inside_missing_extra::fifo_verbosity_single_v`, `inside_missing_extra::fifo_verbosity_vv`
- [x] Verify dangling symlink inside missing dir output at each verbosity level
  - **Implemented by:** `inside_missing_extra::dangling_verbosity_no_v_no_follow`, `inside_missing_extra::dangling_verbosity_no_v_with_follow`, `inside_missing_extra::dangling_verbosity_vv_with_follow`

### Verbosity Behavior Outside Missing/Extra

#### Hash Output with --all
- [x] Hashes appear with --all and -vv
  - **Implemented by:** `flags::verbose_blake3_known_hashes`
- [x] Known hash values verified
  - **Implemented by:** `flags::verbose_blake3_known_hashes`, `edge_cases::zero_byte_files_with_all`
- [ ] Hashes do NOT appear with --all and -v (only -vv)
- [ ] Hashes do NOT appear with --all and no verbosity
- [ ] Hash output format consistency across file types

#### Directory Comparison Output
- [x] -v shows directory comparisons ("DEBUG: Comparing")
  - **Implemented by:** `flags::verbose_dirs_only`
- [x] -v does NOT show file comparisons
  - **Implemented by:** `flags::verbose_dirs_only`
- [ ] Verify exact format of -v directory comparison lines
- [ ] -v with --follow shows symlink-resolved directory comparisons

#### File/Entry Comparison Output
- [x] -vv shows file comparisons ("DEBUG: Comparing file")
  - **Implemented by:** `flags::verbose_files`
- [ ] -vv shows symlink comparisons separately from target comparisons with --follow
- [ ] -vv output for each entry type (file, dir, symlink, FIFO)
- [ ] Verify no DEBUG output without -v flag

#### Symlink-Specific Verbosity
- [ ] Symlink comparison line separate from resolved target line with --follow -vv
- [ ] SYMLINK-SKIPPED output at different verbosity levels
- [ ] DANGLING-SYMLINK output at different verbosity levels

---

## Recommended Additional Tests for Excellent Coverage

### Edge Cases Not Currently Tested

#### Filesystem Boundaries
- [ ] --one-filesystem actually stops at mount points
- [x] Symlink crossing filesystem boundary with --follow --one-filesystem

#### Large Scale
- [ ] Very deep directory nesting (100+ levels)
- [ ] Very wide directory (10,000+ files)
- [ ] Very long filename (255 chars)
- [ ] Very long path (4096 chars approaching PATH_MAX)

#### Race Conditions / Filesystem Changes
- [ ] File deleted between stat and read
- [ ] File modified between stat and hash
- [ ] Directory modified during traversal

#### Special Characters
- [ ] Filenames with newlines
- [ ] Filenames with null bytes (if filesystem allows)
- [ ] Filenames with unicode/emoji
- [ ] Filenames with shell metacharacters

#### Empty/Degenerate Cases
- [ ] Empty file vs non-empty file (currently only same-empty tested)
- [ ] Directory containing only unreadable entries
- [ ] Directory containing only FIFOs
- [ ] Directory containing only dangling symlinks

#### Permission Variations
- [ ] Write-only file (no read permission)
- [ ] Execute-only directory
- [ ] Sticky bit directories
- [ ] SUID/SGID files

#### Symlink Edge Cases
- [ ] Symlink with empty target ("")
- [ ] Symlink to "."
- [ ] Symlink to ".."
- [ ] Symlink with very long target path
- [ ] Broken symlink that previously worked (target deleted)
- [ ] Relative vs absolute symlink targets comparison

#### Content Comparison Edge Cases
- [ ] Binary files with null bytes
- [ ] Files differing only in last byte
- [ ] Files differing only in first byte
- [ ] Sparse files
- [ ] Files with holes

#### Summary/Statistics
- [ ] Percentage calculations with 0 total items
- [ ] Percentage calculations with 1 item
- [ ] Very large counts (overflow protection)

### Combinatorial Coverage Gaps

#### --ignore with Other Flags
- [ ] --ignore with --all (partially tested)
- [ ] --ignore with --follow on ignored symlink
- [ ] --ignore with -s (sampling)
- [ ] Multiple --ignore paths overlapping

#### --follow Combinations
- [ ] --follow with --one-filesystem
- [ ] --follow with -s (sampling through symlinks)
- [ ] --follow with --all on symlinked files

#### Sampling (-s) Edge Cases
- [ ] -s 0 behavior
- [ ] -s larger than file size (this seems useless, it will just sample that many times)
- [ ] -s on empty files
- [ ] -s detecting difference at exact sample boundary

### Output Format Tests

#### CMD Line Reproduction
- [ ] CMD line can be copy-pasted to reproduce run
- [ ] Special characters in paths properly escaped in CMD
- [ ] All flag combinations appear correctly in CMD

#### Summary Block
- [ ] All summary lines present in all scenarios
- [ ] Percentages formatted correctly (2 decimal places)
- [ ] Alignment/formatting of summary block

#### Exit Codes
- [x] Exit 0 when all match
- [x] Exit 1 when differences found
- [x] Exit 2 for CLI errors
- [ ] Exit code precedence (error vs difference)
