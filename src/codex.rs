use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Options {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub session_id: Option<String>,
    /// Extra CLI flags to pass through to the Codex CLI.
    pub additional_args: Vec<String>,
    /// Image files to attach to the Codex prompt.
    pub image_paths: Vec<PathBuf>,
    /// Timeout in seconds for the codex execution. If None, defaults to 600 seconds (10 minutes).
    /// Set to a specific value to override. The library enforces a timeout to prevent unbounded execution.
    pub timeout_secs: Option<u64>,
}

const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_TIMEOUT_SECS: u64 = 3600;

/// Configuration loaded from `codex-mcp.config.json` (or `CODEX_MCP_CONFIG_PATH`).
#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    #[serde(default)]
    additional_args: Vec<String>,
    timeout_secs: Option<u64>,
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("CODEX_MCP_CONFIG_PATH") {
        let trimmed = env_path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    // Fallback: config file in the current working directory
    std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("codex-mcp.config.json"))
}

fn load_server_config() -> ServerConfig {
    let mut cfg = ServerConfig {
        additional_args: Vec::new(),
        timeout_secs: None,
    };

    let Some(config_path) = resolve_config_path() else {
        return cfg;
    };

    if !config_path.is_file() {
        return cfg;
    }

    match std::fs::read_to_string(&config_path) {
        Ok(raw) => match serde_json::from_str::<ServerConfig>(&raw) {
            Ok(parsed) => {
                let mut cleaned = parsed;
                cleaned.additional_args = cleaned
                    .additional_args
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                cfg = cleaned;
            }
            Err(err) => {
                eprintln!(
                    "codex-mcp-rs: failed to parse config {}: {}",
                    config_path.display(),
                    err
                );
            }
        },
        Err(err) => {
            eprintln!(
                "codex-mcp-rs: failed to read config {}: {}",
                config_path.display(),
                err
            );
        }
    }

    cfg
}

fn server_config() -> &'static ServerConfig {
    static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();
    SERVER_CONFIG.get_or_init(load_server_config)
}

/// Default extra CLI flags applied to every Codex CLI invocation.
/// Update configuration via `codex-mcp.config.json` or the
/// `CODEX_MCP_CONFIG_PATH` environment variable.
pub fn default_additional_args() -> Vec<String> {
    server_config().additional_args.clone()
}

/// Default timeout (in seconds) for Codex runs, configurable via
/// `timeout_secs` in `codex-mcp.config.json`. Values <= 0 or missing
/// fall back to 600; values above MAX_TIMEOUT_SECS are clamped.
pub fn default_timeout_secs() -> u64 {
    static CACHED_TIMEOUT: OnceLock<u64> = OnceLock::new();
    *CACHED_TIMEOUT.get_or_init(|| {
        let cfg = server_config();
        match cfg.timeout_secs {
            Some(t) if t > 0 && t <= MAX_TIMEOUT_SECS => t,
            Some(t) if t > MAX_TIMEOUT_SECS => MAX_TIMEOUT_SECS,
            _ => DEFAULT_TIMEOUT_SECS,
        }
    })
}

#[derive(Debug)]
pub struct CodexResult {
    pub success: bool,
    pub session_id: String,
    pub agent_messages: String,
    pub agent_messages_truncated: bool,
    pub all_messages: Vec<HashMap<String, Value>>,
    pub all_messages_truncated: bool,
    pub error: Option<String>,
    pub warnings: Option<String>,
}

/// Result of reading a line with length limit
#[derive(Debug)]
struct ReadLineResult {
    bytes_read: usize,
    truncated: bool,
}

/// Validation mode for enforce_required_fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationMode {
    /// Perform full validation (check session_id and agent_messages)
    Full,
    /// Skip validation (for cases with well-defined errors like timeout or truncation)
    Skip,
}

