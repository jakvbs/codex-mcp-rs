// Common test utilities and helpers

use std::path::PathBuf;

/// Get a temporary directory for testing
pub fn get_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Create a test options with default values
pub fn create_test_options(prompt: &str, working_dir: &str) -> codex_mcp_rs::codex::Options {
    codex_mcp_rs::codex::Options {
        prompt: prompt.to_string(),
        working_dir: working_dir.to_string(),
        sandbox: "read-only".to_string(),
        session_id: None,
        skip_git_repo_check: true,
        return_all_messages: false,
        image_paths: vec![],
        model: None,
        yolo: false,
        profile: None,
    }
}

/// Mock session ID generator
pub fn generate_mock_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("test-session-{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_temp_dir() {
        let temp = get_temp_dir();
        assert!(temp.exists());
        assert!(temp.is_dir());
    }

    #[test]
    fn test_create_test_options() {
        let opts = create_test_options("test prompt", "/tmp");
        assert_eq!(opts.prompt, "test prompt");
        assert_eq!(opts.working_dir, "/tmp");
        assert_eq!(opts.sandbox, "read-only");
    }

    #[test]
    fn test_generate_mock_session_id() {
        let id1 = generate_mock_session_id();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = generate_mock_session_id();

        assert!(id1.starts_with("test-session-"));
        assert!(id2.starts_with("test-session-"));
        assert_ne!(id1, id2);
    }
}
