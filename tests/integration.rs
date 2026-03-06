/// Integration tests for engram CLI.
///
/// All tests use `ENGRAM_TEST_EMBED=1` to activate the mock embedding provider,
/// allowing them to run without Ollama or any network access.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Return the path to the built `engram` binary.
fn engram_bin() -> PathBuf {
    // `cargo test` puts the binary in target/debug
    let mut path = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("parent of test binary")
        .parent()
        .expect("parent of deps dir")
        .to_path_buf();
    path.push("engram");
    path
}

/// Run engram with the given args, using a temporary DB path and mock embeddings.
fn run(db_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let db_path = db_dir.path().join("index.db");
    Command::new(engram_bin())
        .args(args)
        .env("ENGRAM_TEST_EMBED", "1")
        .env("ENGRAM_DB_PATH", db_path.to_str().unwrap())
        .output()
        .expect("failed to execute engram")
}

/// Helper: get stdout as String.
fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper: get stderr as String.
fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

// ---- Test cases ----

#[test]
fn add_single_file() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("hello.md");
    fs::write(
        &file,
        "Hello, world! This is a test document about Rust programming.",
    )
    .unwrap();

    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("Indexed 1 files"),
        "unexpected output: {text}"
    );
}

#[test]
fn add_directory_recursive() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();

    // Create nested structure
    let sub = data.path().join("subdir");
    fs::create_dir_all(&sub).unwrap();
    fs::write(
        data.path().join("top.md"),
        "Top-level document about testing.",
    )
    .unwrap();
    fs::write(sub.join("nested.txt"), "Nested document about integration.").unwrap();
    // Non-supported extension should be skipped
    fs::write(data.path().join("skip.json"), r#"{"not": "indexed"}"#).unwrap();

    let out = run(
        &db,
        &["add", "--no-progress", "-r", data.path().to_str().unwrap()],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    // Should index 2 supported files (top.md + nested.txt)
    assert!(
        text.contains("Indexed 2 files"),
        "unexpected output: {text}"
    );
}

#[test]
fn readd_unchanged_file_is_noop() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("stable.md");
    fs::write(&file, "This document never changes.").unwrap();

    // First add
    let out1 = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out1.status.success());
    assert!(stdout(&out1).contains("Indexed 1 files"));

    // Second add — same content, should skip
    let out2 = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out2.status.success());
    let text = stdout(&out2);
    assert!(
        text.contains("0 unchanged") || text.contains("1 unchanged"),
        "expected skip, got: {text}"
    );
    // 0 newly indexed
    assert!(
        text.contains("Indexed 0 files"),
        "expected 0 indexed, got: {text}"
    );
}

#[test]
fn search_returns_results_after_indexing() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("rust_guide.md");
    fs::write(
        &file,
        "Rust is a systems programming language focused on safety and performance.",
    )
    .unwrap();

    // Index
    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success());

    // Search
    let out = run(&db, &["search", "systems programming"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    // Should find at least one result containing the file path
    assert!(
        text.contains("rust_guide.md"),
        "expected result, got: {text}"
    );
}

#[test]
fn search_empty_index_returns_no_results() {
    let db = TempDir::new().unwrap();
    // Create an empty index by adding then searching on a fresh DB
    // We need to init the DB first — add a file then remove it, or just search
    // directly which should fail gracefully.
    let data = TempDir::new().unwrap();
    let file = data.path().join("temp.md");
    fs::write(&file, "temporary").unwrap();

    // Init the DB by adding a file
    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success());

    // Remove it
    let out = run(&db, &["remove", file.to_str().unwrap()]);
    assert!(out.status.success());

    // Search empty index
    let out = run(&db, &["search", "anything"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("No results"),
        "expected no results, got: {text}"
    );
}

#[test]
fn utf8_multibyte_content() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("unicode.md");
    fs::write(
        &file,
        "日本語のテスト文書。Ñoño café résumé naïve. Emoji: 🦀🔥✨ Деревья и горы.",
    )
    .unwrap();

    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("Indexed 1 files"),
        "unexpected output: {text}"
    );
}

#[test]
fn large_file_produces_multiple_chunks() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("large.md");

    // Create content larger than CHUNK_SIZE (6000 chars)
    let paragraph = "This is a paragraph about software architecture and design patterns. ";
    let content: String = paragraph.repeat(200); // ~14000 chars
    assert!(content.len() > 6000, "test content too small");
    fs::write(&file, &content).unwrap();

    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    // Should still index as 1 file (just with multiple chunks internally)
    assert!(
        text.contains("Indexed 1 files"),
        "unexpected output: {text}"
    );

    // Verify by searching — should find results
    let out = run(&db, &["search", "software architecture"]);
    assert!(out.status.success());
    assert!(
        stdout(&out).contains("large.md"),
        "expected result from large file"
    );
}

#[test]
fn status_reports_correct_counts() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();

    fs::write(data.path().join("a.md"), "Document alpha about databases.").unwrap();
    fs::write(data.path().join("b.txt"), "Document beta about networking.").unwrap();
    fs::write(data.path().join("c.md"), "Document gamma about compilers.").unwrap();

    // Index all files
    let out = run(
        &db,
        &["add", "--no-progress", data.path().to_str().unwrap()],
    );
    assert!(out.status.success());
    assert!(stdout(&out).contains("Indexed 3 files"));

    // Status should report 3 documents
    let out = run(&db, &["status"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("Documents : 3"), "unexpected status: {text}");
}

#[test]
fn deleted_file_keeps_index_entry() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("ephemeral.md");
    fs::write(&file, "This file will be deleted from the filesystem.").unwrap();

    // Index the file
    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Indexed 1 files"));

    // Delete the file from the filesystem
    fs::remove_file(&file).unwrap();

    // The index entry should still be present — status still shows 1 doc
    let out = run(&db, &["status"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("Documents : 1"),
        "index entry should persist after fs delete: {text}"
    );

    // Search should still find it
    let out = run(&db, &["search", "deleted"]);
    assert!(out.status.success());
    assert!(
        stdout(&out).contains("ephemeral.md"),
        "expected result from deleted file"
    );
}

#[test]
fn readd_changed_file_reindexes() {
    let db = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let file = data.path().join("mutable.md");
    fs::write(&file, "Original content about quantum computing.").unwrap();

    // First add
    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Indexed 1 files"));

    // Modify the file
    fs::write(
        &file,
        "Updated content about machine learning and neural networks.",
    )
    .unwrap();

    // Re-add — should re-index (not skip)
    let out = run(&db, &["add", "--no-progress", file.to_str().unwrap()]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(
        text.contains("Indexed 1 files") && text.contains("0 unchanged"),
        "expected re-index, got: {text}"
    );
}
