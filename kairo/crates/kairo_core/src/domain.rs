//! Safe DTOs mirroring Kairo's `/api/v1` wire contract.
//!
//! These are intentionally *not* the Go domain structs: sensitive fields
//! (catalina base, marker token, environment, JVM options, local paths) never
//! cross this boundary. See Kairo ADR-0012 / ADR-0013.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stable error codes from the Kairo protocol. Unknown codes are preserved as
/// strings so a newer agent cannot make the client panic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ErrorCode(pub String);

impl ErrorCode {
    pub const TOOLCHAIN_INCOMPATIBLE: &'static str = "toolchain_incompatible";
    pub const TOOLCHAIN_MISSING: &'static str = "toolchain_missing";
    pub const COMPILE_FAILED: &'static str = "compile_failed";
    pub const DEPLOY_FAILED: &'static str = "deploy_failed";
    pub const DEBUG_ATTACH_FAILED: &'static str = "debug_attach_failed";

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub retryable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope<T: Serialize> {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub payload: T,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope {
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<ApiError>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub agent_version: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub bind_address: String,
    #[serde(default)]
    pub platform: PlatformInfo,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEndpoints {
    pub http: String,
    pub events: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub root_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProjectLayout {
    #[serde(default)]
    pub root_path: String,
    #[serde(default)]
    pub build_system: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub layout: LegacyLayout,
    #[serde(default)]
    pub encoding_by_extension: BTreeMap<String, String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyLayout {
    #[serde(default)]
    pub src: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub lib: String,
    #[serde(default)]
    pub web_root: String,
    #[serde(default)]
    pub build_xml: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetection {
    #[serde(default)]
    pub detected: Vec<DetectedProjectLayout>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub source_roots: Vec<String>,
    #[serde(default)]
    pub resource_roots: Vec<String>,
    #[serde(default)]
    pub library_dirs: Vec<String>,
    #[serde(default)]
    pub webapp_dir: String,
    #[serde(default)]
    pub output_dir: String,
    #[serde(default)]
    pub build_file: String,
    #[serde(default)]
    pub source_level: String,
    #[serde(default)]
    pub target_level: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub build_tool: String,
    #[serde(default)]
    pub context_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toolchain {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub home: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub source_levels: Vec<String>,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub imported_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRequest {
    pub project_id: String,
    #[serde(default)]
    pub clean: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_files: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub column: u32,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildResult {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    /// The agent calls this `state` (`queued`/`running`/`success`/`failure`/`cancelled`).
    #[serde(default, alias = "status")]
    pub state: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub files_compiled: u32,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub finished_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployRequest {
    pub project_id: String,
    #[serde(default)]
    pub build_id: String,
    #[serde(default)]
    pub scope: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub id: String,
    #[serde(default, alias = "status")]
    pub state: String,
    #[serde(default)]
    pub files_touched: u64,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub hot_reload_mode: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartServerRequest {
    pub project_id: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub java_home: String,
    #[serde(default)]
    pub context_path: String,
    #[serde(default)]
    pub http_port: u16,
    #[serde(default)]
    pub debug_port: u16,
    #[serde(default)]
    pub jvm_options: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerResponse {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub ports: ServerPorts,
    #[serde(default)]
    pub context_path: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub runtime_instance_id: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPorts {
    #[serde(default)]
    pub http: u16,
    #[serde(default)]
    pub debug: u16,
    #[serde(default)]
    pub shutdown: u16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    #[serde(default)]
    pub line: String,
    #[serde(default)]
    pub ts: String,
    #[serde(default)]
    pub stream: String,
    #[serde(default)]
    pub ordinal: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingDetection {
    pub file: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub has_bom: bool,
    #[serde(default)]
    pub eol: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecodeRequest {
    pub file: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub eol: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDescriptor {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub env_allowlist: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KairoEvent {
    #[serde(default)]
    pub sequence: u64,
    #[serde(rename = "type", default)]
    pub event_type: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub time: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAdapterStatus {
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub host_jdk: String,
    #[serde(default)]
    pub bridge_path: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateEncodingRequest {
    pub text: String,
    pub encoding: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateEncodingResponse {
    #[serde(default)]
    pub valid: bool,
    #[serde(default)]
    pub error: String,
}

/// Convert an API error into a human-readable message that preserves the
/// fail-closed code (important for `toolchain_incompatible`).
impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ApiError {}