/// Read a line from an async buffered reader with a maximum length limit to prevent memory spikes
/// Returns the number of bytes read (0 on EOF) and whether the line was truncated
/// Reads in chunks and enforces max_len during reading to prevent OOM from extremely long lines
///
/// After hitting max_len, continues reading until newline to properly consume the full line.
/// This ensures the next read starts at the correct position. For subprocess stdout (our use case),
/// this is appropriate because:
/// 1. The Codex CLI always outputs newline-terminated JSON
/// 2. Process-level timeout prevents indefinite blocking
/// 3. We stop allocating memory once max_len is hit, preventing OOM
async fn read_line_with_limit<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    max_len: usize,
) -> std::io::Result<ReadLineResult> {
    let mut total_read = 0;
    let mut truncated = false;

    loop {
        // Fill the internal buffer if needed
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            break; // EOF
        }

        // Process available bytes
        for (i, &byte) in available.iter().enumerate() {
            if !truncated && buf.len() < max_len {
                buf.push(byte);
                total_read += 1;
            } else if !truncated {
                truncated = true;
            }

            if byte == b'\n' {
                reader.consume(i + 1);
                return Ok(ReadLineResult {
                    bytes_read: total_read,
                    truncated,
                });
            }
        }

        let consumed = available.len();
        reader.consume(consumed);
    }

    Ok(ReadLineResult {
        bytes_read: total_read,
        truncated,
    })
}

/// Maximum allowed size for AGENTS.md content (1MB)
const MAX_AGENTS_SIZE: usize = 1024 * 1024;

/// Read AGENTS.md from working directory if it exists
/// Returns (content, warning) where warning is set if there are issues
async fn read_agents_md(working_dir: &std::path::Path) -> (Option<String>, Option<String>) {
    let agents_path = working_dir.join("AGENTS.md");

    if !agents_path.exists() {
        return (None, None);
    }

    // Check file size first to avoid allocating huge strings
    let metadata = match tokio::fs::metadata(&agents_path).await {
        Ok(m) => m,
        Err(e) => {
            let warning = format!("Failed to read AGENTS.md metadata: {}", e);
            return (None, Some(warning));
        }
    };

    let file_size = metadata.len(); // Keep as u64 to avoid overflow

    // If file is extremely large, warn and skip to avoid OOM
    const ABSOLUTE_MAX_SIZE: u64 = 10 * 1024 * 1024; // 10MB hard limit
    if file_size > ABSOLUTE_MAX_SIZE {
        let warning = format!(
            "AGENTS.md is {} bytes, exceeding the absolute maximum of {} bytes and will be skipped.",
            file_size,
            ABSOLUTE_MAX_SIZE
        );
        return (None, Some(warning));
    }

    // Read only up to MAX_AGENTS_SIZE + a small buffer (safe to cast now since we checked against ABSOLUTE_MAX_SIZE)
    let bytes_to_read = (file_size as usize).min(MAX_AGENTS_SIZE + 4); // +4 for potential multibyte char
    let file = match tokio::fs::File::open(&agents_path).await {
        Ok(f) => f,
        Err(e) => {
            let warning = format!("Failed to open AGENTS.md: {}", e);
            return (None, Some(warning));
        }
    };

    let mut content = Vec::with_capacity(bytes_to_read);
    use tokio::io::AsyncReadExt;
    if let Err(e) = file
        .take(bytes_to_read as u64)
        .read_to_end(&mut content)
        .await
    {
        let warning = format!("Failed to read AGENTS.md: {}", e);
        return (None, Some(warning));
    }

    // Check if file is empty or whitespace-only
    if content.is_empty() {
        return (None, None);
    }

    // Check for whitespace-only content, but only for small files
    // For large files, we can't be sure what comes after our read window
    if file_size <= bytes_to_read as u64 {
        if let Ok(s) = std::str::from_utf8(&content) {
            if s.trim().is_empty() {
                return (None, None); // Whitespace-only
            }
        }
    }

    // Truncate to MAX_AGENTS_SIZE on a UTF-8 character boundary
    let (final_content, warning) = if content.len() > MAX_AGENTS_SIZE {
        // Use std::str::from_utf8 to find the longest valid UTF-8 prefix
        let mut end = MAX_AGENTS_SIZE;

        // Try to find the largest valid UTF-8 slice <= MAX_AGENTS_SIZE
        while end > 0 {
            if let Ok(valid_str) = std::str::from_utf8(&content[..end]) {
                let warning = format!(
                    "AGENTS.md is {} bytes, exceeding the {} byte limit and was truncated to {} bytes.",
                    file_size,
                    MAX_AGENTS_SIZE,
                    end
                );
                return (Some(valid_str.to_string()), Some(warning));
            }
            end -= 1;
        }

        // If we can't find any valid UTF-8, skip the file
        let warning = "AGENTS.md contains invalid UTF-8 and was skipped.".to_string();
        return (None, Some(warning));
    } else {
        match String::from_utf8(content) {
            Ok(s) => (s, None),
            Err(_) => {
                let warning = "AGENTS.md contains invalid UTF-8 and was skipped.".to_string();
                return (None, Some(warning));
            }
        }
    };

    (Some(final_content), warning)
}

