# @jakvbs/codex-mcp-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

NPM package for **codex-mcp-rs** - A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Codex CLI.

## Installation

```bash
npm install -g @jakvbs/codex-mcp-rs
```

This will automatically download and install the appropriate binary for your platform (Linux, macOS, or Windows).

## Usage with Claude Code

After installation, add to your Claude Code MCP configuration:

```bash
claude mcp add codex-rs -s user --transport stdio -- codex-mcp-rs
```

Or manually add to your `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "codex-rs": {
      "command": "codex-mcp-rs",
      "transport": "stdio"
    }
  }
}
```

## Features

- ✨ High-performance Rust implementation
- 🚀 Low memory footprint
- 🔒 Configurable Codex CLI flags (e.g. sandbox policy) via server config
- 🔄 Session management for multi-turn conversations
- 🖼️ Image attachment support
- ⚡ Fast async I/O with Tokio

## Supported Platforms

- Linux (x86_64, arm64)
- macOS (x86_64, arm64)
- Windows (x86_64, arm64)

## Prerequisites

You must have the [Codex CLI](https://github.com/anthropics/codex) installed and configured on your system.

## Tool Parameters

The server provides a `codex` tool with a minimal parameter surface:

- **PROMPT** (required): Task instruction
- **SESSION_ID** (optional): Resume a previously started Codex session (Codex
  `thread_id`). Use exactly the `SESSION_ID` value returned from an earlier
  `codex` tool call. When starting a new session, omit this field entirely
  instead of passing an empty string.
- **image** (optional, array): Image file paths to attach to the prompt. Paths
  may be absolute or relative to the current working directory.

Other Codex CLI flags such as `--sandbox`, `--yolo`, `--model`, `--profile`,
`--skip-git-repo-check`, and `--return-all-messages` are not MCP tool
parameters. Configure them globally in `src/codex.rs` via `default_additional_args()`
so they apply to every Codex invocation.

## Documentation

For detailed documentation, see the [GitHub repository](https://github.com/jakvbs/codex-mcp-rs).

## License

MIT License - Copyright (c) 2025 jakvbs

## Related Projects

- [codexmcp](https://github.com/GuDaStudio/codexmcp) - Python implementation
- [codex-mcp-go](https://github.com/w31r4/codex-mcp-go) - Go implementation
- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Gemini CLI MCP server
