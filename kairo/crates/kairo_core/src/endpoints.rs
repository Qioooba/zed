//! Remaining `/api/v1` endpoints. Split out from `client.rs` to keep each file
//! reviewable; all methods use the same typed envelope and error handling.

use anyhow::Result;
use serde_json::{Value, json};

use crate::client::KairoClient;

impl KairoClient {
    // --- discovery / lifecycle -------------------------------------------------
    pub async fn runtime_endpoints(&self) -> Result<Value> {
        self.get_value("/api/v1/endpoints").await
    }

    pub async fn restart_runtime(&self) -> Result<Value> {
        self.post("/api/v1/runtime/restart", &json!({})).await
    }

    pub async fn operation(&self, request_id: &str) -> Result<Value> {
        self.get_value(&format!("/api/v1/operations/{request_id}"))
            .await
    }

    // --- workspace / project ---------------------------------------------------
    pub async fn legacy_import_project(&self, payload: Value) -> Result<Value> {
        let workspace_id = payload
            .get("workspaceId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let path = format!("/api/v1/workspaces/{workspace_id}/projects/import");
        self.post(&path, &payload).await
    }

    pub async fn unregister_project(&self, project_id: &str) -> Result<Value> {
        self.delete_value(&format!("/api/v1/projects/{project_id}"))
            .await
    }

    // --- run configurations ----------------------------------------------------
    pub async fn list_run_configurations(&self, workspace_id: &str) -> Result<Value> {
        self.get_value(&format!(
            "/api/v1/workspaces/{workspace_id}/run-configurations"
        ))
        .await
    }

    pub async fn save_run_configurations(
        &self,
        workspace_id: &str,
        document: Value,
    ) -> Result<Value> {
        self.put_value(
            &format!("/api/v1/workspaces/{workspace_id}/run-configurations"),
            document,
        )
        .await
    }

    pub async fn create_run_configuration(
        &self,
        workspace_id: &str,
        configuration: Value,
    ) -> Result<Value> {
        self.post(
            &format!("/api/v1/workspaces/{workspace_id}/run-configurations"),
            &configuration,
        )
        .await
    }

    pub async fn run_configuration(&self, workspace_id: &str, id: &str) -> Result<Value> {
        self.get_value(&format!(
            "/api/v1/workspaces/{workspace_id}/run-configurations/{id}"
        ))
        .await
    }

    pub async fn update_run_configuration(
        &self,
        workspace_id: &str,
        id: &str,
        configuration: Value,
    ) -> Result<Value> {
        self.put_value(
            &format!("/api/v1/workspaces/{workspace_id}/run-configurations/{id}"),
            configuration,
        )
        .await
    }

    pub async fn delete_run_configuration(&self, workspace_id: &str, id: &str) -> Result<Value> {
        self.delete_value(&format!(
            "/api/v1/workspaces/{workspace_id}/run-configurations/{id}"
        ))
        .await
    }

    pub async fn launch_run_configuration(
        &self,
        workspace_id: &str,
        id: &str,
        mode: &str,
        async_launch: bool,
    ) -> Result<Value> {
        let suffix = if async_launch { "?async=1" } else { "" };
        self.post(
            &format!("/api/v1/workspaces/{workspace_id}/run-configurations/{id}/launch{suffix}"),
            &json!({ "mode": mode }),
        )
        .await
    }

    pub async fn launch_job(&self, id: &str) -> Result<Value> {
        self.get_value(&format!("/api/v1/launch-jobs/{id}")).await
    }

    // --- builds ----------------------------------------------------------------
    pub async fn custom_build(&self, config: Value) -> Result<Value> {
        self.post("/api/v1/build/custom", &config).await
    }

    pub async fn custom_build_status(&self, build_id: &str) -> Result<Value> {
        self.get_value(&format!("/api/v1/build/custom/{build_id}"))
            .await
    }

    pub async fn cancel_custom_build(&self, build_id: &str) -> Result<Value> {
        self.post(
            &format!("/api/v1/build/custom/{build_id}/cancel"),
            &json!({}),
        )
        .await
    }

    pub async fn analyze_ant_classpath(
        &self,
        project_root: &str,
        build_file: &str,
    ) -> Result<Value> {
        self.post(
            "/api/v1/ant/classpath/analyze",
            &json!({ "projectRoot": project_root, "buildFile": build_file }),
        )
        .await
    }

    // --- deployments / servers -------------------------------------------------
    pub async fn deployment_history(&self) -> Result<Value> {
        self.get_value("/api/v1/deployments").await
    }

    pub async fn deployment(&self, id: &str) -> Result<Value> {
        self.get_value(&format!("/api/v1/deployments/{id}")).await
    }

    pub async fn recover_server(&self, server_id: &str) -> Result<Value> {
        self.post(&format!("/api/v1/servers/{server_id}/recover"), &json!({}))
            .await
    }

    pub async fn recoverable_servers(&self) -> Result<Value> {
        self.get_value("/api/v1/servers/recoverable").await
    }

    pub async fn port_diagnostics(&self, port: u16) -> Result<Value> {
        self.get_value(&format!("/api/v1/diagnostics/port?port={port}"))
            .await
    }

    // --- JDT / debug -----------------------------------------------------------
    pub async fn prepare_jdtls_endpoint(&self) -> Result<Value> {
        self.post("/api/v1/jdtls", &json!({})).await
    }

    pub async fn debug_tunnel_connect(&self, config: Value) -> Result<Value> {
        self.post("/api/v1/debug/tunnel/connect", &config).await
    }

    pub async fn debug_tunnel_disconnect(&self) -> Result<Value> {
        self.post("/api/v1/debug/tunnel/disconnect", &json!({}))
            .await
    }

    pub async fn provision_debug_jdk(&self) -> Result<Value> {
        self.post("/api/v1/debug/jdk/download", &json!({})).await
    }

    // --- Java run / detect -----------------------------------------------------
    pub async fn run_java(&self, request: Value) -> Result<Value> {
        self.post("/api/v1/java/run", &request).await
    }

    pub async fn java_run_session(&self, run_id: &str) -> Result<Value> {
        self.get_value(&format!("/api/v1/java/runs/{run_id}")).await
    }

    pub async fn stop_java_run(&self, run_id: &str) -> Result<Value> {
        self.delete_value(&format!("/api/v1/java/runs/{run_id}"))
            .await
    }

    pub async fn detect_java(&self, file_path: &str) -> Result<Value> {
        self.post("/api/v1/java/detect", &json!({ "filePath": file_path }))
            .await
    }

    // --- Maven / SQL / audit ---------------------------------------------------
    pub async fn maven_detect(&self, root: &str) -> Result<Value> {
        self.post("/api/v1/maven/detect", &json!({ "rootPath": root }))
            .await
    }

    pub async fn maven_dependencies(&self, root: &str, offline: bool) -> Result<Value> {
        self.get_value(&format!(
            "/api/v1/maven/dependencies?rootPath={root}&offline={offline}"
        ))
        .await
    }

    pub async fn maven_run(&self, root: &str, task: &str) -> Result<Value> {
        self.post(
            "/api/v1/maven/run",
            &json!({ "rootPath": root, "task": task }),
        )
        .await
    }

    pub async fn sql_test_connection(&self, config: Value) -> Result<Value> {
        self.post("/api/v1/sql/test-connection", &config).await
    }

    pub async fn sql_execute(
        &self,
        connection_id: &str,
        sql: &str,
        max_rows: u32,
    ) -> Result<Value> {
        self.post(
            "/api/v1/sql/execute",
            &json!({
                "connectionId": connection_id,
                "sql": sql,
                "maxRows": max_rows,
                "offset": 0,
            }),
        )
        .await
    }

    pub async fn audit(&self) -> Result<Value> {
        self.get_value("/api/v1/audit").await
    }
}

/// Small helpers that avoid repeating the envelope logic in this file.
impl KairoClient {
    async fn get_value(&self, path: &str) -> Result<Value> {
        self.get::<Value>(path).await
    }

    async fn delete_value(&self, path: &str) -> Result<Value> {
        self.delete::<Value>(path).await
    }

    async fn put_value(&self, path: &str, value: Value) -> Result<Value> {
        self.put(path, &value).await
    }
}
