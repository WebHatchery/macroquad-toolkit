//! Source file size gate — the 800-line hard limit from `CODE_STANDARDS.md`
//! §2.2, runnable under plain `cargo test`.
//!
//! The limit counts **non-test lines only**: an inline `#[cfg(test)] mod tests`
//! block does not count, and extracted test files (`foo/tests.rs`, or anything
//! under a `tests/` directory inside `src/`) are exempt entirely. Blank lines
//! and comments in non-test code do count — the limit exists to catch files
//! that have taken on too many responsibilities, not to reward compressed
//! formatting.
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
/// its non-test line count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oversized {
    pub path: String,
    pub lines: usize,
}

/// Counts the lines of `source` that the file-size limits apply to.
///
/// An inline `#[cfg(test)]` module block ends the count — by §11.3 it sits at
/// the bottom of the file, so everything from the attribute on is test code. A
/// `#[cfg(test)] mod tests;` declaration only skips itself (the module body
/// lives in its own exempt file), and a `#[cfg(test)]` on any non-module item
/// counts normally.
pub fn non_test_lines(source: &str) -> usize {
    let lines: Vec<&str> = source.lines().collect();
    let mut count = 0;
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if let Some(rest) = trimmed.strip_prefix("#[cfg(test)]") {
            let mut item = rest.trim_start();
            let mut j = i;
            while item.is_empty() || item.starts_with("#[") || item.starts_with("//") {
                j += 1;
                match lines.get(j) {
                    Some(line) => item = line.trim_start(),
                    None => {
                        item = "";
                        break;
                    }
                }
            }
            let is_module = ["mod ", "pub mod ", "pub(crate) mod "]
                .iter()
                .any(|prefix| item.starts_with(prefix));
            if is_module {
                if item.contains('{') {
                    return count;
                }
                i = j + 1;
                continue;
            }
        }
        count += 1;
        i += 1;
    }
    count
}

/// True for extracted test files, which the limits exempt entirely:
/// `foo/tests.rs` and anything under a `tests/` directory.
fn is_test_file(relative_to_src: &Path) -> bool {
    relative_to_src
        .components()
        .any(|c| c.as_os_str() == "tests")
        || relative_to_src.file_stem().is_some_and(|s| s == "tests")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every non-test source file under `<manifest_dir>/src` whose non-test line
/// count exceeds `limit`, worst first. Panics if `src/` does not exist — a
/// wrong path must not read as a clean pass.
pub fn oversized_files(manifest_dir: impl AsRef<Path>, limit: usize) -> Vec<Oversized> {
    let src = manifest_dir.as_ref().join("src");
    assert!(
        src.is_dir(),
        "source gate found no src/ under {} — check the path passed in",
        manifest_dir.as_ref().display()
    );
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    let mut over: Vec<Oversized> = files
        .iter()
        .filter(|path| !is_test_file(path.strip_prefix(&src).expect("file under src")))
        .filter_map(|path| {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("source gate cannot read {}: {e}", path.display()));
            let lines = non_test_lines(&source);
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
/// [`HARD_LIMIT`] non-test lines, or if a `grandfathered` entry is no longer
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
            "{} file(s) exceed the {HARD_LIMIT} non-test-line hard limit (CODE_STANDARDS \u{a7}2.2):\n",
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
