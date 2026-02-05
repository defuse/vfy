# Future Test TODOs

Tests that require special infrastructure or cannot be implemented with current testing approach.

## Requires Fault Injection

### I/O Error During Read
- [ ] I/O error during file read → other side handling
- [ ] I/O error partway through large file hash
- [ ] I/O error during directory listing
- [ ] Transient I/O errors (retry behavior if any)

**Why not implementable now:** Requires a FUSE filesystem or similar mechanism to inject controlled I/O errors. The `FileUnreadable` entry type only tests permission errors (EACCES), not I/O errors (EIO, ENOENT race, etc.).

**Potential approaches:**
1. FUSE filesystem that returns errors on specific files
2. LD_PRELOAD library to intercept read() calls
3. Testing on a flaky network filesystem (unreliable)
4. Kernel fault injection (requires root, Linux-specific)

## Requires Special Filesystem Features

### Sparse Files
- [ ] Sparse file comparison (files with holes)
- [ ] Sparse vs non-sparse with same logical content

**Why:** Creating sparse files portably is tricky; behavior may vary by filesystem.

### Extended Attributes / ACLs
- [ ] Files differing only in xattrs
- [ ] Files differing only in ACLs

**Why:** Not all filesystems support xattrs/ACLs; tool may not compare them anyway.

## Requires Timing Control

### Race Conditions
- [ ] File deleted between stat and read
- [ ] File modified between stat and hash
- [ ] Directory contents change during traversal
- [ ] Symlink target changes during comparison

**Why:** Requires precise timing control or multi-threaded test harness to reliably reproduce races.