/// Execute Codex CLI with the given options and return the result
/// Requires timeout to be set to prevent unbounded execution
pub async fn run(mut opts: Options) -> Result<CodexResult> {
    // Read AGENTS.md if it exists and prepend to prompt
    let (agents_content, agents_warning) = read_agents_md(&opts.working_dir).await;
    if let Some(content) = agents_content {
        opts.prompt = format!(
            "<system_prompt>\n{}\n</system_prompt>\n\n{}",
            content, opts.prompt
        );
    }

    // Ensure timeout is always set
    if opts.timeout_secs.is_none() {
        opts.timeout_secs = Some(default_timeout_secs());
    }

    let timeout_secs = opts.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let duration = std::time::Duration::from_secs(timeout_secs);

    match tokio::time::timeout(duration, run_internal(opts, agents_warning.clone())).await {
        Ok(result) => result,
        Err(_) => {
            // Timeout occurred - the child process will be killed automatically via kill_on_drop
            let result = CodexResult {
                success: false,
                session_id: String::new(),
                agent_messages: String::new(),
                agent_messages_truncated: false,
                all_messages: Vec::new(),
                all_messages_truncated: false,
                error: Some(format!(
                    "Codex execution timed out after {} seconds",
                    timeout_secs
                )),
                warnings: agents_warning,
            };
            // Skip validation since timeout error is already well-defined
            Ok(enforce_required_fields(result, ValidationMode::Skip))
        }
    }
}

