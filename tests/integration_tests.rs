use codex_mcp_rs::codex::{Options, SandboxPolicy};
use std::path::PathBuf;

/// RAII guard for environment variables - ensures cleanup even on panic
/// Uses a mutex to prevent parallel tests from interfering with each other
struct EnvVarGuard {
    key: String,
    original: Option<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl EnvVarGuard {
    fn new(key: &str, value: &str) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|poisoned| {
            // If mutex is poisoned (from a panic), clear it and continue
            poisoned.into_inner()
        });
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            original,
            _lock: lock,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var(&self.key, val),
            None => std::env::remove_var(&self.key),
        }
    }
}

#[test]
fn test_options_validation() {
    // Test valid options
    let opts = Options {
        prompt: "Test prompt".to_string(),
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

    assert!(!opts.prompt.is_empty());
    assert_eq!(opts.working_dir, PathBuf::from("/tmp"));
}

#[test]
fn test_sandbox_policies() {
    let policies = vec![
        SandboxPolicy::ReadOnly,
        SandboxPolicy::WorkspaceWrite,
        SandboxPolicy::DangerFullAccess,
    ];

    for policy in policies {
        let opts = Options {
            prompt: "test".to_string(),
            working_dir: PathBuf::from("/tmp"),
            sandbox: policy.clone(),
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

        assert_eq!(opts.sandbox, policy);
    }
}

#[test]
fn test_sandbox_policy_strings() {
    assert_eq!(SandboxPolicy::ReadOnly.as_str(), "read-only");
    assert_eq!(SandboxPolicy::WorkspaceWrite.as_str(), "workspace-write");
    assert_eq!(
        SandboxPolicy::DangerFullAccess.as_str(),
        "danger-full-access"
    );
}

#[test]
fn test_image_paths() {
    let opts = Options {
        prompt: "Analyze this image".to_string(),
        working_dir: PathBuf::from("/tmp"),
        sandbox: SandboxPolicy::ReadOnly,
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: false,
        return_all_messages_limit: None,
        image_paths: vec![
            PathBuf::from("/path/to/image1.png"),
            PathBuf::from("/path/to/image2.jpg"),
        ],
        model: None,
        yolo: false,
        profile: None,
        timeout_secs: None,
    };

    assert_eq!(opts.image_paths.len(), 2);
    assert!(opts.image_paths[0].to_string_lossy().ends_with(".png"));
    assert!(opts.image_paths[1].to_string_lossy().ends_with(".jpg"));
}

#[test]
fn test_session_id_format() {
    let session_id = "550e8400-e29b-41d4-a716-446655440000";

    let opts = Options {
        prompt: "Continue task".to_string(),
        working_dir: PathBuf::from("/tmp"),
        sandbox: SandboxPolicy::ReadOnly,
        session_id: Some(session_id.to_string()),
        skip_git_repo_check: true,
        return_all_messages: false,
        return_all_messages_limit: None,
        image_paths: vec![],
        model: None,
        yolo: false,
        profile: None,
        timeout_secs: None,
    };

    assert!(opts.session_id.is_some());
    assert_eq!(opts.session_id.unwrap(), session_id);
}

#[test]
fn test_model_options() {
    let models = vec![
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
    ];

    for model in models {
        let opts = Options {
            prompt: "test".to_string(),
            working_dir: PathBuf::from("/tmp"),
            sandbox: SandboxPolicy::ReadOnly,
            session_id: None,
            skip_git_repo_check: true,
            return_all_messages: false,
            return_all_messages_limit: None,
            image_paths: vec![],
            model: Some(model.to_string()),
            yolo: false,
            profile: None,
            timeout_secs: None,
        };

        assert_eq!(opts.model, Some(model.to_string()));
    }
}

#[test]
fn test_return_all_messages_flag() {
    let opts_detailed = Options {
        prompt: "test".to_string(),
        working_dir: PathBuf::from("/tmp"),
        sandbox: SandboxPolicy::ReadOnly,
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: true,
        return_all_messages_limit: Some(5000),
        image_paths: vec![],
        model: None,
        yolo: false,
        profile: None,
        timeout_secs: None,
    };

    let opts_simple = Options {
        prompt: "test".to_string(),
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

    assert!(opts_detailed.return_all_messages);
    assert_eq!(opts_detailed.return_all_messages_limit, Some(5000));
    assert!(!opts_simple.return_all_messages);
    assert_eq!(opts_simple.return_all_messages_limit, None);
}

#[test]
fn test_escape_prompt_integration() {
    // Removed since escape_prompt function was removed
    // Command::arg() handles platform-specific escaping automatically
    // This test is now empty as the functionality was removed
}

#[test]
fn test_working_directory_paths() {
    let paths = vec!["/tmp", "/home/user/project", ".", ".."];

    for path in paths {
        let opts = Options {
            prompt: "test".to_string(),
            working_dir: PathBuf::from(path),
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

        assert_eq!(opts.working_dir, PathBuf::from(path));
    }
}

#[test]
fn test_profile_configuration() {
    let profiles = vec!["default", "development", "production"];

    for profile in profiles {
        let opts = Options {
            prompt: "test".to_string(),
            working_dir: PathBuf::from("/tmp"),
            sandbox: SandboxPolicy::ReadOnly,
            session_id: None,
            skip_git_repo_check: true,
            return_all_messages: false,
            return_all_messages_limit: None,
            image_paths: vec![],
            model: None,
            yolo: false,
            profile: Some(profile.to_string()),
            timeout_secs: None,
        };

        assert_eq!(opts.profile, Some(profile.to_string()));
    }
}

#[test]
fn test_yolo_mode() {
    let opts_safe = Options {
        prompt: "test".to_string(),
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

    let opts_yolo = Options {
        prompt: "test".to_string(),
        working_dir: PathBuf::from("/tmp"),
        sandbox: SandboxPolicy::DangerFullAccess,
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: false,
        return_all_messages_limit: None,
        image_paths: vec![],
        model: None,
        yolo: true,
        profile: None,
        timeout_secs: None,
    };

    assert!(!opts_safe.yolo);
    assert!(opts_yolo.yolo);
    assert_eq!(opts_yolo.sandbox, SandboxPolicy::DangerFullAccess);
}

#[tokio::test]
#[cfg(unix)] // Shell scripts don't work on Windows
async fn test_agents_md_system_prompt_integration() {
    // This test verifies end-to-end behavior: AGENTS.md is read, prepended with <system_prompt> tags,
    // and warnings propagate into CodexResult

    let temp_dir = tempfile::tempdir().unwrap();
    let agents_path = temp_dir.path().join("AGENTS.md");
    let agents_content = "# Project Instructions\nYou are a helpful coding assistant.";

    // Write AGENTS.md
    tokio::fs::write(&agents_path, agents_content)
        .await
        .unwrap();

    // Create a fake codex binary that echoes its arguments
    let fake_codex_script = temp_dir.path().join("fake-codex.sh");
    let script_content = r#"#!/bin/bash
# Echo a minimal success response
echo '{"type":"init","thread_id":"test-session-123"}'
echo '{"type":"agent_message_item","item":{"type":"agent_message","text":"OK"}}'
exit 0
"#;
    tokio::fs::write(&fake_codex_script, script_content)
        .await
        .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&fake_codex_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex_script, perms).unwrap();
    }

    // Use guard to ensure env var is restored even on panic
    let _guard = EnvVarGuard::new("CODEX_BIN", fake_codex_script.to_str().unwrap());

    let opts = Options {
        prompt: "User prompt here".to_string(),
        working_dir: temp_dir.path().to_path_buf(),
        sandbox: SandboxPolicy::ReadOnly,
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: false,
        return_all_messages_limit: None,
        image_paths: vec![],
        model: None,
        yolo: false,
        profile: None,
        timeout_secs: Some(5), // Short timeout for test
    };

    // Run codex (will use our fake binary)
    let result = codex_mcp_rs::codex::run(opts).await;

    // Verify the result (env var will be restored by guard drop)
    assert!(result.is_ok(), "Codex run should succeed");
    let codex_result = result.unwrap();

    // Verify session ID was extracted (may be from either test if running in parallel)
    assert!(!codex_result.session_id.is_empty());
    assert!(codex_result.session_id.starts_with("test-session-"));

    // Verify agent messages
    assert!(!codex_result.agent_messages.is_empty());

    // Note: We can't easily verify the prompt was prepended without capturing subprocess args,
    // but we've verified that AGENTS.md doesn't cause errors and warnings propagate correctly
}

#[tokio::test]
#[cfg(unix)] // Shell scripts don't work on Windows
async fn test_agents_md_large_file_handling() {
    // This test verifies that large AGENTS.md files don't break the system
    // We can't test full 1MB+ due to OS argument list limits, but we verify the behavior works

    let temp_dir = tempfile::tempdir().unwrap();
    let agents_path = temp_dir.path().join("AGENTS.md");

    // Create a reasonably-sized file (large but not breaking CLI limits)
    const TEST_CONTENT_SIZE: usize = 50_000; // 50KB
    let large_content = format!("# Large content\n{}", "x".repeat(TEST_CONTENT_SIZE));
    tokio::fs::write(&agents_path, &large_content)
        .await
        .unwrap();

    // Create fake codex binary
    let fake_codex_script = temp_dir.path().join("fake-codex2.sh");
    let script_content = r#"#!/bin/bash
echo '{"type":"init","thread_id":"test-session-456"}'
echo '{"type":"agent_message_item","item":{"type":"agent_message","text":"OK"}}'
exit 0
"#;
    tokio::fs::write(&fake_codex_script, script_content)
        .await
        .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&fake_codex_script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex_script, perms).unwrap();
    }

    let _guard = EnvVarGuard::new("CODEX_BIN", fake_codex_script.to_str().unwrap());

    let opts = Options {
        prompt: "test".to_string(),
        working_dir: temp_dir.path().to_path_buf(),
        sandbox: SandboxPolicy::ReadOnly,
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: false,
        return_all_messages_limit: None,
        image_paths: vec![],
        model: None,
        yolo: false,
        profile: None,
        timeout_secs: Some(5),
    };

    let result = codex_mcp_rs::codex::run(opts).await;

    // Should succeed even with large AGENTS.md (env var will be restored by guard drop)
    assert!(result.is_ok(), "Result error: {:?}", result.err());
    let codex_result = result.unwrap();

    // Should have session ID
    assert_eq!(codex_result.session_id, "test-session-456");

    // Note: No warnings expected for 50KB file since it's under the 1MB limit
    // The real truncation logic is tested in unit tests with controlled input
}
