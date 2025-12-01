use crate::codex::{self, Options};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

mod serialize_as_os_string_vec {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::path::PathBuf;

    #[allow(dead_code)]
    pub fn serialize<S>(paths: &Vec<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(paths.len()))?;
        for path in paths {
            match path.to_str() {
                Some(s) => seq.serialize_element(s)?,
                None => return Err(serde::ser::Error::custom("path contains invalid UTF-8")),
            }
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec_strings = <Vec<String> as Deserialize>::deserialize(deserializer)?;
        Ok(vec_strings.into_iter().map(PathBuf::from).collect())
    }
}

/// Input parameters for codex tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CodexArgs {
    /// Instruction for task to send to codex
    #[serde(rename = "PROMPT")]
    pub prompt: String,
    /// Attach one or more image files to the initial prompt.
    #[serde(
        serialize_with = "serialize_as_os_string_vec::serialize",
        deserialize_with = "serialize_as_os_string_vec::deserialize",
        default
    )]
    pub images: Vec<PathBuf>,
    /// Resume a previously started Codex session. Must be the exact `SESSION_ID`
    /// string returned by an earlier `codex` tool call (typically a UUID). If
    /// omitted, a new session is created. Do not pass custom labels here.
    #[serde(rename = "SESSION_ID", default)]
    pub session_id: Option<String>,
}

/// Output from the codex tool
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct CodexOutput {
    success: bool,
    #[serde(rename = "SESSION_ID")]
    session_id: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_messages_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    all_messages: Option<Vec<HashMap<String, Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    all_messages_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<String>,
}

fn build_codex_output(
    result: &codex::CodexResult,
    return_all_messages: bool,
    warnings: Option<String>,
) -> CodexOutput {
    CodexOutput {
        success: result.success,
        session_id: result.session_id.clone(),
        message: result.agent_messages.clone(),
        agent_messages_truncated: result.agent_messages_truncated.then_some(true),
        all_messages: return_all_messages.then_some(result.all_messages.clone()),
        all_messages_truncated: (return_all_messages && result.all_messages_truncated)
            .then_some(true),
        error: result.error.clone(),
        warnings,
    }
}

#[derive(Clone)]
pub struct CodexServer {
    tool_router: ToolRouter<CodexServer>,
}

impl Default for CodexServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl CodexServer {
    /// Executes a non-interactive Codex session via CLI to perform AI-assisted coding tasks.
    /// This tool wraps the `codex exec` command, enabling model-driven code generation, debugging,
    /// or automation based on natural language prompts, and supports resuming ongoing sessions for continuity.
    #[tool(
        name = "codex",
        description = "Execute Codex CLI for AI-assisted coding tasks"
    )]
    async fn codex(
        &self,
        Parameters(args): Parameters<CodexArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate required parameters
        if args.prompt.is_empty() {
            return Err(McpError::invalid_params(
                "PROMPT is required and must be a non-empty string",
                None,
            ));
        }

        // Resolve and validate working directory based on the current process directory.
        let working_dir = std::env::current_dir().map_err(|e| {
            McpError::invalid_params(
                format!("failed to resolve current working directory: {}", e),
                None,
            )
        })?;
        let canonical_working_dir = working_dir.canonicalize().map_err(|e| {
            McpError::invalid_params(
                format!(
                    "working directory does not exist or is not accessible: {} ({})",
                    working_dir.display(),
                    e
                ),
                None,
            )
        })?;

        if !canonical_working_dir.is_dir() {
            return Err(McpError::invalid_params(
                format!(
                    "working directory is not a directory: {}",
                    working_dir.display()
                ),
                None,
            ));
        }

        // Validate image files exist and are regular files
        let mut canonical_image_paths = Vec::new();
        for img_path in &args.images {
            // Resolve image path relative to the working directory first, then canonicalize
            let resolved_path = if img_path.is_absolute() {
                img_path.clone()
            } else {
                canonical_working_dir.join(img_path)
            };

            let canonical = resolved_path.canonicalize().map_err(|e| {
                McpError::invalid_params(
                    format!(
                        "image file does not exist or is not accessible: {} ({})",
                        resolved_path.display(),
                        e
                    ),
                    None,
                )
            })?;

            if !canonical.is_file() {
                return Err(McpError::invalid_params(
                    format!("image path is not a file: {}", resolved_path.display()),
                    None,
                ));
            }

            canonical_image_paths.push(canonical);
        }

        // Create options for codex client
        let opts = Options {
            prompt: args.prompt,
            working_dir: canonical_working_dir,
            session_id: args.session_id,
            additional_args: codex::default_additional_args(),
            image_paths: canonical_image_paths,
            timeout_secs: None,
        };

        // Execute codex
        let result = codex::run(opts).await.map_err(|e| {
            McpError::internal_error(format!("Failed to execute codex: {}", e), None)
        })?;

        let combined_warnings = result.warnings.clone();

        // Prepare the response
        let output = build_codex_output(&result, false, combined_warnings);

        let json_output = serde_json::to_string(&output).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize output: {}", e), None)
        })?;

        // Always return structured content so callers can inspect success, error, and warning fields.
        Ok(CallToolResult::success(vec![Content::text(json_output)]))
    }
}

#[tool_handler]
impl ServerHandler for CodexServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a codex tool for AI-assisted coding tasks. Use the codex tool to execute coding tasks via the Codex CLI.".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
