# codex-mcp-rs

[![CI](https://github.com/jakvbs/codex-mcp-rs/workflows/CI/badge.svg)](https://github.com/jakvbs/codex-mcp-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Codex CLI for AI-assisted coding tasks.

> **Note**: This is a Rust port of the original Python implementation [codexmcp](../codexmcp). It offers the same functionality with improved performance and lower resource usage.

## Features

- **MCP Protocol Support**: Implements the official Model Context Protocol using the Rust SDK
- **Codex Integration**: Wraps the Codex CLI to enable AI-assisted coding through MCP
- **Session Management**: Supports multi-turn conversations via session IDs
- **Sandbox Safety**: Configurable sandbox policies (read-only, workspace-write, danger-full-access)
- **Image Support**: Attach images to prompts for visual context
- **Async Runtime**: Built on Tokio for efficient async I/O

## Prerequisites

- Rust 1.90+ (uses 2021 edition)
- [Codex CLI](https://github.com/anthropics/codex) installed and configured
- Claude Code or another MCP client

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Running

The server communicates via stdio transport:

```bash
cargo run
```

Or after building:

```bash
./target/release/codex-mcp-rs
```

## Installation

### Option 1: Install via NPM (Recommended)

The easiest way to install is via npm, which will automatically download the correct binary for your platform:

```bash
npm install -g @jakvbs/codex-mcp-rs
```

Then add to your Claude Code MCP configuration:

```bash
claude mcp add codex-rs -s user --transport stdio -- codex-mcp-rs
```

### Option 2: Install via Install Script (Linux/macOS)

Automatically download and install the latest release binary to `/opt/codex-mcp-rs/`:

```bash
curl -sSL https://raw.githubusercontent.com/jakvbs/codex-mcp-rs/master/scripts/install.sh | bash
```

This script will:
- Detect your platform and architecture
- Download the latest release from GitHub
- Install the binary to `/opt/codex-mcp-rs/codex-mcp-rs`
- Automatically add it to your Claude Code MCP configuration

### Option 3: Install from Release

Download the appropriate binary for your platform from the [releases page](https://github.com/jakvbs/codex-mcp-rs/releases), extract it, and add to your MCP configuration:

```bash
claude mcp add codex-rs -s user --transport stdio -- /path/to/codex-mcp-rs
```

### Option 4: Build from Source

```bash
git clone https://github.com/jakvbs/codex-mcp-rs.git
cd codex-mcp-rs
cargo build --release
claude mcp add codex-rs -s user --transport stdio -- $(pwd)/target/release/codex-mcp-rs
```

## Tool Usage

The server provides a single `codex` tool with a deliberately small parameter
surface. Most Codex CLI flags are configured globally in the server rather
than exposed as MCP parameters.

### Required Parameters

- `PROMPT` (string): Task instruction for Codex

### Optional Parameters

- `SESSION_ID` (string): Resume a previously started Codex session for
  multi-turn conversations. Use exactly the `SESSION_ID` value returned from an
  earlier `codex` tool call (typically a UUID). If omitted, a new session is
  created. Do not pass custom labels here.
- `image` (array of strings): One or more image file paths to attach to the
  initial prompt. Paths may be absolute or relative; each valid image is passed
  through to Codex CLI as a separate `--image <path>` argument.

## Configuration (JSON)

The server can load additional Codex CLI arguments and a default timeout from
`codex-mcp.config.json` in the current working directory, or from a path
specified via the `CODEX_MCP_CONFIG_PATH` environment variable.

Example:

```json
{
  "additional_args": [
    "--dangerously-bypass-approvals-and-sandbox",
    "--profile",
    "gpt-5"
  ],
  "timeout_secs": 600
}
```

`additional_args` are appended to every Codex CLI invocation after the core
flags (`--json`) and before any `resume`/`-- <prompt>` arguments.
`timeout_secs` controls the maximum runtime for each Codex execution:
- omitted or <= 0 → defaults to 600 seconds,
- values above 3600 are clamped to 3600 seconds.

### AGENTS.md System Prompt

The server automatically looks for an `AGENTS.md` file in the working directory. If found, its contents are prepended to every prompt as a system prompt, allowing you to define project-specific instructions or context:

**Example AGENTS.md:**
```markdown
# Project Context

You are working on a Rust project using the Tokio async runtime.
Always use proper error handling with `Result` and `?`.
Follow the project's code style in CLAUDE.md.
```

The contents will be wrapped in `<system_prompt>` tags and prepended before the user's prompt. Changes to `AGENTS.md` take effect immediately on the next invocation.

## Testing

The project has comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# See detailed testing guide
cat TESTING.md
```

Test categories:
- **Unit tests** (22): Core functionality including AGENTS.md handling, prompt processing, Options
- **Error flow tests** (9): Error handling and edge cases
- **Integration tests** (13): End-to-end scenarios including AGENTS.md integration (2 are Unix-only)
- **Server tests** (5): MCP protocol implementation
- **CI tests**: Multi-platform validation

Total: 49 tests passing ✅ (47 on Windows due to platform-specific tests)

Current test coverage: See [Codecov](https://codecov.io/gh/jakvbs/codex-mcp-rs)

## Architecture

See [CLAUDE.md](./CLAUDE.md) for detailed architecture documentation.

## Comparison with Other Implementations

| Feature | codex-mcp-rs (Rust) | codexmcp (Python) | codex-mcp-go |
|---------|---------------------|-------------------|--------------|
| Language | Rust | Python | Go |
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Memory Usage | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Binary Size | Medium | N/A | Small |
| Startup Time | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Session Management | ✓ | ✓ | ✓ |
| Image Support | ✓ | ✓ | ✓ |
| Sandbox Policies | ✓ | ✓ | ✓ |

## Related Projects

- [codexmcp](https://github.com/GuDaStudio/codexmcp) - Original Python implementation by guda.studio
- [codex-mcp-go](https://github.com/w31r4/codex-mcp-go) - Go implementation
- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Python MCP server for Gemini CLI

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

MIT License - Copyright (c) 2025 jakvbs

See [LICENSE](./LICENSE) for details.
