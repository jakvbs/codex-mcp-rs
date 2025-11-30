use codex_mcp_rs::codex;
use codex_mcp_rs::codex::Options;
use std::env;
use std::path::PathBuf;

/// Verify that image_paths are passed as repeated --image flags to the Codex CLI.
#[tokio::test]
async fn test_image_paths_are_passed_to_codex_cli() {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("codex_mcp_image_test");
    let _ = std::fs::create_dir_all(&temp_path);

    // Create dummy image files
    let image1 = temp_path.join("img1.png");
    let image2 = temp_path.join("img2.png");
    std::fs::write(&image1, b"dummy").expect("Failed to write img1");
    std::fs::write(&image2, b"dummy").expect("Failed to write img2");

    // Path where the helper script will log its argv
    let log_path = temp_path.join("codex_image_args.log");

    // Helper script that logs argv and emits a minimal JSON event
    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let script_path = temp_path.join("echo_image_args.sh");
        let script_contents = r#"#!/bin/sh
LOG_FILE="${CODEX_IMAGE_ARGS_LOG}"
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
    }

    #[cfg(target_os = "windows")]
    {
        use std::fs;

        let script_path = temp_path.join("echo_image_args.bat");
        let script_contents = r#"@echo off
set LOG_FILE=%CODEX_IMAGE_ARGS_LOG%
echo %0 %* > "%LOG_FILE%"
echo {"thread_id":"test-session","item":{"type":"agent_message","text":"ok"}}
"#;
        fs::write(&script_path, script_contents).expect("Failed to write script");
        env::set_var("CODEX_BIN", script_path.to_str().unwrap());
    }

    env::set_var("CODEX_IMAGE_ARGS_LOG", log_path.to_str().unwrap());

    let opts = Options {
        prompt: "test images".to_string(),
        working_dir: temp_path.clone(),
        session_id: None,
        additional_args: Vec::new(),
        image_paths: vec![image1.clone(), image2.clone()],
        timeout_secs: Some(10),
    };

    let result = codex::run(opts).await.expect("run should return Ok");

    assert!(result.success, "helper script should succeed");
    assert_eq!(result.session_id, "test-session");
    assert_eq!(result.agent_messages.trim(), "ok");

    // Verify that image paths were passed through as --image flags
    let log = std::fs::read_to_string(&log_path).expect("failed to read args log");
    let parts: Vec<&str> = log.split_whitespace().collect();

    // Find all indices of --image
    let indices: Vec<usize> = parts
        .iter()
        .enumerate()
        .filter_map(|(i, s)| if *s == "--image" { Some(i) } else { None })
        .collect();

    assert_eq!(indices.len(), 2, "expected two --image flags in argv");

    // Each --image should be followed by the corresponding path
    assert_eq!(PathBuf::from(parts[indices[0] + 1]), image1);
    assert_eq!(PathBuf::from(parts[indices[1] + 1]), image2);

    env::remove_var("CODEX_BIN");
    env::remove_var("CODEX_IMAGE_ARGS_LOG");
}

