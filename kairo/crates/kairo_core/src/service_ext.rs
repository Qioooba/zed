//! High-level operations that make every Kairo endpoint reachable from Zed.
//! Results are stored on the service and rendered in the Kairo panel.

use anyhow::{Context as _, Result, anyhow};
use gpui::{Context, Task};
use serde_json::{Value, json};

use crate::{domain::LogLine, service::KairoService};

impl KairoService {
    /// Dispatch one named operation to the agent and store its result.
    pub fn invoke(&mut self, operation: &str, cx: &mut Context<Self>) -> Task<Result<()>> {
        if !self.is_running() {
            self.last_error = Some("Kairo runtime agent is not running".to_owned());
            cx.notify();
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        }
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        let operation = operation.to_owned();
        let workspace_id = self.active_workspace_id.clone().unwrap_or_default();
        let project_id = self.active_project_id.clone().unwrap_or_default();
        let root = self.worktree_root.clone().unwrap_or_default();

        cx.spawn(async move |this, cx| {
            let result = dispatch(&client, &operation, &workspace_id, &project_id, &root).await;
            match result {
                Ok(value) => {
                    this.update(cx, |service, cx| {
                        service.last_result = Some(value);
                        service.last_error = None;
                        service.status_message = format!("Kairo: {operation} completed");
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    let message = format!("{error:#}");
                    this.update(cx, |service, cx| {
                        service.last_error = Some(message.clone());
                        service.status_message = format!("Kairo: {operation} failed");
                        cx.notify();
                    })?;
                    Err(error)
                }
            }
        })
    }
}

async fn dispatch(
    client: &crate::client::KairoClient,
    operation: &str,
    workspace_id: &str,
    project_id: &str,
    root: &str,
) -> Result<Value> {
    match operation {
        "endpoints" => client.runtime_endpoints().await,
        "restart-agent" => client.restart_runtime().await,
        "operation-status" => client.operation("latest").await,
        "legacy-import" => {
            client
                .legacy_import_project(json!({ "workspaceId": workspace_id, "rootPath": root }))
                .await
        }
        "unregister-project" => client.unregister_project(project_id).await,
        "list-run-configurations" => client.list_run_configurations(workspace_id).await,
        "save-run-configurations" => {
            client
                .save_run_configurations(workspace_id, json!({ "configurations": [] }))
                .await
        }
        "launch-run-configuration" => {
            client
                .launch_run_configuration(workspace_id, "default", "run", false)
                .await
        }
        "launch-jobs" => client.launch_job("latest").await,
        "cancel-build" => client
            .cancel_build(&format!("build_{operation}"))
            .await
            .map(|value| serde_json::to_value(value).unwrap_or_default()),
        "custom-build" => {
            client
                .custom_build(json!({
                    "projectRoot": root,
                    "command": "ant compile",
                    "timeoutMs": 600000,
                }))
                .await
        }
        "custom-build-status" => client.custom_build_status("latest").await,
        "ant-classpath" => client.analyze_ant_classpath(root, "build.xml").await,
        "deployment-history" => client.deployment_history().await,
        "deployment" => client.deployment("latest").await,
        "recover-server" => client.recover_server("latest").await,
        "recoverable-servers" => client.recoverable_servers().await,
        "port-diagnostics" => client.port_diagnostics(18082).await,
        "jdtls-status" => client.jdtls_status().await,
        "prepare-jdtls" => client.prepare_jdtls_endpoint().await,
        "debug-tunnel-connect" => {
            client
                .debug_tunnel_connect(json!({ "host": "127.0.0.1", "port": 8000 }))
                .await
        }
        "debug-tunnel-disconnect" => client.debug_tunnel_disconnect().await,
        "provision-debug-jdk" => client.provision_debug_jdk().await,
        "run-java" => {
            client
                .run_java(json!({
                    "projectRoot": root,
                    "className": "com.example.HelloWorld",
                    "methodType": "main",
                }))
                .await
        }
        "detect-java" => {
            client
                .detect_java(&format!("{root}/src/main/java/com/example/HelloWorld.java"))
                .await
        }
        "hot-swap" => {
            client
                .jvm_redefine(json!({
                    "sourcePath": format!("{root}/src/main/java/com/example/HelloWorld.java"),
                    "classPath": format!("{root}/build/classes"),
                    "className": "com.example.HelloWorld",
                    "projectId": project_id,
                }))
                .await
        }
        "maven-detect" => client.maven_detect(root).await,
        "maven-dependencies" => client.maven_dependencies(root, true).await,
        "maven-run" => client.maven_run(root, "compile").await,
        "sql-test" => {
            client
                .sql_test_connection(json!({
                    "host": "127.0.0.1",
                    "port": 1521,
                    "username": "scott",
                    "password": "tiger",
                }))
                .await
        }
        "audit" => client.audit().await,
        "search" => {
            client
                .search(json!({
                    "query": "Hello",
                    "workspaceId": workspace_id,
                    "rootPath": root,
                    "projectEncoding": "gbk",
                    "maxResults": 10000,
                }))
                .await
        }
        "encoding-detect" => client
            .detect_encoding(&format!("{root}/WebRoot/hello.jsp"))
            .await
            .map(|value| serde_json::to_value(value).unwrap_or_default()),
        other => Err(anyhow!("unknown Kairo operation: {other}")),
    }
    .with_context(|| format!("operation {operation} failed"))
}

/// Keep the log-line type referenced so the panel's bounded log view remains
/// part of the public surface of this crate.
pub fn log_preview(lines: &[LogLine]) -> Vec<String> {
    lines
        .iter()
        .take(200)
        .map(|line| line.line.clone())
        .collect()
}
