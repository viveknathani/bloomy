# Repository Guidelines

## Project Structure & Module Organization

Bloomy is a Rust storage engine lab. Source code lives in `src/`.

- `src/lib.rs`: library entry point.
- `src/main.rs`: small binary entry point.
- `src/api/`: public key-value API types and traits.
- `src/engine/`: concrete storage engines, starting with `engine::lsm`.
- `src/storage/`: reusable pieces such as WAL, memtables, SSTables, manifests, and compaction.
- `src/config/`: JSON configuration loading and validation.
- `src/io/`: synchronous I/O helpers and future backends.
- `docs/ARCHITECTURE.md`: design overview.
- `docs/ROADMAP.md`: working implementation checklist.

Put unit tests near the code they exercise with `#[cfg(test)] mod tests`. Use `tests/` later for integration tests that exercise the public API.

## Build, Test, and Development Commands

- `cargo fmt`: format Rust code.
- `cargo test`: run unit, integration, and doc tests.
- `cargo build`: compile the project without running it.
- `cargo run`: run the small Bloomy binary.
- `cargo check`: quickly type-check during development.

Run `cargo fmt` and `cargo test` before submitting changes.

## Coding Style & Naming Conventions

Use standard Rust formatting through `rustfmt` with 4-space indentation. Prefer clear, explicit code over clever abstractions. Start synchronous; do not add async, macros, or unsafe code unless there is a documented reason.

Use Rust naming conventions:

- modules and files: `snake_case`
- functions and variables: `snake_case`
- structs, enums, and traits: `PascalCase`
- constants: `SCREAMING_SNAKE_CASE`

Keep placeholder modules minimal. Add comments only when they explain a real design decision or non-obvious behavior.

JSON configuration should stay explicit and validated. Prefer the typed
`BloomyConfig` model over passing loosely structured values through the engine.
The file format is `bloomy.json`; keep `docs/CONFIGURATION.md` updated when
fields change.

## Testing Guidelines

Bloomy uses Rust's built-in test framework. New behavior should include focused tests for correctness, especially around persistence, recovery, range scans, and compaction.

Name tests by behavior, for example:

- `put_then_get_returns_value`
- `delete_writes_tombstone`
- `replay_wal_recovers_memtable`

Prefer deterministic tests with temporary directories for storage files. Avoid relying on test order or shared state.

## Commit & Pull Request Guidelines

Use Conventional Commits. Keep the subject short and imperative:

- `feat: add wal record format`
- `fix: handle partial wal record replay`
- `docs: update lsm recovery notes`
- `test: cover tombstone range scans`
- `refactor: split sstable reader state`

Pull requests should include:

- a concise description of the change
- relevant tests run, usually `cargo test`
- updates to `docs/ARCHITECTURE.md` or `docs/ROADMAP.md` when design or scope changes
- linked issues when applicable
