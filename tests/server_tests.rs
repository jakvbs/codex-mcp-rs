use codex_mcp_rs::server::CodexServer;
use rmcp::{model::*, ServerHandler};

#[test]
fn test_server_creation() {
    let server = CodexServer::new();

    // Server should be created successfully
    assert!(std::mem::size_of_val(&server) > 0);
}

#[test]
fn test_server_info() {
    let server = CodexServer::new();
    let info = server.get_info();

    // Check protocol version
    assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);

    // Check capabilities
    assert!(info.capabilities.tools.is_some());

    // Check server info - name and version come from Implementation::from_build_env()
    assert!(!info.server_info.name.is_empty());
    assert!(!info.server_info.version.is_empty());

    // Check instructions
    assert!(info.instructions.is_some());
    assert!(info.instructions.unwrap().contains("codex tool"));
}

#[test]
fn test_default_implementation() {
    let server1 = CodexServer::new();
    let server2 = CodexServer::default();

    // Both should create valid servers
    assert!(std::mem::size_of_val(&server1) > 0);
    assert!(std::mem::size_of_val(&server2) > 0);
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_server_name() {
        let server = CodexServer::new();
        let info = server.get_info();

        assert!(!info.server_info.name.is_empty());
        // Name is set by from_build_env() which uses package name
        assert!(!info.server_info.name.is_empty());
    }

    #[test]
    fn test_version_format() {
        let server = CodexServer::new();
        let info = server.get_info();

        // Version should be in semver format (x.y.z)
        let version = &info.server_info.version;
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }
}
