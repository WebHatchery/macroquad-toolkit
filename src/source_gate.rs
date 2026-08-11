//! Source file size gate — the 800-line hard limit from `CODE_STANDARDS.md`
//! §2.2, runnable under plain `cargo test`.
//!
//! The limit counts every physical line in every Rust source file, including
//! test files, examples, benches, and build scripts. There are no exceptions.
//!
//! A game adopts the gate with one integration test:
//!
//! ```ignore
//! // tests/code_standards.rs
//! #[test]
//! fn source_files_stay_under_the_limit() {
//!     macroquad_toolkit::source_gate::assert_source_files_within_limit(
//!         env!("CARGO_MANIFEST_DIR"),
//!         &[],
//!     );
//! }
//! ```
//!
//! Files already over the limit when the gate arrives go in the second
//! argument as paths relative to the manifest dir (forward slashes, e.g.
//! `"src/sim.rs"`). A grandfathered file is tolerated but ratcheted: once it
//! drops back under the limit the gate fails until its entry is removed, so
//! the list can only shrink.

use std::fs;
use std::path::{Path, PathBuf};

/// The hard limit from CODE_STANDARDS §2.2.
pub const HARD_LIMIT: usize = 800;

/// A source file over the limit: manifest-relative path (forward slashes) and
/// its total physical line count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oversized {
    pub path: String,
    pub lines: usize,
}

/// Counts every physical line in `source`.
pub fn source_lines(source: &str) -> usize {
    source.lines().count()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", dir.display()))
            .path();
        if path.is_dir()
            && !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | "target")
            )
        {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every Rust source file under `<manifest_dir>` whose total line count exceeds
/// `limit`, worst first. Build output and Git metadata are ignored.
pub fn oversized_files(manifest_dir: impl AsRef<Path>, limit: usize) -> Vec<Oversized> {
    let root = manifest_dir.as_ref();
    assert!(
        root.is_dir(),
        "source gate found no project directory at {} — check the path passed in",
        root.display()
    );
    let mut files = Vec::new();
    collect_rs_files(root, &mut files);
    let mut over: Vec<Oversized> = files
        .iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", path.display()));
            let lines = source_lines(&source);
            (lines > limit).then(|| Oversized {
                path: path
                    .strip_prefix(manifest_dir.as_ref())
                    .expect("file under manifest dir")
                    .to_string_lossy()
                    .replace('\\', "/"),
                lines,
            })
        })
        .collect();
    over.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.path.cmp(&b.path)));
    over
}

/// The gate: panics if any file under `<manifest_dir>/src` exceeds
/// [`HARD_LIMIT`] total lines, or if a `grandfathered` entry is no longer
/// over the limit (remove it — the list only shrinks).
pub fn assert_source_files_within_limit(manifest_dir: impl AsRef<Path>, grandfathered: &[&str]) {
    let over = oversized_files(&manifest_dir, HARD_LIMIT);
    let violations: Vec<&Oversized> = over
        .iter()
        .filter(|o| !grandfathered.contains(&o.path.as_str()))
        .collect();
    let stale: Vec<&&str> = grandfathered
        .iter()
        .filter(|g| !over.iter().any(|o| o.path == **g))
        .collect();

    let mut message = String::new();
    if !violations.is_empty() {
        message.push_str(&format!(
            "{} file(s) exceed the {HARD_LIMIT}-total-line hard limit (CODE_STANDARDS \u{a7}2.2):\n",
            violations.len()
        ));
        for o in &violations {
            message.push_str(&format!("  {}: {} lines\n", o.path, o.lines));
        }
        message.push_str(
            "Split a cohesive responsibility into a sibling module — never strip whitespace or compress formatting to pass.\n",
        );
    }
    if !stale.is_empty() {
        message.push_str("Grandfathered entries no longer over the limit — remove them from the exception list:\n");
        for g in &stale {
            message.push_str(&format!("  {g}\n"));
        }
    }
    assert!(message.is_empty(), "{message}");
}

#[cfg(test)]
mod tests;
