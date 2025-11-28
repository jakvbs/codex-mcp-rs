use anyhow::{Context, Result};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Sandbox policy for model-generated commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxPolicy {
    /// Read-only access (safe for exploration)
    #[default]
    ReadOnly,
    /// Write access within workspace (modify files)
    WorkspaceWrite,
    /// Full system access (dangerous)
    DangerFullAccess,
}

impl SandboxPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            SandboxPolicy::ReadOnly => "read-only",
            SandboxPolicy::WorkspaceWrite => "workspace-write",
            SandboxPolicy::DangerFullAccess => "danger-full-access",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub sandbox: SandboxPolicy,
    pub session_id: Option<String>,
    pub skip_git_repo_check: bool,
    pub return_all_messages: bool,
    pub return_all_messages_limit: Option<usize>,
    pub image_paths: Vec<PathBuf>,
    pub model: Option<String>,
    pub yolo: bool,
    pub profile: Option<String>,
    /// Timeout in seconds for the codex execution. None means no timeout.
    pub timeout_secs: Option<u64>,
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

/// Execute Codex CLI with the given options and return the result
pub async fn run(opts: Options) -> Result<CodexResult> {
    // Apply timeout if specified
    if let Some(timeout_secs) = opts.timeout_secs {
        let duration = std::time::Duration::from_secs(timeout_secs);
        match tokio::time::timeout(duration, run_internal(opts)).await {
            Ok(result) => result,
            Err(_) => {
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
                    warnings: None,
                };
                Ok(result)
            }
        }
    } else {
        run_internal(opts).await
    }
}

