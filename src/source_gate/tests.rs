use super::*;

fn repeated(line: &str, n: usize) -> String {
    std::iter::repeat_n(line, n).collect::<Vec<_>>().join("\n")
}

#[test]
fn counts_every_line_including_blank_ones() {
    assert_eq!(non_test_lines("let a = 1;\n\n// comment\n"), 3);
}

#[test]
fn inline_test_module_ends_the_count() {
    let source = format!(
        "{}\n#[cfg(test)]\nmod tests {{\n{}\n}}\n",
        repeated("fn code() {}", 10),
        repeated("    // test line", 900)
    );
    assert_eq!(non_test_lines(&source), 10);
}

#[test]
fn test_module_declaration_skips_only_itself() {
    let source = format!(
        "#[cfg(test)]\nmod tests;\n{}\n",
        repeated("fn code() {}", 10)
    );
    assert_eq!(non_test_lines(&source), 10);
}

#[test]
fn cfg_test_on_a_non_module_item_counts_normally() {
    let source = "#[cfg(test)]\nfn helper() {}\nfn code() {}\n";
    assert_eq!(non_test_lines(source), 3);
}

#[test]
fn attribute_and_module_on_one_line_end_the_count() {
    let source = format!(
        "fn code() {{}}\n#[cfg(test)] mod tests {{\n{}\n}}\n",
        repeated("x", 50)
    );
    assert_eq!(non_test_lines(&source), 1);
}

#[test]
fn extracted_test_files_are_exempt() {
    assert!(is_test_file(Path::new("foo/tests.rs")));
    assert!(is_test_file(Path::new("state/tests/ledger.rs")));
    assert!(!is_test_file(Path::new("state/ledger.rs")));
    assert!(!is_test_file(Path::new("contests.rs")));
}

#[test]
fn oversized_files_reports_offenders_worst_first_and_skips_test_files() {
    let root = std::env::temp_dir().join(format!("source_gate_test_{}", std::process::id()));
    let src = root.join("src").join("state");
    fs::create_dir_all(src.join("tests")).unwrap();
    fs::write(src.join("big.rs"), repeated("fn f() {}", 30)).unwrap();
    fs::write(src.join("bigger.rs"), repeated("fn f() {}", 40)).unwrap();
    fs::write(src.join("small.rs"), repeated("fn f() {}", 5)).unwrap();
    fs::write(src.join("tests").join("huge.rs"), repeated("x", 500)).unwrap();

    let over = oversized_files(&root, 20);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(
        over,
        vec![
            Oversized {
                path: "src/state/bigger.rs".into(),
                lines: 40
            },
            Oversized {
                path: "src/state/big.rs".into(),
                lines: 30
            },
        ]
    );
}

#[test]
#[should_panic(expected = "no src/")]
fn a_wrong_path_fails_instead_of_passing_clean() {
    oversized_files(std::env::temp_dir().join("source_gate_missing"), HARD_LIMIT);
}

#[test]
#[should_panic(expected = "remove them from the exception list")]
fn a_stale_grandfather_entry_fails_the_gate() {
    assert_source_files_within_limit(env!("CARGO_MANIFEST_DIR"), &["src/lib.rs"]);
}

// The gate applied to the crate it ships in: the toolkit holds itself to the
// same limit it enforces for the games.
#[test]
fn the_toolkit_passes_its_own_gate() {
    assert_source_files_within_limit(env!("CARGO_MANIFEST_DIR"), &[]);
}
