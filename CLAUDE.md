# engram — Claude Code Context

engram is an open-source Rust CLI that gives AI agents semantic memory.
It indexes plain text/markdown files into a local sqlite-vec database and
lets you search them by meaning, not keywords.

## Architecture

```
src/
  main.rs    — CLI entry point and arg dispatch
  cli.rs     — clap command/subcommand definitions
  db.rs      — SQLite + sqlite-vec operations (open, init, upsert, search, remove)
  embed.rs   — embedding providers: Ollama (/api/embeddings) + OpenAI-compatible
  index.rs   — add/search/remove/rebuild logic, chunking, file walking
tests/       — integration tests (to be created)
.github/     — CI/release workflows (to be created)
```

## Key implementation details

- SQLite is **bundled** via `rusqlite` (features = ["bundled"]) — no system dep
- sqlite-vec loaded via `sqlite3_auto_extension` + unsafe transmute
- Embedding provider auto-detected at index init, stored in metadata table
- Files chunked at 6000 chars with 200-char overlap, char-boundary safe
- Hash-based dedup: unchanged files are skipped (no re-embedding)
- `floor_char_boundary(s, idx)` helper for safe UTF-8 slicing

## Test mock provider

`ENGRAM_TEST_EMBED=1` env var activates a deterministic mock embed provider.
When set, `embed::embed()` must return a 768-dim `Vec<f32>` derived from the
input text without making any HTTP calls. Use blake3 hash of the text bytes
seeded into a deterministic float sequence. This allows all tests to run
without Ollama.

The mock must produce **different vectors for different inputs** so that
search ranking is meaningful in tests.

## Coding standards

- No `unwrap()` in library code — use `?` and `anyhow::Context`
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- All new public functions should have doc comments
- Commit messages: conventional commits (`feat:`, `fix:`, `test:`, `ci:`, `docs:`)

## Push credentials

The repo remote is already configured with HTTPS + token auth.
Use `git push origin main` directly — no extra setup needed.
