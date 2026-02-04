use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Stats {
    original_items: AtomicU64,
    backup_items: AtomicU64,
    missing: AtomicU64,
    different: AtomicU64,
    similarities: AtomicU64,
    extras: AtomicU64,
    not_a_file_or_dir: AtomicU64,
    skipped: AtomicU64,
    errors: AtomicU64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            original_items: AtomicU64::new(0),
            backup_items: AtomicU64::new(0),
            missing: AtomicU64::new(0),
            different: AtomicU64::new(0),
            similarities: AtomicU64::new(0),
            extras: AtomicU64::new(0),
            not_a_file_or_dir: AtomicU64::new(0),
            skipped: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    pub fn inc_original_items(&self) {
        self.original_items.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_backup_items(&self) {
        self.backup_items.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_missing(&self) {
        self.missing.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_different(&self) {
        self.different.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_similarities(&self) {
        self.similarities.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_extras(&self) {
        self.extras.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_not_a_file_or_dir(&self) {
        self.not_a_file_or_dir.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_skipped(&self) {
        self.skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    fn format_summary(&self) -> String {
        let orig = self.original_items.load(Ordering::Relaxed);
        let missing = self.missing.load(Ordering::Relaxed);
        let different = self.different.load(Ordering::Relaxed);
        let missing_different = missing + different;
        let pct = if orig > 0 {
            (missing_different as f64 / orig as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "SUMMARY:\n\
             \x20   Original items processed: {}\n\
             \x20   Backup items processed: {}\n\
             \x20   Missing/different: {} ({:.2}%)\n\
             \x20   Extras: {}\n\
             \x20   Not a file or dir: {}\n\
             \x20   Similarities: {}\n\
             \x20   Skipped: {}\n\
             \x20   Errors: {}",
            orig,
            self.backup_items.load(Ordering::Relaxed),
            missing_different, pct,
            self.extras.load(Ordering::Relaxed),
            self.not_a_file_or_dir.load(Ordering::Relaxed),
            self.similarities.load(Ordering::Relaxed),
            self.skipped.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }

    pub fn print_summary(&self) {
        println!("{}", self.format_summary());
    }

    /// Print summary to stderr (for ctrl-c handler when stdout may be broken).
    pub fn eprint_summary(&self) {
        eprintln!("{}", self.format_summary());
    }

    pub fn has_differences(&self) -> bool {
        self.missing.load(Ordering::Relaxed) > 0
            || self.different.load(Ordering::Relaxed) > 0
            || self.extras.load(Ordering::Relaxed) > 0
            || self.not_a_file_or_dir.load(Ordering::Relaxed) > 0
            || self.errors.load(Ordering::Relaxed) > 0
    }
}

#[derive(Debug, Default)]
pub struct DiffReasons {
    pub size: bool,
    pub sample: bool,
    pub hash: bool,
}

impl DiffReasons {
    pub fn any(&self) -> bool {
        self.size || self.sample || self.hash
    }
}

impl fmt::Display for DiffReasons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.size {
            parts.push("SIZE");
        }
        if self.sample {
            parts.push("SAMPLE");
        }
        if self.hash {
            parts.push("HASH");
        }
        write!(f, "{}", parts.join(", "))
    }
}
