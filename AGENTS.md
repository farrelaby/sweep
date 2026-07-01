# AGENTS.md

## Commands

| Command | Purpose |
|---------|---------|
| `cargo test` | Run all tests (unit + integration) |
| `cargo clippy -- -D warnings` | Lint (warnings are errors) |
| `cargo fmt --check` | Check formatting |
| `cargo fmt` | Auto-format |
| `cargo install --path .` | Install locally (release mode by default) |

CI runs in order: **fmt → clippy → test**. Match this order locally.

## Architecture

Single crate, 6 modules:

| Module | Role |
|--------|------|
| `main.rs` | Entry point, CLI (clap), TUI event loop |
| `app.rs` | `AppState`, `TreeEntry`, selection/deletion logic |
| `scanner.rs` | Filesystem walk, lock file detection, size calculation |
| `config.rs` | Known target dirs + lock file → package manager mappings |
| `ui.rs` | ratatui rendering |
| `commands.rs` | `update` and `uninstall` subcommands (uses `self-replace`) |

Entry point: `main.rs:fn main()`. TUI loop: `main.rs:fn run_app()`. Library root: `lib.rs` re-exports all modules.

## Conventions

- **Rust edition 2024** — uses `let chains` (`if let ... && let ...`)
- **Tests**: `#[cfg(test)] mod tests` in each module file; integration tests in `tests/integration.rs`
- **Commits**: conventional commits (`feat:`, `fix:`, `docs:`, `test:`, etc.)
- **Clippy**: `-D warnings` — no warnings allowed
- **Do not commit unless explicitly asked** — always wait for user confirmation before committing
- **Changelog**: Always append a new `[Unreleased]` section to `CHANGELOG.md` for every release, documenting all changes since the last tagged version

## Branch Rules

- `main` is protected — all changes must go through a PR
- Feature branches named `kebab-case` (e.g. `ui-polish`, `language-based-redesign`)
- **Never mix version bumps into feature branches** — versioning commits belong on a dedicated release branch or directly on `main`
- **Release version bumps must always be prepared on their own dedicated branch** (e.g. `release/v0.2.2`), never alongside feature work

## Gotchas

- `tempfile` is in both `[dependencies]` and `[dev-dependencies]` — needed at runtime for `commands.rs` update downloads
- `self-replace` handles all platform-specific self-delete complexity (Unix: simple unlink; Windows: GC exe with `FILE_FLAG_DELETE_ON_CLOSE`)
- npm package (`npm/`) is a thin Node.js shim — Rust binary is downloaded at `postinstall` time via `install.js`
- Release workflow syncs version into `Cargo.toml` and `npm/package.json` via `sed`/`jq` before publishing
- 4 release targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`

## Release Flow

Tag push `v*` triggers `.github/workflows/release.yml`:

1. Build all 4 targets
2. Package (tar.gz Unix, zip Windows)
3. Generate `checksums.txt`
4. Create draft GitHub release
5. Publish to crates.io
6. Publish to npm
