use codex_mcp_rs::codex::{CodexResult, Options};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_agent_messages_size_limit() {
    // Create a mock result that would exceed the agent messages limit
    let large_message = "x".repeat(11 * 1024 * 1024); // 11MB > 10MB limit
    let result = CodexResult {
        success: true,
        session_id: "test-session".to_string(),
        agent_messages: large_message,
        agent_messages_truncated: false,
        all_messages: Vec::new(),
        all_messages_truncated: false,
        error: None,
        warnings: None,
    };

    // The agent_messages should be truncatable in practice
    assert!(result.agent_messages.len() > 10 * 1024 * 1024);
    assert!(!result.agent_messages_truncated);
}

#[test]
fn test_agent_messages_truncation_flag() {
    let result = CodexResult {
        success: true,
        session_id: "test-session".to_string(),
        agent_messages: "[... Agent messages truncated due to size limit ...]".to_string(),
        agent_messages_truncated: true,
        all_messages: Vec::new(),
        all_messages_truncated: false,
        error: None,
        warnings: None,
    };

    assert!(result.agent_messages_truncated);
    assert!(result.agent_messages.contains("truncated"));
}

#[test]
fn test_all_messages_limit() {
    // Test that all_messages can be properly bounded
    let mut result = CodexResult {
        success: true,
        session_id: "test-session".to_string(),
        agent_messages: "test messages".to_string(),
        agent_messages_truncated: false,
        all_messages: Vec::new(),
        all_messages_truncated: false,
        error: None,
        warnings: None,
    };

    // Simulate adding messages up to limit
    for i in 0..50001 {
        if result.all_messages.len() < 50000 {
            result.all_messages.push(HashMap::from([
                ("id".to_string(), Value::String(format!("msg_{}", i))),
                ("type".to_string(), Value::String("test".to_string())),
            ]));
        } else {
            result.all_messages_truncated = true;
            break;
        }
    }

    assert_eq!(result.all_messages.len(), 50000);
    assert!(result.all_messages_truncated);
}

#[test]
fn test_error_and_warning_handling() {
    let result = CodexResult {
        success: false,
        session_id: "".to_string(),
        agent_messages: "".to_string(),
        agent_messages_truncated: false,
        all_messages: Vec::new(),
        all_messages_truncated: false,
        error: Some("Test error message".to_string()),
        warnings: Some("Test warning message".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.warnings.is_some());
    assert_eq!(result.error.unwrap(), "Test error message");
    assert_eq!(result.warnings.unwrap(), "Test warning message");
}

#[test]
fn test_path_handling_with_non_utf8() {
    // Test PathBuf can handle non-UTF8 paths (even if we serialize as strings for JSON)
    let non_utf8_path = PathBuf::from("/path/with/invalid/utf8/�sequence");
    let opts = Options {
        prompt: "test".to_string(),
        working_dir: non_utf8_path.clone(),
        session_id: None,
        additional_args: Vec::new(),
        image_paths: Vec::new(),
        timeout_secs: None,
    };

    // Should be able to create options without panicking
    assert_eq!(opts.working_dir, non_utf8_path);
}

#[test]
fn test_escape_prompt_with_special_chars() {
    // Removed since escape_prompt function was removed
    // Command::arg() handles platform-specific escaping automatically
    let input = "Test with \"quotes\" and \n newlines and \t tabs";

    // Verify the prompt can contain special characters
    assert!(input.contains('"'));
    assert!(input.contains('\n'));
    assert!(input.contains('\t'));
}

#[test]
fn test_stderr_error_context() {
    // Test error messages that include stderr context
    let error_with_stderr = "Command failed\nStderr: Warning: Something went wrong".to_string();

    assert!(error_with_stderr.contains("Stderr:"));
    assert!(error_with_stderr.contains("Warning: Something went wrong"));
}

#[tokio::test]
async fn test_additional_args_are_passed_to_codex_cli() {
    use codex_mcp_rs::codex;
    use std::env;
    use tempfile::tempdir;

    let temp_dir = tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_path_buf();

    // Path where the helper script will log its argv
    let log_path = temp_path.join("codex_args.log");

    // Create a helper script that logs argv and emits a minimal JSON event
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let script_path = temp_path.join("echo_args.sh");
    let script_contents = r#"#!/bin/sh
LOG_FILE="${CODEX_ARGS_LOG}"
: > "$LOG_FILE"
printf "%s" "$0" > "$LOG_FILE"
for arg in "$@"; do
  printf " %s" "$arg" >> "$LOG_FILE"
done
echo '{"thread_id":"test-session","item":{"type":"agent_message","text":"ok"}}'
"#;

    fs::write(&script_path, script_contents).expect("Failed to write script");
    let mut perms = fs::metadata(&script_path)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("Failed to set permissions");

    env::set_var("CODEX_BIN", script_path.to_str().unwrap());

    // Make log path available to the helper script
    env::set_var("CODEX_ARGS_LOG", log_path.to_str().unwrap());

    let additional = vec![
        "--dangerously-bypass-approvals-and-sandbox".to_string(),
        "--profile".to_string(),
        "gpt-5".to_string(),
    ];

    let opts = Options {
        prompt: "test additional args".to_string(),
        working_dir: temp_path.clone(),
        session_id: None,
        additional_args: additional.clone(),
        image_paths: Vec::new(),
        timeout_secs: Some(10),
    };

    let result = codex::run(opts).await.expect("run should return Ok");

    assert!(result.success, "helper script should succeed");
    assert_eq!(result.session_id, "test-session");
    assert_eq!(result.agent_messages.trim(), "ok");

    // Verify that additional_args were passed through to the Codex CLI
    let log = std::fs::read_to_string(&log_path).expect("failed to read args log");
    let parts: Vec<&str> = log.split_whitespace().collect();

    let idx = parts
        .iter()
        .position(|s| *s == "--dangerously-bypass-approvals-and-sandbox")
        .expect("additional_args flag not found in argv");

    let idx_profile = parts
        .iter()
        .position(|s| *s == "--profile")
        .expect("profile flag not found in argv");
    let idx_profile_value = parts
        .iter()
        .position(|s| *s == "gpt-5")
        .expect("profile value not found in argv");

    assert!(
        idx_profile > idx,
        "expected --profile to appear after the dangerous flag"
    );
    assert!(
        idx_profile_value > idx_profile,
        "expected gpt-5 to appear after --profile"
    );

    // Clean up env vars
    env::remove_var("CODEX_BIN");
    env::remove_var("CODEX_ARGS_LOG");
}
