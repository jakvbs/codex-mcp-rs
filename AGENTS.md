# Repository Guidelines

## Project Structure & Module Organization
- Core code lives in `src/`: `main.rs` (entry), `server.rs` (MCP server + tool), `codex.rs` (Codex CLI wrapper), `lib.rs` (modules).
- Tests sit in `tests/`: `integration_tests.rs`, `server_tests.rs`, and `common/` helpers; unit tests live alongside code in `src/`.
- NPM packaging wrapper is under `npm/` (`bin.js`, `install.js`, `package.json`); keep binaries out of version control.
- Utilities and docs: `Makefile`, `scripts/check-version.sh`, `README.md`, `TESTING.md`, `CLAUDE.md`, `PROJECT_STRUCTURE.md`, `server.json`.

## Build, Test, and Development Commands
- Build: `cargo build` (debug) or `cargo build --release` (optimised). Run locally with `cargo run`.
- Fast paths: `make check` (fmt + clippy + tests) and `make ci` (check + release build).
- Quality gates: `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`.
- Tests: `cargo test`, `cargo test --lib`, `cargo test --test '*'`; verbose runs via `cargo test -- --nocapture`.
- Coverage: `cargo tarpaulin --out Html --out Xml`; keep reports out of the repo.
- Version sync before release: `make check-version` ensures `Cargo.toml`, `npm/package.json`, and `server.json` match.

## Coding Style & Naming Conventions
- Rust 2021 with rustfmt defaults (4-space indent, ordered imports); run fmt before commits.
- Prefer `anyhow::Result`/`?` for error flow; avoid `unwrap`/`expect` in server paths.
- Naming: snake_case for functions/vars/modules, CamelCase for types/traits, SCREAMING_SNAKE_CASE for consts, kebab-case for feature branches (e.g., `feature/tooling-updates`).

## Testing Guidelines
- Add unit tests near the code and integration coverage in `tests/integration_tests.rs` using `tests/common` helpers.
- Mirror existing naming (`test_*`) and keep tests deterministic; prefer table-driven cases for option parsing.
- Run `cargo test` plus `cargo fmt` and `cargo clippy` before pushing; capture stderr with `-- --nocapture` when debugging.
- For new surface areas, include at least one integration test exercising the MCP tool contract.

## Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`). Release commits follow `chore: release v0.x.y` before tagging.
- PRs should summarize changes, link issues, and note the commands you ran (fmt, clippy, tests). Include output snippets if behavior changes.
- Keep diffs focused; update docs/config (README, server.json, npm/package.json) when user-facing behavior or versions change.

## Security & Configuration Tips
- Respect sandbox expectations: code should not write outside the working directory unless explicitly allowed; keep defaults safe for `read-only` and `workspace-write`.
- When modifying transport or process spawning, ensure CLI arguments remain escaped (see `escape_prompt` tests) and validate paths from user input.
- Update `server.json` metadata when altering capabilities so MCP clients display accurate info.
