# Project Structure

```
codex-mcp-rs/
├── .github/
│   └── workflows/
│       ├── ci.yml              # CI workflow for testing and linting
│       └── release.yml         # Release automation workflow
├── npm/
│   ├── bin.js                  # NPM binary wrapper script
│   ├── install.js              # Post-install script to download binary
│   ├── package.json            # NPM package configuration
│   └── README.md               # NPM package documentation
├── scripts/
│   └── check-version.sh        # Version consistency checker
├── src/
│   ├── codex.rs                # Codex CLI wrapper implementation
│   ├── lib.rs                  # Library root
│   ├── main.rs                 # Binary entry point
│   └── server.rs               # MCP server and tool implementation
├── .cargo-release.toml         # Cargo release configuration
├── .gitignore                  # Git ignore rules
├── .npmignore                  # NPM package ignore rules
├── Cargo.lock                  # Cargo dependency lock file
├── Cargo.toml                  # Rust project configuration
├── CLAUDE.md                   # Claude Code guidance document
├── CONTRIBUTING.md             # Contribution guidelines
├── LICENSE                     # MIT License
├── Makefile                    # Development convenience commands
├── PROJECT_STRUCTURE.md        # This file
├── README.md                   # Main project documentation
└── server.json                 # MCP registry server configuration
```

## File Descriptions

### Core Source Files

- **src/main.rs**: Entry point that initializes the MCP server with stdio transport
- **src/server.rs**: Defines the `CodexServer` struct and implements the `codex` tool using rmcp macros
- **src/codex.rs**: Handles spawning the Codex CLI process and parsing its JSON output
- **src/lib.rs**: Library module declarations

### Build & Release

- **.github/workflows/ci.yml**: Runs tests and linting on every push/PR
- **.github/workflows/release.yml**: Builds multi-platform binaries and publishes to npm/MCP registry
- **Cargo.toml**: Rust dependencies and package metadata
- **.cargo-release.toml**: Configuration for cargo-release tool

### NPM Package

- **npm/package.json**: NPM package metadata and dependencies
- **npm/bin.js**: Wrapper script that executes the platform-specific binary
- **npm/install.js**: Downloads the correct binary from GitHub releases on installation
- **npm/README.md**: Documentation shown on npmjs.com

### Documentation

- **README.md**: Main project documentation with installation and usage instructions
- **CLAUDE.md**: Architecture and development guidance for Claude Code
- **CONTRIBUTING.md**: Guidelines for contributors
- **LICENSE**: MIT license text

### Configuration

- **server.json**: MCP registry metadata for server discovery
- **.gitignore**: Files to exclude from git
- **.npmignore**: Files to exclude from npm package
- **Makefile**: Convenience commands for development tasks

### Utilities

- **scripts/check-version.sh**: Ensures version consistency across Cargo.toml, package.json, and server.json

## Build Artifacts (Not in Repo)

- **target/**: Cargo build output (debug and release)
- **npm/node_modules/**: NPM dependencies for install script
- **npm/*.tar.gz, npm/*.zip**: Downloaded binary archives
- **npm/codex-mcp-rs[.exe]**: Extracted binary

## Development Workflow

1. **Make changes** to source files in `src/`
2. **Test locally**: `cargo test && cargo build`
3. **Lint**: `cargo fmt && cargo clippy`
4. **Update versions** in Cargo.toml, npm/package.json, and server.json
5. **Verify versions**: `make check-version`
6. **Commit and tag**: `git commit -am "chore: release v0.x.y" && git tag v0.x.y`
7. **Push**: `git push && git push --tags`
8. **CI/CD** automatically builds and publishes

## Release Process

When a `v*` tag is pushed:

1. **Build stage**: Compiles binaries for 6 platforms (Linux/macOS/Windows × x86_64/arm64)
2. **GitHub release**: Creates release with binaries attached
3. **NPM publish**: Publishes npm package (which downloads binaries on install)
4. **MCP registry**: Registers server for discovery in Claude Code

Users can then install via:
- `npm install -g @gudastudio/codex-mcp-rs`
- Direct binary download from GitHub releases
- Building from source with `cargo build --release`
