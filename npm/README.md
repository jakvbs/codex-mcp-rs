# @missdeer/codex-mcp-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

NPM package for **codex-mcp-rs** - A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Codex CLI.

## Installation

```bash
npm install -g @missdeer/codex-mcp-rs
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
- 🔒 Configurable sandbox policies
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

The server provides a `codex` tool with the following parameters:

- **PROMPT** (required): Task instruction
- **cd** (required): Working directory
- **sandbox**: Security policy (read-only, workspace-write, danger-full-access)
- **SESSION_ID**: Resume previous session
- **skip_git_repo_check**: Allow running outside git repos
- **return_all_messages**: Return full reasoning trace
- **image**: Attach image files
- **model**: Override Codex model
- **yolo**: Disable all prompts
- **profile**: Load config profile

## Documentation

For detailed documentation, see the [GitHub repository](https://github.com/missdeer/codex-mcp-rs).

## License

MIT License - Copyright (c) 2025 missdeer

## Related Projects

- [codexmcp](https://github.com/GuDaStudio/codexmcp) - Python implementation
- [codex-mcp-go](https://github.com/w31r4/codex-mcp-go) - Go implementation
- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Gemini CLI MCP server
