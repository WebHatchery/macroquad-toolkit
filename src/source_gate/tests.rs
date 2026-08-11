use super::*;

fn repeated(line: &str, n: usize) -> String {
    std::iter::repeat_n(line, n).collect::<Vec<_>>().join("\n")
}

#[test]
fn counts_every_line_including_blank_ones() {
    assert_eq!(source_lines("let a = 1;\n\n// comment\n"), 3);
}

#[test]
fn inline_test_module_lines_count_toward_the_limit() {
    let source = format!(
        "{}\n#[cfg(test)]\nmod tests {{\n{}\n}}\n",
        repeated("fn code() {}", 10),
        repeated("    // test line", 900)
    );
    assert_eq!(source_lines(&source), 913);
}

#[test]
fn test_module_declaration_lines_count_toward_the_limit() {
    let source = format!(
        "#[cfg(test)]\nmod tests;\n{}\n",
        repeated("fn code() {}", 10)
    );
    assert_eq!(source_lines(&source), 12);
}

#[test]
fn oversized_files_reports_offenders_worst_first_including_test_files() {
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
                path: "src/state/tests/huge.rs".into(),
                lines: 500
            },
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
#[should_panic(expected = "no project directory")]
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