/// Internal implementation of codex execution
async fn run_internal(opts: Options, agents_warning: Option<String>) -> Result<CodexResult> {
    // Allow overriding the codex binary for tests or custom setups
    let codex_bin = std::env::var("CODEX_BIN").unwrap_or_else(|_| "codex".to_string());

    // Build the base command
    let mut cmd = Command::new(codex_bin);
    cmd.args(["exec", "--cd"]);

    // Use OsStr for path handling to support non-UTF-8 paths
    cmd.arg(opts.working_dir.as_os_str());
    cmd.arg("--json");

    // Append any extra CLI flags requested by the caller, before the prompt delimiter.
    for arg in &opts.additional_args {
        cmd.arg(arg);
    }

    // Attach image files, if any, as repeated --image flags.
    for image_path in &opts.image_paths {
        cmd.arg("--image");
        cmd.arg(image_path);
    }

    // Add session resume or prompt
    if let Some(ref session_id) = opts.session_id {
        cmd.args(["resume", session_id]);
    }

    // Add the prompt at the end - Command::arg() handles proper escaping across platforms
    // Note: When resuming, the prompt serves as a continuation message in the existing session
    cmd.args(["--", &opts.prompt]);

    // Configure process
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true); // Ensure child is killed if this future is dropped (e.g., on timeout)

    // Spawn the process
    let mut child = cmd.spawn().context("Failed to spawn codex command")?;

    // Read stdout
    let stdout = child.stdout.take().context("Failed to get stdout")?;
    let stderr = child.stderr.take().context("Failed to get stderr")?;

    let mut result = CodexResult {
        success: true,
        session_id: String::new(),
        agent_messages: String::new(),
        agent_messages_truncated: false,
        all_messages: Vec::new(),
        all_messages_truncated: false,
        error: None,
        warnings: None,
    };

    // Spawn a task to drain stderr and capture diagnostics with better error handling
    const MAX_STDERR_SIZE: usize = 1024 * 1024; // 1MB limit for stderr
    const MAX_LINE_LENGTH: usize = 1024 * 1024; // 1MB per line to prevent memory spikes
    const MAX_AGENT_MESSAGES_SIZE: usize = 10 * 1024 * 1024; // 10MB limit for agent messages
    const MAX_ALL_MESSAGES_SIZE: usize = 50 * 1024 * 1024; // 50MB limit for all messages combined
    let stderr_handle = tokio::spawn(async move {
        let mut stderr_output = String::new();
        let mut stderr_reader = BufReader::new(stderr);
        let mut truncated = false;
        let mut line_buf = Vec::new();

        loop {
            line_buf.clear();
            match read_line_with_limit(&mut stderr_reader, &mut line_buf, MAX_LINE_LENGTH).await {
                Ok(read_result) => {
                    if read_result.bytes_read == 0 {
                        break; // EOF
                    }
                    // Convert to string, handling invalid UTF-8
                    let line = String::from_utf8_lossy(&line_buf);
                    let line = line.trim_end_matches('\n').trim_end_matches('\r');

                    // Check if adding this line would exceed the limit
                    let new_size = stderr_output.len() + line.len() + 1; // +1 for newline
                    if new_size > MAX_STDERR_SIZE {
                        if !truncated {
                            if !stderr_output.is_empty() {
                                stderr_output.push('\n');
                            }
                            stderr_output.push_str("[... stderr truncated due to size limit ...]");
                            truncated = true;
                        }
                        // Continue draining to prevent blocking the child process
                    } else if !truncated {
                        if !stderr_output.is_empty() {
                            stderr_output.push('\n');
                        }
                        stderr_output.push_str(line.as_ref());
                    }
                }
                Err(e) => {
                    // Log the read error but continue - this preserves diagnostic info
                    eprintln!("Warning: Failed to read from stderr: {}", e);
                    break;
                }
            }
        }

        stderr_output
    });

    // Read stdout line by line with length limit
    let mut reader = BufReader::new(stdout);
    let mut parse_error_seen = false;
    let mut line_buf = Vec::new();
    let mut all_messages_size: usize = 0;

    loop {
        line_buf.clear();
        match read_line_with_limit(&mut reader, &mut line_buf, MAX_LINE_LENGTH).await {
            Ok(read_result) => {
                if read_result.bytes_read == 0 {
                    break; // EOF
                }

                // Check for line truncation - short-circuit to error instead of attempting parse
                if read_result.truncated {
                    let error_msg = format!(
                        "Output line exceeded {} byte limit and was truncated, cannot parse JSON.",
                        MAX_LINE_LENGTH
                    );
                    result.success = false;
                    result.error = Some(error_msg);
                    if !parse_error_seen {
                        parse_error_seen = true;
                        // Stop the child so it cannot block on a full pipe, then keep draining
                        let _ = child.start_kill();
                    }
                    continue;
                }

                // Convert to string
                let line = String::from_utf8_lossy(&line_buf);
                let line = line.trim_end_matches('\n').trim_end_matches('\r');

                if line.is_empty() {
                    continue;
                }

                // After a parse error, keep draining stdout to avoid blocking the child process
                if parse_error_seen {
                    continue;
                }

                // Parse JSON line
                let line_data: Value = match serde_json::from_str(line) {
                    Ok(data) => data,
                    Err(e) => {
                        record_parse_error(&mut result, &e, line);
                        if !parse_error_seen {
                            parse_error_seen = true;
                            // Stop the child so it cannot block on a full pipe, then keep draining
                            let _ = child.start_kill();
                        }
                        continue;
                    }
                };

                // Collect all messages with bounds checking
                if let Ok(map) =
                    serde_json::from_value::<HashMap<String, Value>>(line_data.clone())
                {
                    // Estimate size of this message (JSON serialized size)
                    let message_size =
                        serde_json::to_string(&map).map(|s| s.len()).unwrap_or(0);

                    // Check if adding this message would exceed byte limit
                    if all_messages_size + message_size <= MAX_ALL_MESSAGES_SIZE {
                        all_messages_size += message_size;
                        result.all_messages.push(map);
                    } else if !result.all_messages_truncated {
                        result.all_messages_truncated = true;
                    }
                }

                // Extract thread_id
                if let Some(thread_id) = line_data.get("thread_id").and_then(|v| v.as_str()) {
                    if !thread_id.is_empty() {
                        result.session_id = thread_id.to_string();
                    }
                }

                // Extract agent messages with size limits
                if let Some(item) = line_data.get("item").and_then(|v| v.as_object()) {
                    if let Some(item_type) = item.get("type").and_then(|v| v.as_str()) {
                        if item_type == "agent_message" {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                // Check if adding this text would exceed the limit
                                let new_size = result.agent_messages.len() + text.len();
                                if new_size > MAX_AGENT_MESSAGES_SIZE {
                                    if !result.agent_messages_truncated {
                                        result.agent_messages.push_str(
                                    "\n[... Agent messages truncated due to size limit ...]",
                                );
                                        result.agent_messages_truncated = true;
                                    }
                                } else if !result.agent_messages_truncated {
                                    // Add a newline separator between multiple agent messages for better parsing
                                    if !result.agent_messages.is_empty() && !text.is_empty() {
                                        result.agent_messages.push('\n');
                                    }
                                    result.agent_messages.push_str(text);
                                }
                            }
                        }
                    }
                }

                // Check for errors
                if let Some(line_type) = line_data.get("type").and_then(|v| v.as_str()) {
                    if line_type.contains("fail") || line_type.contains("error") {
                        // Always mark as failure when we encounter error/fail events
                        result.success = false;
                        if let Some(error_obj) = line_data.get("error").and_then(|v| v.as_object())
                        {
                            if let Some(msg) = error_obj.get("message").and_then(|v| v.as_str()) {
                                result.error = Some(format!("codex error: {}", msg));
                            }
                        } else if let Some(msg) = line_data.get("message").and_then(|v| v.as_str())
                        {
                            result.error = Some(format!("codex error: {}", msg));
                        }
                    }
                }
            }
            Err(e) => {
                // Create a simple IO error for the parse error
                let io_error = std::io::Error::from(e.kind());
                record_parse_error(&mut result, &serde_json::Error::io(io_error), "");
                break;
            }
        }
    }

    // Wait for process to finish
    let status = child
        .wait()
        .await
        .context("Failed to wait for codex command")?;

    // Collect stderr output with better error handling
    let stderr_output = match stderr_handle.await {
        Ok(output) => output,
        Err(e) => {
            // Log the join error but continue processing
            eprintln!("Warning: Failed to join stderr task: {}", e);
            String::new()
        }
    };

    if !status.success() {
        result.success = false;
        let error_msg = if let Some(ref err) = result.error {
            err.clone()
        } else {
            format!("codex command failed with exit code: {:?}", status.code())
        };

        // Append stderr diagnostics if available
        if !stderr_output.is_empty() {
            result.error = Some(format!("{}\nStderr: {}", error_msg, stderr_output));
        } else {
            result.error = Some(error_msg);
        }
    } else if !stderr_output.is_empty() {
        // On success, put stderr in warnings field instead of error
        result.warnings = Some(stderr_output);
    }

    // Prepend AGENTS.md warning if present
    if let Some(agents_warn) = agents_warning {
        result.warnings = match result.warnings.take() {
            Some(existing) => Some(format!("{}\n{}", agents_warn, existing)),
            None => Some(agents_warn),
        };
    }

    Ok(enforce_required_fields(result, ValidationMode::Full))
}

