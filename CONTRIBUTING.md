# Contributing to codex-mcp-rs

Thank you for your interest in contributing to codex-mcp-rs!

## Development Setup

1. Install Rust (1.70+):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/jakvbs/codex-mcp-rs.git
   cd codex-mcp-rs
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test
   ```

## Code Style

- Run `cargo fmt` before committing to format code
- Run `cargo clippy` to check for common mistakes
- Follow Rust naming conventions and best practices

## Testing

Before submitting a PR:

1. Ensure all tests pass: `cargo test`
2. Check formatting: `cargo fmt -- --check`
3. Run clippy: `cargo clippy -- -D warnings`
4. Test the binary manually with a real Codex CLI installation

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Add tests if applicable
5. Commit with clear messages following [Conventional Commits](https://www.conventionalcommits.org/)
6. Push to your fork
7. Open a Pull Request

## Release Process

Releases are automated via GitHub Actions when a tag is pushed:

1. Update version in `Cargo.toml`
2. Update version in `npm/package.json`
3. Update version in `server.json`
4. Commit: `git commit -am "chore: release v0.x.y"`
5. Tag: `git tag v0.x.y`
6. Push: `git push && git push --tags`

The CI will automatically:
- Build binaries for all platforms
- Create a GitHub release
- Publish to npm
- Register with MCP registry

## Architecture

See [CLAUDE.md](./CLAUDE.md) for detailed architecture documentation.

## Questions?

Open an issue on GitHub or contact jakvbs
