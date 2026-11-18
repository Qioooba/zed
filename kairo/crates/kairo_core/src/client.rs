//! Typed client for the Kairo runtime agent's `/api/v1` HTTP protocol.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Context as _, Result, bail};
use futures::AsyncReadExt as _;
use gpui::App;
use http_client::{AsyncBody, HttpClient, Method, Request as HttpRequest};

use crate::domain::{
    ApiError, BuildRequest, BuildResult, DebugAdapterStatus, DeployRequest, DeployResult,
    EncodingDetection, HealthResponse, KairoEvent, LaunchDescriptor, LogLine, Project,
    ProjectDetection, RecodeRequest, RequestEnvelope, ResponseEnvelope, RuntimeEndpoints,
    ServerResponse, StartServerRequest, Toolchain, WorkspaceRecord,
};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn new_request_id() -> String {
    format!("req_kairo_{}", NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
}

/// A client bound to one running Kairo runtime-agent process.
#[derive(Clone)]
pub struct KairoClient {
    http: Arc<dyn HttpClient>,
    base_url: String,
    secret: Option<String>,
    workspace_id: Option<String>,
    project_id: Option<String>,
}

impl KairoClient {
    pub fn new(
        http: Arc<dyn HttpClient>,
        base_url: String,
        secret: Option<String>,
    ) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_owned(),
            secret,
            workspace_id: None,
            project_id: None,
        }
    }

    pub fn with_scope(mut self, workspace_id: Option<String>, project_id: Option<String>) -> Self {
        self.workspace_id = workspace_id;
        self.project_id = project_id;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Discover the actual bound port when the agent was started with `-port 0`.
    /// `GET /health` is public and also reports the chosen port, but
    /// `GET /endpoints` is the canonical discovery endpoint.
    pub async fn endpoints(&self) -> Result<RuntimeEndpoints> {
        self.get::<RuntimeEndpoints>("/api/v1/endpoints").await
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        self.get::<HealthResponse>("/api/v1/health").await
    }

    pub async fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>> {
        self.get::<Vec<WorkspaceRecord>>("/api/v1/workspaces").await
    }

    pub async fn open_workspace(&self, root_path: impl Into<String>) -> Result<WorkspaceRecord> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct OpenWorkspace {
            root_path: String,
        }
        self.post(
            "/api/v1/workspaces",
            &OpenWorkspace {
                root_path: root_path.into(),
            },
        )
        .await
    }

    pub async fn scan_project(&self, workspace_id: &str) -> Result<ProjectDetection> {
        self.post(
            &format!("/api/v1/workspaces/{workspace_id}/scan"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn detect_project(&self, root_path: &str) -> Result<serde_json::Value> {
        self.post(
            "/api/v1/projects/detect",
            &serde_json::json!({ "rootPath": root_path }),
        )
        .await
    }

    pub async fn import_project(&self, payload: serde_json::Value) -> Result<Project> {
        self.post("/api/v1/projects/import", &payload).await
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        self.get::<Vec<Project>>("/api/v1/projects").await
    }

    pub async fn list_toolchains(&self) -> Result<Vec<Toolchain>> {
        self.get::<Vec<Toolchain>>("/api/v1/toolchains").await
    }

    pub async fn start_build(&self, request: &BuildRequest) -> Result<BuildResult> {
        self.post("/api/v1/builds", request).await
    }

    pub async fn build(&self, build_id: &str) -> Result<BuildResult> {
        self.get::<BuildResult>(&format!("/api/v1/builds/{build_id}"))
            .await
    }

    pub async fn cancel_build(&self, build_id: &str) -> Result<BuildResult> {
        self.delete::<BuildResult>(&format!("/api/v1/builds/{build_id}"))
            .await
    }

    pub async fn deploy(&self, request: &DeployRequest) -> Result<DeployResult> {
        self.post("/api/v1/deployments", request).await
    }

    pub async fn start_server(&self, request: &StartServerRequest) -> Result<ServerResponse> {
        self.post("/api/v1/servers", request).await
    }

    pub async fn get_servers_for_workspace(&self, workspace_id: &str) -> Result<Vec<ServerResponse>> {
        self.get::<Vec<ServerResponse>>(&format!(
            "/api/v1/servers?workspaceId={workspace_id}"
        ))
        .await
    }

    pub async fn server(&self, server_id: &str) -> Result<ServerResponse> {
        self.get::<ServerResponse>(&format!("/api/v1/servers/{server_id}"))
            .await
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<ServerResponse> {
        self.delete::<ServerResponse>(&format!("/api/v1/servers/{server_id}"))
            .await
    }

    pub async fn restart_server(&self, server_id: &str) -> Result<ServerResponse> {
        self.post(
            &format!("/api/v1/servers/{server_id}/restart"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn debug_server(&self, server_id: &str) -> Result<ServerResponse> {
        self.post(
            &format!("/api/v1/servers/{server_id}/debug"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn reload_server(&self, server_id: &str) -> Result<serde_json::Value> {
        self.post(
            &format!("/api/v1/servers/{server_id}/reload"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn server_logs(&self, server_id: &str, tail: u32) -> Result<Vec<LogLine>> {
        self.get::<Vec<LogLine>>(&format!(
            "/api/v1/servers/{server_id}/logs?tail={tail}"
        ))
        .await
    }

    /// Apply the Zed-side settings to the registered project. Levels and
    /// encoding are server-derived on build, so this is the correct seam for
    /// `source_level` / `target_level` / `encoding`.
    pub async fn update_project(&self, project_id: &str, project: &serde_json::Value) -> Result<Project> {
        self.put(
            &format!("/api/v1/projects/{project_id}"),
            project,
        )
        .await
    }

    pub async fn validate_encoding(&self, text: &str, encoding: &str) -> Result<serde_json::Value> {
        self.post(
            "/api/v1/encoding/validate",
            &crate::domain::ValidateEncodingRequest {
                text: text.to_owned(),
                encoding: encoding.to_owned(),
            },
        )
        .await
    }

    pub async fn prepare_jdtls(&self, workspace_id: &str) -> Result<serde_json::Value> {
        self.post(
            &format!("/api/v1/workspaces/{workspace_id}/java/prepare"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn jdtls_status(&self) -> Result<serde_json::Value> {
        self.get::<serde_json::Value>("/api/v1/jdtls").await
    }

    pub async fn detect_encoding(&self, file: &str) -> Result<EncodingDetection> {
        self.post(
            "/api/v1/encoding/detect",
            &serde_json::json!({ "file": file }),
        )
        .await
    }

    pub async fn recode(&self, request: &RecodeRequest) -> Result<serde_json::Value> {
        self.post("/api/v1/encoding/recode", request).await
    }

    pub async fn launch_descriptor(&self, workspace_id: &str, project_id: &str) -> Result<LaunchDescriptor> {
        self.get::<LaunchDescriptor>(&format!(
            "/api/v1/workspaces/{workspace_id}/java/launch-descriptor?projectId={project_id}"
        ))
        .await
    }

    pub async fn generate_jdt_project(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        self.post("/api/v1/jdtls/project", &payload).await
    }

    pub async fn debug_adapter_status(&self) -> Result<DebugAdapterStatus> {
        self.get::<DebugAdapterStatus>("/api/v1/debug/adapter/status")
            .await
    }

    pub async fn jvm_compile(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        self.post("/api/v1/jvm/compile", &payload).await
    }

    pub async fn jvm_redefine(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        self.post("/api/v1/jvm/redefine", &payload).await
    }

    pub async fn search(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        self.post("/api/v1/search", &payload).await
    }

    /// Subscribe to the agent's event stream. The caller owns the returned
    /// stream and can cancel it by dropping the future/connection.
    pub async fn subscribe_events(&self) -> Result<impl futures::Stream<Item = Result<KairoEvent>>> {
        // The real implementation upgrades `/api/v1/events` to a WebSocket with
        // the `kairo-secret-v1` subprotocol. This is a placeholder stream that
        // reports "not connected" so callers can degrade gracefully until WS
        // support is wired in kairo_core::events.
        Ok(futures::stream::empty())
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_json(Method::GET, path, None::<&serde_json::Value>)
            .await
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request_json(Method::POST, path, Some(body)).await
    }

    async fn delete<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_json(Method::DELETE, path, None::<&serde_json::Value>)
            .await
    }

    async fn put<T: serde::de::DeserializeOwned, B: serde::Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        // The agent routes PUT by path; reuse request_json's POST envelope
        // path by temporarily building the request directly.
        let uri = format!("{}{}", self.base_url, path);
        let payload_value = serde_json::to_value(body)?;
        let envelope = RequestEnvelope {
            request_id: new_request_id(),
            correlation_id: None,
            workspace_id: self.workspace_id.clone(),
            project_id: self.project_id.clone(),
            payload: payload_value,
        };
        let request = HttpRequest::builder()
            .method(Method::PUT)
            .uri(&uri)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(AsyncBody::from(serde_json::to_string(&envelope)?))
            .context("invalid Kairo PUT request")?;
        self.send_and_decode(request).await
    }

    async fn request_json<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let uri = format!("{}{}", self.base_url, path);
        let mut builder = HttpRequest::builder().method(method).uri(&uri);

        if let Some(payload) = body {
            let payload_value = serde_json::to_value(payload)?;
            let envelope = RequestEnvelope {
                request_id: new_request_id(),
                correlation_id: None,
                workspace_id: self.workspace_id.clone(),
                project_id: self.project_id.clone(),
                payload: payload_value,
            };
            let encoded = serde_json::to_string(&envelope)
                .context("failed to serialize Kairo request envelope")?;
            builder = builder
                .header("Content-Type", "application/json")
                .header("Accept", "application/json");
            let request = builder
                .body(AsyncBody::from(encoded))
                .context("invalid Kairo request")?;
            return self.send_and_decode(request).await;
        }

        let request = builder
            .header("Accept", "application/json")
            .body(AsyncBody::empty())
            .context("invalid Kairo request")?;
        self.send_and_decode(request).await
    }

    async fn send_and_decode<T: serde::de::DeserializeOwned>(
        &self,
        request: HttpRequest<AsyncBody>,
    ) -> Result<T> {
        let mut response = self
            .http
            .send(request)
            .await
            .context("failed to reach Kairo runtime agent")?;

        let status = response.status();
        let mut body_bytes = Vec::new();
        response
            .body_mut()
            .read_to_end(&mut body_bytes)
            .await
            .context("failed to read Kairo response body")?;

        if !status.is_success() {
            // Even error statuses carry the typed envelope. Try to surface the
            // closed error code rather than a generic HTTP error.
            if let Ok(envelope) =
                serde_json::from_slice::<ResponseEnvelope<serde_json::Value>>(&body_bytes)
                && let Some(error) = envelope.error
            {
                bail!(error);
            }
            bail!(
                "Kairo agent returned HTTP {} for request: {}",
                status,
                String::from_utf8_lossy(&body_bytes).chars().take(500).collect::<String>()
            );
        }

        // GET list endpoints return a bare JSON array; POST endpoints return
        // the envelope. Accept both so the client works across agent versions.
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
            if let Some(object) = value.as_object()
                && object.contains_key("ok")
            {
                let envelope: ResponseEnvelope<T> = serde_json::from_value(value)
                    .context("invalid Kairo response envelope")?;
                if !envelope.ok {
                    if let Some(error) = envelope.error {
                        bail!(error);
                    }
                    bail!("Kairo agent reported an unspecified error");
                }
                return envelope.payload.context("Kairo response missing payload");
            }
            return serde_json::from_value(value).context("invalid Kairo response payload");
        }

        bail!("Kairo agent returned a non-JSON response")
    }
}

/// Expose the shared HTTP client registered by Zed to the whole app.
pub fn global_http_client(cx: &App) -> Arc<dyn HttpClient> {
    client::Client::global(cx).http_client()
}

/// Convenience for callers that just want to know whether an error means the
/// agent is missing a real toolchain (never treat this as a transient error).
pub fn is_toolchain_incompatible(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ApiError>()
        .is_some_and(|api| api.code.as_str() == crate::domain::ErrorCode::TOOLCHAIN_INCOMPATIBLE)
}