fn record_parse_error(result: &mut CodexResult, error: &serde_json::Error, line: &str) {
    let parse_msg = format!("JSON parse error: {}. Line: {}", error, line);
    result.success = false;
    result.error = match result.error.take() {
        Some(existing) if !existing.is_empty() => Some(format!("{existing}\n{parse_msg}")),
        _ => Some(parse_msg),
    };
}

fn push_warning(existing: Option<String>, warning: &str) -> Option<String> {
    match existing {
        Some(mut current) => {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(warning);
            Some(current)
        }
        None => Some(warning.to_string()),
    }
}

fn enforce_required_fields(mut result: CodexResult, mode: ValidationMode) -> CodexResult {
    // Skip validation for cases where we already have a well-defined error (e.g., timeout, truncation)
    if mode == ValidationMode::Skip {
        return result;
    }

    // Skip session_id check if there's already an error (e.g., truncation, I/O error)
    // to avoid masking the original error
    if result.session_id.is_empty() && result.error.is_none() {
        result.success = false;
        result.error = Some("Failed to get SESSION_ID from the codex session.".to_string());
    }

    if result.agent_messages.is_empty() {
        // Preserve success but surface as a warning so callers can decide how to handle it
        let warning_msg = "No agent_messages returned; check Codex CLI output or enable richer logging if needed.";
        result.warnings = push_warning(result.warnings.take(), warning_msg);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_creation() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            working_dir: PathBuf::from("/tmp"),
            session_id: None,
            additional_args: Vec::new(),
            image_paths: Vec::new(),
            timeout_secs: None,
        };

        assert_eq!(opts.prompt, "test prompt");
        assert_eq!(opts.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_options_with_session() {
        let opts = Options {
            prompt: "resume task".to_string(),
            working_dir: PathBuf::from("/tmp"),
            session_id: Some("test-session-123".to_string()),
            additional_args: vec!["--json".to_string()],
            image_paths: vec![PathBuf::from("image.png")],
            timeout_secs: Some(600),
        };

        assert_eq!(opts.session_id, Some("test-session-123".to_string()));
        assert_eq!(opts.timeout_secs, Some(600));
        assert_eq!(opts.image_paths.len(), 1);
    }

    #[test]
    fn test_record_parse_error_sets_failure_and_appends_message() {
        let mut result = CodexResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: "ok".to_string(),
            agent_messages_truncated: false,
            all_messages: Vec::new(),
            all_messages_truncated: false,
            error: Some("existing".to_string()),
            warnings: None,
        };

        let err = serde_json::from_str::<Value>("not-json").unwrap_err();
        record_parse_error(&mut result, &err, "not-json");

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("JSON parse error"));
        assert!(result.error.as_ref().unwrap().contains("existing"));
    }

    #[test]
    fn test_enforce_required_fields_warns_on_missing_agent_messages() {
        let result = CodexResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: String::new(),
            agent_messages_truncated: false,
            all_messages: vec![HashMap::new()],
            all_messages_truncated: false,
            error: None,
            warnings: None,
        };

        let updated = enforce_required_fields(result, ValidationMode::Full);

        assert!(updated.success);
        assert!(updated
            .warnings
            .as_ref()
            .unwrap()
            .contains("No agent_messages"));
    }

    #[test]
    fn test_enforce_required_fields_requires_session_id() {
        let result = CodexResult {
            success: true,
            session_id: String::new(),
            agent_messages: "msg".to_string(),
            agent_messages_truncated: false,
            all_messages: Vec::new(),
            all_messages_truncated: false,
            error: None,
            warnings: None,
        };

        let updated = enforce_required_fields(result, ValidationMode::Full);

        assert!(!updated.success);
        assert!(updated
            .error
            .as_ref()
            .unwrap()
            .contains("Failed to get SESSION_ID"));
    }

    #[test]
    fn test_push_warning_appends_with_newline() {
        let combined = push_warning(Some("first".to_string()), "second").unwrap();
        assert!(combined.contains("first"));
        assert!(combined.contains("second"));
        assert!(combined.contains('\n'));
    }

    #[test]
    fn test_enforce_required_fields_skips_validation_when_requested() {
        // Simulate a timeout result with empty session_id and agent_messages
        let result = CodexResult {
            success: false,
            session_id: String::new(),
            agent_messages: String::new(),
            agent_messages_truncated: false,
            all_messages: Vec::new(),
            all_messages_truncated: false,
            error: Some("Codex execution timed out after 10 seconds".to_string()),
            warnings: None,
        };

        let updated = enforce_required_fields(result, ValidationMode::Skip);

        // When skipping validation, the original error should be preserved
        assert!(!updated.success);
        assert_eq!(
            updated.error.unwrap(),
            "Codex execution timed out after 10 seconds"
        );
        // Should NOT have session_id error appended
        // Should NOT have agent_messages warning
        assert!(updated.warnings.is_none());
        assert!(updated.session_id.is_empty());
    }

    #[test]
    fn test_enforce_required_fields_skips_session_id_when_error_exists() {
        // Simulate a truncation error with empty session_id
        let result = CodexResult {
            success: false,
            session_id: String::new(),
            agent_messages: String::new(),
            agent_messages_truncated: false,
            all_messages: Vec::new(),
            all_messages_truncated: false,
            error: Some(
                "Output line exceeded 1048576 byte limit and was truncated, cannot parse JSON."
                    .to_string(),
            ),
            warnings: None,
        };

        let updated = enforce_required_fields(result, ValidationMode::Full);

        // When there's already an error, session_id check should be skipped
        assert!(!updated.success);
        let error = updated.error.unwrap();
        assert!(error.contains("truncated"));
        assert!(
            !error.contains("SESSION_ID"),
            "Should not add session_id error when truncation error exists"
        );
        // Agent_messages warning should still be added since it's a separate concern
        assert!(updated.warnings.is_some());
        assert!(updated.warnings.unwrap().contains("No agent_messages"));
    }

    #[tokio::test]
    async fn test_read_agents_md_returns_none_when_file_not_exists() {
        let temp_dir = tempfile::tempdir().unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_none());
        assert!(warning.is_none());
    }

    #[tokio::test]
    async fn test_read_agents_md_returns_content_when_file_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        let test_content = "# System Prompt\nYou are a helpful assistant.";
        tokio::fs::write(&agents_path, test_content).await.unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_some());
        assert_eq!(content.unwrap(), test_content);
        assert!(warning.is_none());
    }

    #[tokio::test]
    async fn test_read_agents_md_returns_none_when_file_is_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        tokio::fs::write(&agents_path, "   \n\t  \n").await.unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_none());
        assert!(warning.is_none());
    }

    #[tokio::test]
    async fn test_read_agents_md_truncates_large_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        // Create a file larger than MAX_AGENTS_SIZE
        let large_content = "a".repeat(MAX_AGENTS_SIZE + 1000);
        tokio::fs::write(&agents_path, &large_content)
            .await
            .unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_some());
        assert!(warning.is_some());

        let content_str = content.unwrap();
        assert!(content_str.len() <= MAX_AGENTS_SIZE);
        assert!(warning.unwrap().contains("truncated"));
    }

    #[tokio::test]
    async fn test_read_agents_md_handles_unreadable_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        // Create a file then make it unreadable (Unix-specific)
        tokio::fs::write(&agents_path, "test content")
            .await
            .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&agents_path).unwrap().permissions();
            perms.set_mode(0o000); // No permissions
            std::fs::set_permissions(&agents_path, perms).unwrap();

            let (content, warning) = read_agents_md(temp_dir.path()).await;
            assert!(content.is_none());
            assert!(warning.is_some());
            let warn_msg = warning.unwrap();
            assert!(warn_msg.contains("Failed to open") || warn_msg.contains("Failed to read"));

            // Restore permissions for cleanup
            let mut perms = std::fs::metadata(&agents_path).unwrap().permissions();
            perms.set_mode(0o644);
            std::fs::set_permissions(&agents_path, perms).unwrap();
        }

        #[cfg(not(unix))]
        {
            // On Windows, just verify the function doesn't panic
            let (content, _warning) = read_agents_md(temp_dir.path()).await;
            assert!(content.is_some());
        }
    }

    #[tokio::test]
    async fn test_read_agents_md_handles_invalid_utf8() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        // Write invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        tokio::fs::write(&agents_path, &invalid_utf8).await.unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_none());
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("invalid UTF-8"));
    }

    #[tokio::test]
    async fn test_read_agents_md_truncates_multibyte_chars_correctly() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_path = temp_dir.path().join("AGENTS.md");

        // Create content with multibyte UTF-8 characters that would be cut mid-character
        let base = "你好世界"; // Chinese characters (3 bytes each in UTF-8)
        let mut large_content = base.repeat(MAX_AGENTS_SIZE / base.len() + 100);
        large_content.push_str("final");

        tokio::fs::write(&agents_path, &large_content)
            .await
            .unwrap();

        let (content, warning) = read_agents_md(temp_dir.path()).await;
        assert!(content.is_some());
        assert!(warning.is_some());

        let content_str = content.unwrap();
        // Verify it's valid UTF-8 (no panic)
        assert!(content_str.len() <= MAX_AGENTS_SIZE);
        // Verify it's actually valid UTF-8 by checking we can iterate chars
        assert!(content_str.chars().count() > 0);
    }
}