/// Internal implementation of codex execution
async fn run_internal(opts: Options) -> Result<CodexResult> {
    // Allow overriding the codex binary for tests or custom setups
    let codex_bin = std::env::var("CODEX_BIN").unwrap_or_else(|_| "codex".to_string());

    // Build the base command
    let mut cmd = Command::new(codex_bin);
    cmd.args(["exec", "--sandbox", opts.sandbox.as_str(), "--cd"]);

    // Use OsStr for path handling to support non-UTF-8 paths
    cmd.arg(opts.working_dir.as_os_str());
    cmd.arg("--json");

    // Add optional flags - use repeated --image args for paths with special chars
    for image_path in &opts.image_paths {
        cmd.arg("--image");
        cmd.arg(image_path);
    }
    if let Some(ref model) = opts.model {
        cmd.args(["--model", model]);
    }
    if let Some(ref profile) = opts.profile {
        cmd.args(["--profile", profile]);
    }
    if opts.yolo {
        cmd.arg("--yolo");
    }
    if opts.skip_git_repo_check {
        cmd.arg("--skip-git-repo-check");
    }
    if opts.return_all_messages {
        cmd.arg("--return-all-messages");
        if let Some(limit) = opts.return_all_messages_limit {
            cmd.args(["--return-all-messages-limit", &limit.to_string()]);
        }
    }

    // Add session resume or prompt
    if let Some(ref session_id) = opts.session_id {
        cmd.args(["resume", session_id]);
    }

    // Add the prompt at the end - Command::arg() handles proper escaping across platforms
    cmd.args(["--", &opts.prompt]);

    // Configure process
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

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

    // Set default limit if return_all_messages is enabled but no limit specified
    // Cap at 50000 to prevent excessive memory usage
    const MAX_MESSAGE_LIMIT: usize = 50000;
    const DEFAULT_MESSAGE_LIMIT: usize = 10000;
    const MAX_AGENT_MESSAGES_SIZE: usize = 10 * 1024 * 1024; // 10MB limit for agent messages
    let message_limit = if let Some(limit) = opts.return_all_messages_limit {
        limit.min(MAX_MESSAGE_LIMIT)
    } else {
        DEFAULT_MESSAGE_LIMIT
    };

    // Spawn a task to drain stderr and capture diagnostics with better error handling
    let stderr_handle = tokio::spawn(async move {
        let mut stderr_output = String::new();
        let mut stderr_reader = BufReader::new(stderr).lines();

        loop {
            match stderr_reader.next_line().await {
                Ok(Some(line)) => {
                    if !stderr_output.is_empty() {
                        stderr_output.push('\n');
                    }
                    stderr_output.push_str(&line);
                }
                Ok(None) => break, // EOF reached
                Err(e) => {
                    // Log the read error but continue - this preserves diagnostic info
                    eprintln!("Warning: Failed to read from stderr: {}", e);
                    break;
                }
            }
        }

        stderr_output
    });

    // Read stdout line by line
    let mut reader = BufReader::new(stdout).lines();
    let mut parse_error_seen = false;
    while let Some(line) = reader.next_line().await? {
        if line.is_empty() {
            continue;
        }

        // After a parse error, keep draining stdout to avoid blocking the child process
        if parse_error_seen {
            continue;
        }

        // Parse JSON line
        let line_data: Value = match serde_json::from_str(&line) {
            Ok(data) => data,
            Err(e) => {
                record_parse_error(&mut result, &e, &line);
                if !parse_error_seen {
                    parse_error_seen = true;
                    // Stop the child so it cannot block on a full pipe, then keep draining
                    let _ = child.start_kill();
                }
                continue;
            }
        };

        // Collect all messages if requested (with bounds checking)
        if opts.return_all_messages {
            if result.all_messages.len() < message_limit {
                if let Ok(map) = serde_json::from_value::<HashMap<String, Value>>(line_data.clone())
                {
                    result.all_messages.push(map);
                }
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
                                    "[... Agent messages truncated due to size limit ...]",
                                );
                                result.agent_messages_truncated = true;
                            }
                        } else if !result.agent_messages_truncated {
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
                if let Some(error_obj) = line_data.get("error").and_then(|v| v.as_object()) {
                    if let Some(msg) = error_obj.get("message").and_then(|v| v.as_str()) {
                        result.error = Some(format!("codex error: {}", msg));
                    }
                } else if let Some(msg) = line_data.get("message").and_then(|v| v.as_str()) {
                    result.error = Some(format!("codex error: {}", msg));
                }
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

    Ok(enforce_required_fields(result))
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

fn enforce_required_fields(mut result: CodexResult) -> CodexResult {
    if result.session_id.is_empty() {
        result.success = false;
        let prev_error = result.error.take().unwrap_or_default();
        result.error = Some(format!(
            "Failed to get SESSION_ID from the codex session.\n\n{}",
            prev_error
        ));
    }

    if result.agent_messages.is_empty() {
        // Preserve success but surface as a warning so callers can decide how to handle it
        let warning_msg = "No agent_messages returned; enable return_all_messages or check codex output for details.";
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
            sandbox: SandboxPolicy::ReadOnly,
            session_id: None,
            skip_git_repo_check: true,
            return_all_messages: false,
            return_all_messages_limit: None,
            image_paths: vec![],
            model: None,
            yolo: false,
            profile: None,
            timeout_secs: None,
        };

        assert_eq!(opts.prompt, "test prompt");
        assert_eq!(opts.working_dir, PathBuf::from("/tmp"));
        assert_eq!(opts.sandbox, SandboxPolicy::ReadOnly);
        assert!(opts.skip_git_repo_check);
    }

    #[test]
    fn test_options_with_session() {
        let opts = Options {
            prompt: "resume task".to_string(),
            working_dir: PathBuf::from("/tmp"),
            sandbox: SandboxPolicy::WorkspaceWrite,
            session_id: Some("test-session-123".to_string()),
            skip_git_repo_check: false,
            return_all_messages: true,
            return_all_messages_limit: Some(5000),
            image_paths: vec![PathBuf::from("/path/to/image.png")],
            model: Some("claude-3-opus".to_string()),
            yolo: false,
            profile: Some("default".to_string()),
            timeout_secs: Some(600),
        };

        assert_eq!(opts.session_id, Some("test-session-123".to_string()));
        assert_eq!(opts.model, Some("claude-3-opus".to_string()));
        assert!(opts.return_all_messages);
        assert!(!opts.skip_git_repo_check);
        assert_eq!(opts.sandbox, SandboxPolicy::WorkspaceWrite);
        assert_eq!(opts.timeout_secs, Some(600));
    }

    #[test]
    fn test_sandbox_policy_as_str() {
        assert_eq!(SandboxPolicy::ReadOnly.as_str(), "read-only");
        assert_eq!(SandboxPolicy::WorkspaceWrite.as_str(), "workspace-write");
        assert_eq!(
            SandboxPolicy::DangerFullAccess.as_str(),
            "danger-full-access"
        );
    }

    #[test]
    fn test_sandbox_policy_default() {
        assert_eq!(SandboxPolicy::default(), SandboxPolicy::ReadOnly);
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

        let updated = enforce_required_fields(result);

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

        let updated = enforce_required_fields(result);

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
}
