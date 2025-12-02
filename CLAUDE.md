# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **codex-mcp-rs**, a Rust implementation of an MCP (Model Context Protocol) server that wraps the Codex CLI. It enables Claude Code to invoke Codex for AI-assisted coding tasks through the MCP protocol.

Related implementations in this workspace:
- `codexmcp/` - Python implementation with session persistence and parallel execution
- `codex-mcp-go/` - Go implementation
- `geminimcp/` - Python MCP server for Gemini CLI

## Build and Development Commands

### Building
```bash
cargo build              # Build in debug mode
cargo build --release    # Build optimized binary
```

### Running
```bash
cargo run                # Run the MCP server (listens on stdio)
```

### Testing
```bash
cargo test               # Run all tests
cargo test --lib         # Run library tests only
```

### Code Quality
```bash
cargo check              # Fast compilation check without producing binary
cargo clippy             # Lint with clippy
cargo fmt                # Format code
```

## Architecture

### Entry Point and Server Setup
The application follows a simple architecture:

1. **main.rs** - Entry point that initializes the MCP server with stdio transport
2. **server.rs** - Defines the `codex` MCP tool and handles parameter validation
3. **codex.rs** - Core Codex CLI wrapper that spawns processes and parses output
4. **lib.rs** - Module declarations

### Data Flow

```
Claude Code (MCP Client)
    ↓
stdio transport
    ↓
MCP Server (main.rs) → server::codex() tool
    ↓
codex::run() → spawns `codex exec` subprocess
    ↓
Parses JSON-streamed output line-by-line
    ↓
Returns CodexResult with session_id, agent_messages, all_messages
```

### Key Components

**server.rs:codex()** - MCP tool function that:
- Validates required parameters (PROMPT, cd)
- Validates working directory and image file paths exist
- Sets default values (sandbox="read-only", skip_git_repo_check=false)
- Calls `codex::run()` and formats response as `CodexOutput`

**codex.rs:run()** - Core execution function that:
- Builds the `codex exec` command with proper arguments
- Uses Windows-specific prompt escaping when needed
- Spawns subprocess with stdin=null, stdout/stderr=piped
- Streams stdout line-by-line, parsing JSON events
- Extracts `thread_id` (returned as SESSION_ID), `agent_message` items, and error types
- Returns `CodexResult` with all collected data

### Important Implementation Details

**Session Management**: The `SESSION_ID` (Codex's `thread_id`) enables multi-turn conversations. The server extracts it from JSON output and returns it to the client for subsequent calls.

**Error Handling**: The code checks for:
- Empty SESSION_ID (indicates failed session initialization)
- Empty agent_messages (indicates no response from Codex)
- Non-zero exit codes from the Codex subprocess
- JSON parse errors in streamed output

**Platform Differences**: Windows requires special prompt escaping (backslashes, quotes, newlines) to prevent shell interpretation issues.

**Streaming Output**: The Codex CLI outputs JSONL (JSON Lines). The server reads line-by-line to handle potentially long-running operations and collect all agent messages incrementally.

## Dependencies

The project uses:
- **rmcp** - Official Rust MCP SDK from `modelcontextprotocol/rust-sdk`
- **tokio** - Async runtime (required by rmcp)
- **serde/serde_json** - Serialization for MCP protocol and Codex output parsing
- **anyhow** - Error handling
- **uuid** - Session ID handling

## Codex CLI Integration

This server wraps the `codex exec` command. Key flags used:
- `--cd <path>` - Sets working directory
- `--sandbox <policy>` - Security policy (read-only/workspace-write/danger-full-access)
- `--json` - Enables JSON output streaming
- `--skip-git-repo-check` - Allows running outside git repos
- `--image <paths>` - Attaches images to prompt
- `--model <name>` - Specifies model override
- `--profile <name>` - Uses config profile from ~/.codex/config.toml
- `--yolo` - Disables approval prompts
- `resume <session_id>` - Continues previous session
- `-- <prompt>` - The task prompt (must come last)

### AGENTS.md System Prompt Support

The server automatically looks for an `AGENTS.md` file in the working directory. If found and non-empty, its contents are prepended to the user prompt as a system prompt:

```
<system_prompt>
[contents of AGENTS.md]
</system_prompt>

[user prompt]
```

This allows you to define project-specific instructions or context that will be included with every Codex invocation in that directory. The file is read on each invocation, so changes take effect immediately.

## Testing Strategy

The project includes comprehensive tests (49 total) covering:
- **Unit tests** (22): Core functionality including AGENTS.md reading with:
  - File existence handling (missing, empty, whitespace-only)
  - Size limit enforcement with UTF-8-aware truncation
  - Permission error handling (returns warnings, not errors)
  - Invalid UTF-8 detection and graceful degradation
  - Multibyte character boundary handling
  - Prompt handling, Options, sandbox policies
- **Error flow tests** (9): Edge cases including prompt escaping, size limits, timeouts
- **Integration tests** (13): End-to-end scenarios with codex CLI including AGENTS.md integration
- **Server tests** (5): MCP protocol implementation and security restrictions
- **CI tests**: Multi-platform validation (Linux, macOS, Windows)
