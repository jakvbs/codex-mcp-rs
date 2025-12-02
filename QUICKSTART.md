# Quick Start Guide

Get started with codex-mcp-rs in 5 minutes!

## Prerequisites

1. Install [Codex CLI](https://github.com/anthropics/codex):
   ```bash
   # Follow Codex installation instructions
   codex --version
   ```

2. Install [Claude Code](https://docs.claude.com/docs/claude-code):
   ```bash
   # Follow Claude Code installation instructions
   claude --version
   ```

## Installation

### Using NPM (Recommended)

```bash
# Install globally
npm install -g @jakvbs/codex-mcp-rs

# Add to Claude Code
claude mcp add codex-rs -s user --transport stdio -- codex-mcp-rs
```

### Using Pre-built Binary

1. Download from [releases](https://github.com/jakvbs/codex-mcp-rs/releases)
2. Extract the archive
3. Add to Claude Code:
   ```bash
   claude mcp add codex-rs -s user --transport stdio -- /path/to/codex-mcp-rs
   ```

### Building from Source

```bash
# Clone repository
git clone https://github.com/jakvbs/codex-mcp-rs.git
cd codex-mcp-rs

# Build release binary
cargo build --release

# Add to Claude Code
claude mcp add codex-rs -s user --transport stdio -- $(pwd)/target/release/codex-mcp-rs
```

## Verification

Check that the server is registered:

```bash
claude mcp list
```

You should see:
```
codex-rs: codex-mcp-rs - ✓ Connected
```

## Basic Usage

In Claude Code, you can now use the `codex` tool:

```
Use the codex tool to implement a function that calculates fibonacci numbers
in /home/user/my-project
```

Claude Code will call the codex tool with:
```json
{
  "PROMPT": "implement a function that calculates fibonacci numbers",
  "cd": "/home/user/my-project"
}
```

## Common Use Cases

### 1. Generate Code

```
Use codex to create a REST API server in Go with CRUD operations
Working directory: /home/user/my-api
```

### 2. Fix Bugs

```
Use codex to debug and fix the error in src/main.rs
Working directory: /home/user/my-rust-project
```

### 3. Refactor Code

```
Use codex to refactor the authentication module to use JWT
Working directory: /home/user/my-web-app
Sandbox: workspace-write
```

### 4. Multi-turn Conversation

```
First call:
Use codex to analyze the codebase structure in /home/user/my-app

Second call (using SESSION_ID from first response):
Now suggest improvements to the architecture
SESSION_ID: <previous-session-id>
```

## Configuration

### Sandbox Policies

- **read-only** (default): Codex can only read files, not modify them
- **workspace-write**: Codex can modify files in the working directory
- **danger-full-access**: Codex has full system access (use with caution!)

### Working Directory

Always specify an absolute path:
```json
{
  "cd": "/home/user/project"  // ✓ Good
}
```

Not relative paths:
```json
{
  "cd": "../project"  // ✗ Bad
}
```

## Troubleshooting

### "command not found: codex-mcp-rs"

NPM binary not in PATH. Try:
```bash
npm list -g @jakvbs/codex-mcp-rs
which codex-mcp-rs
```

If installed, add npm global bin to PATH:
```bash
export PATH="$PATH:$(npm bin -g)"
```

### "working directory does not exist"

Ensure the path exists:
```bash
ls -la /path/to/directory
```

### "Failed to execute codex"

Check Codex CLI is installed:
```bash
codex --version
```

### Server won't start

Check logs:
```bash
claude mcp logs codex-rs
```

## Next Steps

- Read [README.md](./README.md) for detailed features
- See [CLAUDE.md](./CLAUDE.md) for architecture details
- Check [CONTRIBUTING.md](./CONTRIBUTING.md) to contribute
- Browse [examples](./examples/) for code samples

## Getting Help

- 🐛 [Report bugs](https://github.com/jakvbs/codex-mcp-rs/issues)
- 💬 [Discussions](https://github.com/jakvbs/codex-mcp-rs/discussions)
