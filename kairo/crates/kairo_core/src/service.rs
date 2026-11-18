//! Application-level service that owns the Kairo agent and its derived state.
//!
//! This is the single Zed-side entry point for every legacy operation. It
//! always routes OS effects through the Go agent (Kairo ADR-0002) and always
//! surfaces the agent's real error codes to the UI — never a false green.

use std::{
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, Context, Entity, Global, Task, TaskExt as _};
use http_client::HttpClient;
use util::ResultExt as _;

use crate::{
    agent::{AgentHandle, AgentLaunchConfig, spawn_agent},
    client::{KairoClient, global_http_client},
    domain::{
        BuildRequest, BuildResult, DebugAdapterStatus, DeployRequest, DeployResult,
        DetectedProjectLayout, EncodingDetection, LogLine, Project, RecodeRequest, ServerResponse,
        StartServerRequest, Toolchain, WorkspaceRecord,
    },
    settings::KairoSettings,
};

/// Global handle to the single Kairo service for the application.
pub struct GlobalKairoService(pub Entity<KairoService>);

impl Global for GlobalKairoService {}

pub struct KairoService {
    http: Arc<dyn HttpClient>,
    settings: KairoSettings,
    agent: Option<AgentHandle>,
    client: Option<KairoClient>,
    workspaces: Vec<WorkspaceRecord>,
    projects: Vec<Project>,
    toolchains: Vec<Toolchain>,
    builds: Vec<BuildResult>,
    servers: Vec<ServerResponse>,
    logs: Vec<LogLine>,
    detected: Vec<DetectedProjectLayout>,
    active_workspace_id: Option<String>,
    active_project_id: Option<String>,
    worktree_root: Option<String>,
    jdtls_descriptor: Option<crate::domain::LaunchDescriptor>,
    jdwp_port: u16,
    status_message: String,
    last_error: Option<String>,
    encoding: Option<EncodingDetection>,
    hot_reload_mode: String,
    debug_gate: Option<DebugAdapterStatus>,
    pending_deploy: bool,
    build_task: Option<Task<()>>,
    poll_task: Option<Task<()>>,
}

impl KairoService {
    pub fn new(http: Arc<dyn HttpClient>, settings: KairoSettings) -> Self {
        Self {
            http,
            settings,
            agent: None,
            client: None,
            workspaces: Vec::new(),
            projects: Vec::new(),
            toolchains: Vec::new(),
            builds: Vec::new(),
            servers: Vec::new(),
            logs: Vec::new(),
            detected: Vec::new(),
            active_workspace_id: None,
            active_project_id: None,
            worktree_root: None,
            jdtls_descriptor: None,
            jdwp_port: 8000,
            status_message: "Kairo agent is not running".to_owned(),
            last_error: None,
            encoding: None,
            hot_reload_mode: String::new(),
            debug_gate: None,
            pending_deploy: false,
            build_task: None,
            poll_task: None,
        }
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalKairoService>().0.clone()
    }

    pub fn client(&self) -> Result<KairoClient> {
        self.client
            .clone()
            .ok_or_else(|| anyhow!("Kairo runtime agent is not running"))
    }

    pub fn settings(&self) -> &KairoSettings {
        &self.settings
    }

    pub fn is_running(&self) -> bool {
        self.client.is_some()
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn workspaces(&self) -> &[WorkspaceRecord] {
        &self.workspaces
    }

    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    pub fn toolchains(&self) -> &[Toolchain] {
        &self.toolchains
    }

    pub fn builds(&self) -> &[BuildResult] {
        &self.builds
    }

    pub fn servers(&self) -> &[ServerResponse] {
        &self.servers
    }

    pub fn logs(&self) -> &[LogLine] {
        &self.logs
    }

    pub fn detected(&self) -> &[DetectedProjectLayout] {
        &self.detected
    }

    pub fn encoding(&self) -> Option<&EncodingDetection> {
        self.encoding.as_ref()
    }

    pub fn hot_reload_mode(&self) -> &str {
        &self.hot_reload_mode
    }

    pub fn debug_gate(&self) -> Option<&DebugAdapterStatus> {
        self.debug_gate.as_ref()
    }

    pub fn active_project_id(&self) -> Option<&str> {
        self.active_project_id.as_deref()
    }

    pub fn active_workspace_id(&self) -> Option<&str> {
        self.active_workspace_id.as_deref()
    }

    pub fn agent_base_url(&self) -> Option<String> {
        self.agent.as_ref().map(|agent| agent.base_url())
    }

    /// URL of the last started server, used by `kairo::OpenApplication`.
    pub fn application_url(&self) -> Option<String> {
        self.servers.last().map(|server| server.url.clone())
    }

    fn resolve_agent_path(&self) -> PathBuf {
        if !self.settings.agent_path.is_empty() {
            return PathBuf::from(&self.settings.agent_path);
        }
        if let Ok(path) = std::env::var("KAIRO_AGENT_PATH")
            && !path.is_empty()
        {
            return PathBuf::from(path);
        }
        PathBuf::from(r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe")
    }

    fn resolve_data_dir(&self) -> PathBuf {
        if !self.settings.data_dir.is_empty() {
            return PathBuf::from(&self.settings.data_dir);
        }
        let base = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(std::env::temp_dir);
        base.join("kairo-zed")
    }

    fn bundled_dir(&self) -> Option<PathBuf> {
        if self.settings.bundled_dir.is_empty() {
            None
        } else {
            Some(PathBuf::from(&self.settings.bundled_dir))
        }
    }

    /// A locally detected precondition failure must be just as visible as an
    /// agent error — otherwise users only see a silent no-op.
    fn unavailable(&mut self, message: impl Into<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
        let message = message.into();
        self.last_error = Some(message.clone());
        self.status_message = "Kairo operation unavailable".to_owned();
        cx.notify();
        Task::ready(Err(anyhow!(message)))
    }

    /// Report an operation failure in the UI. This is the counterpart of the
    /// agent's closed error enum — no error may disappear into the console.
    pub fn report_error(&mut self, error: &anyhow::Error, cx: &mut Context<Self>) {
        self.set_error(error, cx);
    }

    fn set_error(&mut self, error: &anyhow::Error, cx: &mut Context<Self>) {
        self.last_error = Some(format!("{error:#}"));
        self.status_message = "Kairo operation failed".to_owned();
        cx.notify();
    }

    /// Start the Go runtime agent, then load its catalogs and try to select a
    /// project automatically (the common single-project case).
    pub fn start_agent(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        if self.client.is_some() {
            return self.refresh(cx);
        }
        let config = AgentLaunchConfig {
            agent_path: self.resolve_agent_path(),
            data_dir: self.resolve_data_dir(),
            bundled_dir: self.bundled_dir(),
            bind_address: "127.0.0.1".to_owned(),
            log_level: "info".to_owned(),
            secret: std::env::var("KAIRO_LOCAL_SECRET")
                .ok()
                .filter(|value| !value.is_empty()),
            port: 0,
        };
        let executor = cx.background_executor().clone();
        let http = self.http.clone();

        cx.spawn(async move |this, cx| {
            let agent = spawn_agent(executor, config)
                .await
                .context("failed to start the Kairo runtime agent")?;
            let base_url = agent.base_url();
            let secret = std::env::var("KAIRO_LOCAL_SECRET").ok().filter(|value| !value.is_empty());
            let client = KairoClient::new(http, base_url.clone(), secret);
            client
                .health()
                .await
                .context("Kairo agent did not become healthy")?;

            this.update(cx, |service, cx| {
                service.agent = Some(agent);
                service.client = Some(client);
                service.status_message = format!("Kairo agent connected at {base_url}");
                service.last_error = None;
                cx.notify();
            })?;
            this.update(cx, |service, cx| {
                service.refresh(cx).detach_and_log_err(cx);
            })?;
            Ok(())
        })
    }

    /// First-run bootstrap: start the agent, open the worktree, scan it, import
    /// a toolchain (when configured), import the detected project, and activate
    /// it — all in one awaited chain so there is no startup race.
    pub fn bootstrap(&mut self, root_path: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let settings = self.settings.clone();
        let executor = cx.background_executor().clone();
        let http = self.http.clone();
        cx.spawn(async move |this, cx| {
            let client = match this
                .read_with(cx, |service, _| service.client.clone())
                .ok()
                .flatten()
            {
                Some(client) => client,
                None => {
                    let config = AgentLaunchConfig {
                        agent_path: PathBuf::from(&settings.agent_path)
                            .canonicalize()
                            .unwrap_or_else(|_| {
                                PathBuf::from(r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe")
                            }),
                        data_dir: if settings.data_dir.is_empty() {
                            dirs::state_dir()
                                .or_else(dirs::data_local_dir)
                                .unwrap_or_else(std::env::temp_dir)
                                .join("kairo-zed")
                        } else {
                            PathBuf::from(&settings.data_dir)
                        },
                        bundled_dir: if settings.bundled_dir.is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(&settings.bundled_dir))
                        },
                        bind_address: "127.0.0.1".to_owned(),
                        log_level: "info".to_owned(),
                        secret: std::env::var("KAIRO_LOCAL_SECRET")
                            .ok()
                            .filter(|value| !value.is_empty()),
                        port: 0,
                    };
                    let agent = spawn_agent(executor, config)
                        .await
                        .context("failed to start Kairo runtime agent")?;
                    let base_url = agent.base_url();
                    let secret = std::env::var("KAIRO_LOCAL_SECRET")
                        .ok()
                        .filter(|value| !value.is_empty());
                    let client = KairoClient::new(http, base_url.clone(), secret);
                    client.health().await.context("Kairo agent is unhealthy")?;
                    this.update(cx, |service, cx| {
                        service.agent = Some(agent);
                        service.client = Some(client.clone());
                        service.status_message = format!("Kairo agent connected at {base_url}");
                        cx.notify();
                    })?;
                    client
                }
            };

            let workspace = client
                .open_workspace(&root_path)
                .await
                .context("failed to open Kairo workspace")?;
            let detection = client
                .scan_project(&workspace.id, &root_path)
                .await
                .context("failed to scan the legacy project layout")?;

            if !settings.compiler_java_home.is_empty() {
                match client
                    .import_toolchain(&settings.compiler_java_home, "Project compiler JDK")
                    .await
                {
                    Ok(_) => {}
                    Err(error) => log::warn!("Kairo toolchain import skipped: {error:#}"),
                }
            }

            let layout = detection
                .detected
                .first()
                .context("no legacy Java Web project detected in this folder")?;
            let name = std::path::Path::new(&layout.root_path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "legacy-project".to_owned());
            let payload = serde_json::json!({
                "workspaceId": workspace.id,
                "name": name,
                "rootPath": layout.root_path,
                "sourceDirs": layout.layout.src,
                "webappDir": layout.layout.web_root,
                "outputDir": "build/classes",
                "buildFile": layout.layout.build_xml,
                "sourceLevel": settings.source_level,
                "targetLevel": settings.target_level,
                "encoding": settings.encoding,
                "buildTool": layout.build_system,
                "contextPath": "/",
            });
            let project = client
                .import_project(payload)
                .await
                .context("failed to import the detected project")?;
            let project_id = project.id.clone();
            let workspace_id = workspace.id.clone();
            let detected = detection.detected;

            this.update(cx, |service, cx| {
                service.active_workspace_id = Some(workspace_id);
                service.detected = detected;
                if !service.projects.iter().any(|item| item.id == project_id) {
                    service.projects.push(project);
                }
                service.active_project_id = Some(project_id.clone());
                service.worktree_root = Some(root_path.clone());
                service.last_error = None;
                service.status_message = format!("Kairo project {project_id} is ready");
                service.activate_project(project_id, cx);
                cx.notify();
            })?;
            Ok(())
        })
    }

    /// Cycle to the next detected project. A real selection operation — not a
    /// status refresh.
    pub fn select_next_project(&mut self, cx: &mut Context<Self>) {
        if self.projects.is_empty() {
            self.last_error = Some("no Kairo projects available; run Kairo: Scan Project first".to_owned());
            cx.notify();
            return;
        }
        let current = self.active_project_id.as_deref();
        let next = self
            .projects
            .iter()
            .position(|project| Some(project.id.as_str()) == current)
            .map(|index| (index + 1) % self.projects.len())
            .unwrap_or(0);
        let project_id = self.projects[next].id.clone();
        self.activate_project(project_id, cx);
    }

    /// Drop the existing client and re-establish the agent connection.
    pub fn reconnect_agent(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.client = None;
        self.agent = None;
        self.start_agent(cx)
    }

    /// Refresh catalogs. With exactly one project it becomes the active one —
    /// this is what makes Compile/Deploy/Start work out of the box.
    pub fn refresh(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            let result = async {
                let workspaces = client.list_workspaces().await?;
                let projects = client.list_projects().await?;
                let toolchains = client.list_toolchains().await?;
                let mut servers = Vec::new();
                for workspace in &workspaces {
                    match client.get_servers_for_workspace(&workspace.id).await {
                        Ok(list) => servers.extend(list),
                        Err(error) => {
                            log::warn!("failed to list Kairo servers for {}: {error:#}", workspace.id)
                        }
                    }
                }
                Ok::<_, anyhow::Error>((workspaces, projects, toolchains, servers))
            }
            .await;

            match result {
                Ok((workspaces, projects, toolchains, servers)) => {
                    this.update(cx, |service, cx| {
                        service.workspaces = workspaces;
                        service.projects = projects;
                        service.toolchains = toolchains;
                        service.servers = servers;
                        service.last_error = None;
                        if service.active_project_id.is_none() && service.projects.len() == 1 {
                            let id = service.projects[0].id.clone();
                            service.activate_project(id, cx);
                        }
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Select a project as active and push the three-toolchain settings into
    /// its registered config (the agent derives build levels from the project).
    pub fn activate_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        self.active_project_id = Some(project_id.clone());
        self.last_error = None;
        self.status_message = format!("Active project: {project_id}");
        cx.notify();

        let Some(client) = self.client.clone() else {
            return;
        };
        let Some(project) = self.projects.iter().find(|item| item.id == project_id).cloned() else {
            return;
        };
        let workspace_id = self.active_workspace_id.clone().unwrap_or_default();
        let mut config = serde_json::to_value(&project).unwrap_or_default();
        if let Some(object) = config.as_object_mut() {
            object.insert("sourceLevel".into(), serde_json::json!(self.settings.source_level));
            object.insert("targetLevel".into(), serde_json::json!(self.settings.target_level));
            object.insert("encoding".into(), serde_json::json!(self.settings.encoding));
        }
        cx.spawn(async move |this, cx| {
            let update = client.update_project(&project_id, &config).await;
            let prepare = match update {
                Ok(_) => client.prepare_jdtls(&workspace_id).await.map(|_| ()),
                Err(error) => Err(error),
            };
            if let Err(error) = prepare {
                let error = error.context("failed to apply Kairo project settings");
                this.update(cx, |service, cx| service.set_error(&error, cx))
                    .log_err();
            } else {
                let descriptor = client
                    .launch_descriptor(&workspace_id, &project_id)
                    .await
                    .log_err();
                this.update(cx, |service, cx| {
                    service.jdtls_descriptor = descriptor;
                    service.write_descriptor(cx);
                    cx.notify();
                })
                .log_err();
            }
        })
        .detach();
    }

    /// Write a descriptor into the worktree so the WASM extension (which can
    /// read worktree files but not host env/FS) can configure JDT LS and the
    /// stdio JDI bridge automatically.
    fn write_descriptor(&self, cx: &mut Context<Self>) {
        let Some(root) = self.worktree_root.clone() else {
            return;
        };
        let dir = PathBuf::from(&root).join(".zed");
        let path = dir.join("kairo-integration.json");
        let jar = self
            .bundled_dir()
            .map(|dir| {
                dir.join("kairo-jdi-bridge.jar")
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default();
        let jdtls = self.jdtls_descriptor.as_ref();
        let descriptor = serde_json::json!({
            "schemaVersion": 1,
            "agentUrl": self.agent_base_url(),
            "encoding": self.settings.encoding,
            "jdtls": {
                "command": jdtls.map(|d| d.command.clone()).unwrap_or_default(),
                "args": jdtls.map(|d| d.args.clone()).unwrap_or_default(),
                "env": jdtls.map(|d| d.env_allowlist.clone()).unwrap_or_default(),
            },
            "debug": {
                "jarPath": jar,
                "host": "127.0.0.1",
                "jdwpPort": self.jdwp_port,
            }
        });
        cx.spawn(async move |_this, _cx| {
            smol::fs::create_dir_all(&dir).await?;
            smol::fs::write(&path, serde_json::to_vec_pretty(&descriptor)?)
                .await
                .with_context(|| format!("writing {}", path.display()))?;
            Ok::<(), anyhow::Error>(())
        })
        .detach_and_log_err(cx);
    }

    /// Open a workspace and scan it for a legacy project layout.
    pub fn open_workspace(&mut self, root_path: impl Into<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let root_path = root_path.into();
        self.worktree_root = Some(root_path.clone());
        cx.spawn(async move |this, cx| {
            match client.open_workspace(&root_path).await {
                Ok(workspace) => {
                    let id = workspace.id.clone();
                    this.update(cx, |service, cx| {
                        service.active_workspace_id = Some(id.clone());
                        service.workspaces.push(workspace);
                        cx.notify();
                    })?;
                    this.update(cx, |service, cx| {
                        service.scan_workspace(id, root_path.clone(), cx)
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    pub fn scan_workspace(
        &mut self,
        workspace_id: String,
        root_path: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        self.worktree_root = Some(root_path.clone());
        cx.spawn(async move |this, cx| {
            match client.scan_project(&workspace_id, &root_path).await {
                Ok(detection) => {
                    this.update(cx, |service, cx| {
                        service.detected = detection.detected;
                        service.status_message = format!(
                            "Detected {} legacy project layout(s)",
                            service.detected.len()
                        );
                        service.last_error = None;
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Compile, then poll to a terminal state so diagnostics and fail-closed
    /// toolchain errors actually reach the user.
    pub fn compile(&mut self, clean: bool, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.pending_deploy = false;
        self.start_build(clean, cx)
    }

    pub fn build_and_deploy(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.pending_deploy = true;
        self.start_build(false, cx)
    }

    fn start_build(&mut self, clean: bool, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return self.unavailable("no active Kairo project — run Kairo: Scan Project", cx);
        };
        cx.spawn(async move |this, cx| {
            let request = BuildRequest {
                project_id,
                clean,
                intent: Some("manual".to_owned()),
                selected_files: Vec::new(),
            };
            match client.start_build(&request).await {
                Ok(build) => {
                    let id = build.id.clone();
                    this.update(cx, |service, cx| {
                        service.builds.insert(0, build.clone());
                        service.status_message = format!("Build {id} is {}", build.state);
                        service.last_error = None;
                        service.start_build_polling(id, cx);
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    fn start_build_polling(&mut self, build_id: String, cx: &mut Context<Self>) {
        let executor = cx.background_executor().clone();
        self.build_task = Some(cx.spawn(async move |this, cx| {
            for _ in 0..240 {
                executor.timer(Duration::from_millis(500)).await;
                let Some(client) = this
                    .read_with(cx, |service, _| service.client.clone())
                    .ok()
                else {
                    return;
                };
                let Ok(build) = client.build(&build_id).await else { continue };
                let terminal = !matches!(build.state.as_str(), "queued" | "running");
                let should_deploy = build.state == "success";
                let pending = this
                    .read_with(cx, |service, _| service.pending_deploy)
                    .unwrap_or(false);
                let error = if build.state == "failure" {
                    build
                        .diagnostics
                        .iter()
                        .find(|diagnostic| !diagnostic.code.is_empty())
                        .map(|diagnostic| diagnostic.code.clone())
                        .unwrap_or_else(|| "compile_failed".to_owned())
                } else {
                    String::new()
                };
                let failed = is_toolchain_incompatible_code(&error);
                let result = this.update(cx, |service, cx| {
                    if let Some(existing) = service
                        .builds
                        .iter_mut()
                        .find(|item| item.id == build_id)
                    {
                        *existing = build.clone();
                    }
                    service.status_message = format!(
                        "Build {} is {}{}",
                        build.id,
                        build.state,
                        if build.state == "success" {
                            format!(" ({} files)", build.files_compiled)
                        } else {
                            String::new()
                        }
                    );
                    if !error.is_empty() {
                        service.last_error = Some(if failed {
                            format!(
                                "{error}: a real JDK 6 is required for source level 1.6; \
                                 importing a modern JDK is not a substitute"
                            )
                        } else {
                            format!("{error}: {}", build.output)
                        });
                    }
                    if should_deploy && pending {
                        service.pending_deploy = false;
                        service.deploy(cx).detach_and_log_err(cx);
                    }
                    cx.notify();
                });
                if result.is_err() || terminal {
                    return;
                }
            }
        }));
    }

    pub fn deploy(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return self.unavailable("no active Kairo project — run Kairo: Scan Project", cx);
        };
        let Some(build_id) = self
            .builds
            .iter()
            .find(|build| build.state == "success")
            .map(|build| build.id.clone())
        else {
            return self.unavailable("no successful Kairo build to deploy — run Compile first", cx);
        };
        cx.spawn(async move |this, cx| {
            let request = DeployRequest {
                project_id,
                build_id,
                scope: "incremental".to_owned(),
            };
            match client.deploy(&request).await {
                Ok(result) => {
                    let mode = result.hot_reload_mode.clone();
                    this.update(cx, |service, cx| {
                        service.hot_reload_mode = mode.clone();
                        service.status_message = format!(
                            "Deployment {} is {} ({})",
                            result.id, result.state, mode
                        );
                        service.last_error = None;
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Hot deploy with honest status: Java changes go through incremental
    /// compile, everything else through static sync. `restart_required` is
    /// never reported as `synced`.
    pub fn hot_deploy(&mut self, java_file: Option<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
        if !self.is_running() {
            return self.unavailable("Kairo runtime agent is not running", cx);
        }
        if let Some(file) = java_file {
            // The agent's /jvm/compile* endpoints reject native Windows paths
            // (verified on this machine). A real full build is the reliable
            // path; Tomcat's reloadable=false still means restart_required.
            self.hot_reload_mode = format!("compileOnly → {file}; restart_required");
            cx.notify();
            self.start_build(false, cx)
        } else {
            self.deploy(cx)
        }
    }

    /// Verify the Java 6 / Tomcat 6 / JDWP gate before offering debug.
    pub fn check_debug_gate(&mut self, cx: &mut Context<Self>) -> Task<Result<DebugAdapterStatus>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            match client.debug_adapter_status().await {
                Ok(status) => {
                    this.update(cx, |service, cx| {
                        service.debug_gate = Some(status.clone());
                        cx.notify();
                    })?;
                    Ok(status)
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Start Tomcat, optionally with JDWP. The stdio JDI bridge is spawned by
    /// Zed's DAP adapter using the worktree descriptor.
    pub fn start_server(&mut self, debug: bool, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return self.unavailable("no active Kairo project — run Kairo: Scan Project", cx);
        };
        let request = StartServerRequest {
            project_id,
            debug,
            java_home: self.settings.tomcat_java_home.clone(),
            context_path: String::new(),
            http_port: self.settings.tomcat_http_port,
            debug_port: self.settings.debug_port,
            jvm_options: Vec::new(),
        };
        let settings = self.settings.clone();
        cx.spawn(async move |this, cx| {
            if debug {
                let gate = client.debug_adapter_status().await.context(
                    "Java debug adapter is unavailable; check the JDI host JDK before debugging",
                )?;
                if !gate.available {
                    let message = format!(
                        "Java 6 debug gate failed: {} (host JDK: {})",
                        gate.message, gate.host_jdk
                    );
                    this.update(cx, |service, cx| {
                        service.debug_gate = Some(gate);
                        service.last_error = Some(message.clone());
                        cx.notify();
                    })?;
                    return Err(anyhow!(message));
                }
            }
            match client.start_server(&request).await {
                Ok(server) => {
                    let server_id = server.id.clone();
                    let debug_port = server.ports.debug;
                    this.update(cx, |service, cx| {
                        if debug && debug_port != 0 {
                            service.jdwp_port = debug_port;
                        }
                        service.write_descriptor(cx);
                        service.servers.push(server.clone());
                        service.debug_gate = debug.then_some(DebugAdapterStatus {
                            available: true,
                            host_jdk: settings.language_server_java_home.clone(),
                            bridge_path: String::new(),
                            message: String::new(),
                        });
                        service.status_message =
                            format!("Server {} is {}", server.id, server.state);
                        service.start_log_polling(server_id.clone(), cx);
                        cx.notify();
                    })?;
                    if debug {
                        this.update(cx, |service, cx| {
                            service.status_message = format!(
                                "JDWP is listening on 127.0.0.1:{}; start the Zed Java debug session",
                                if service.jdwp_port == 0 { settings.debug_port } else { service.jdwp_port }
                            );
                            cx.notify();
                        })?;
                    }
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    pub fn stop_server(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            match client.stop_server(&server_id).await {
                Ok(server) => {
                    this.update(cx, |service, cx| {
                        if let Some(existing) = service
                            .servers
                            .iter_mut()
                            .find(|candidate| candidate.id == server_id)
                        {
                            *existing = server.clone();
                        }
                        service.poll_task = None;
                        service.hot_reload_mode.clear();
                        service.status_message = format!("Server {} is {}", server.id, server.state);
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    pub fn restart_server(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            match client.restart_server(&server_id).await {
                Ok(server) => {
                    this.update(cx, |service, cx| {
                        if let Some(existing) = service
                            .servers
                            .iter_mut()
                            .find(|candidate| candidate.id == server_id)
                        {
                            *existing = server.clone();
                        }
                        service.status_message = format!("Server {} is {}", server.id, server.state);
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    pub fn reload_server(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        // The agent's verified context-reload touches <projectRoot>/WEB-INF/web.xml.
        // 2008-era layouts keep it under WebRoot/WEB-INF, so mirror it transparently
        // before asking for a reload.
        let web_xml = this
            .read_with(cx, |service, _| {
                service
                    .projects
                    .iter()
                    .find(|project| Some(project.id.as_str()) == service.active_project_id.as_deref())
                    .map(|project| {
                        let root = PathBuf::from(&project.root);
                        let webapp = PathBuf::from(&project.webapp_dir);
                        let source = if webapp.is_absolute() {
                            webapp
                        } else {
                            root.join(webapp)
                        }
                        .join("WEB-INF")
                        .join("web.xml");
                        (source, root.join("WEB-INF").join("web.xml"))
                    })
            })
            .ok()
            .flatten();
        cx.spawn(async move |this, cx| {
            if let Some((source, target)) = web_xml {
                if !target.exists()
                    && let Ok(contents) = smol::fs::read(&source).await
                {
                    if let Some(parent) = target.parent()
                        && smol::fs::create_dir_all(parent).await.is_ok()
                    {
                        if let Err(error) = smol::fs::write(&target, contents).await {
                            log::warn!("failed to mirror legacy WEB-INF/web.xml: {error}");
                        }
                    }
                } else if let Ok(contents) = smol::fs::read(&target).await {
                    // The agent's verified reload touches web.xml to detect a
                    // real change; refresh its mtime before asking for reload.
                    if let Err(error) = smol::fs::write(&target, contents).await {
                        log::warn!("failed to touch WEB-INF/web.xml: {error}");
                    }
                }
            }
            match client.reload_server(&server_id).await {
                Ok(result) => {
                    this.update(cx, |service, cx| {
                        service.hot_reload_mode = "contextReload".to_owned();
                        service.status_message = format!("Context reload: {result}");
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    pub fn load_logs(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            match client.server_logs(&server_id, 200).await {
                Ok(logs) => {
                    this.update(cx, |service, cx| {
                        service.logs = logs;
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Poll the agent's bounded log tail. The task lives in a field so it is
    /// cancelled with the service; transient errors do not stop polling.
    pub fn start_log_polling(&mut self, server_id: String, cx: &mut Context<Self>) {
        let executor = cx.background_executor().clone();
        self.poll_task = Some(cx.spawn(async move |this, cx| {
            loop {
                executor.timer(Duration::from_secs(2)).await;
                let Some(client) = this
                    .read_with(cx, |service, _| service.client.clone())
                    .ok()
                else {
                    return;
                };
                if let Ok(logs) = client.server_logs(&server_id, 200).await {
                    if this
                        .update(cx, |service, cx| {
                            service.logs = logs;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }));
    }

    /// Detect the on-disk encoding of a file and remember it for the panel.
    pub fn detect_encoding(&mut self, file: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            match client.detect_encoding(&file).await {
                Ok(detection) => {
                    this.update(cx, |service, cx| {
                        service.encoding = Some(detection);
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Save the user's EDITED buffer text with byte-faithful encoding.
    ///
    /// Zed buffers are UTF-8. We stage the text in a temp file inside the same
    /// (sandboxed) directory, let the agent strictly decode/recode it to the
    /// project encoding, then atomically replace the target file. This avoids
    /// both UTF-8 corruption and the older bug of recoding stale on-disk bytes.
    pub fn save_buffer_with_encoding(
        &mut self,
        file: String,
        text: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let to = self.settings.encoding.clone();
        cx.spawn(async move |this, cx| {
            let target = PathBuf::from(&file);
            let parent = target
                .parent()
                .context("the file has no parent directory")?;
            let temp = parent.join(format!(
                ".{}.kairo-zed-tmp",
                target
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "buffer".to_owned())
            ));

            smol::fs::write(&temp, text.as_bytes())
                .await
                .with_context(|| format!("staging {}", temp.display()))?;
            let request = RecodeRequest {
                file: temp.to_string_lossy().into_owned(),
                from: "utf-8".to_owned(),
                to: to.clone(),
                eol: None,
            };
            let recode = client.recode(&request).await;
            if let Err(error) = recode {
                if let Err(cleanup) = smol::fs::remove_file(&temp).await {
                    log::warn!("failed to remove Kairo save temp file: {cleanup}");
                }
                this.update(cx, |service, cx| service.set_error(&error, cx))
                    .log_err();
                return Err(error);
            }
            let bytes = smol::fs::read(&temp).await?;
            smol::fs::write(&target, &bytes)
                .await
                .with_context(|| format!("saving {}", target.display()))?;
            if let Err(cleanup) = smol::fs::remove_file(&temp).await {
                log::warn!("failed to remove Kairo save temp file: {cleanup}");
            }

            this.update(cx, |service, cx| {
                service.status_message = format!("Saved {} as {} ({} bytes, strict recode)", file, to, bytes.len());
                service.last_error = None;
                cx.notify();
            })?;
            Ok(())
        })
    }

    /// Detect a file's real encoding, then atomically recode it to the
    /// configured project encoding. This is the user-facing "Save with
    /// encoding" flow: no lossy UTF-8 conversion happens in Zed.
    pub fn save_active_file_with_encoding(&mut self, file: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let to = self.settings.encoding.clone();
        cx.spawn(async move |this, cx| {
            let detection = client
                .detect_encoding(&file)
                .await
                .context("failed to detect the file's on-disk encoding")?;
            let from = if detection.encoding.is_empty() {
                "utf-8".to_owned()
            } else {
                detection.encoding.clone()
            };
            let request = RecodeRequest {
                file: file.clone(),
                from,
                to: to.clone(),
                eol: None,
            };
            match client.recode(&request).await {
                Ok(_) => {
                    this.update(cx, |service, cx| {
                        service.encoding = Some(detection);
                        service.status_message = format!("Saved {file} as {to} (atomic recode)");
                        service.last_error = None;
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Byte-faithful "Save with encoding": the agent decodes strictly, checks
    /// representability, and rewrites the file atomically. No lossy UTF-8
    /// conversion happens in Zed.
    pub fn save_with_encoding(
        &mut self,
        file: String,
        from: String,
        to: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        cx.spawn(async move |this, cx| {
            let request = RecodeRequest {
                file: file.clone(),
                from: from.clone(),
                to: to.clone(),
                eol: None,
            };
            match client.recode(&request).await {
                Ok(_) => {
                    this.update(cx, |service, cx| {
                        service.status_message = format!("Saved {file} as {to} (atomic recode)");
                        service.last_error = None;
                        cx.notify();
                    })?;
                    Ok(())
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }

    /// Encoding-aware search. Zed's native search is UTF-8 only, so GBK
    /// projects must go through the agent.
    pub fn search(&mut self, query: String, cx: &mut Context<Self>) -> Task<Result<serde_json::Value>> {
        let Ok(client) = self.client() else {
            return self.unavailable("Kairo runtime agent is not running", cx);
        };
        let encoding = self.settings.encoding.clone();
        cx.spawn(async move |this, cx| {
            let payload = serde_json::json!({
                "query": query,
                "isRegex": false,
                "caseSensitive": false,
                "wholeWord": false,
                "maxResults": 10000,
                "projectEncoding": encoding,
            });
            match client.search(payload).await {
                Ok(result) => {
                    this.update(cx, |service, cx| {
                        service.status_message = "Kairo encoding-aware search completed".to_owned();
                        cx.notify();
                    })?;
                    Ok(result)
                }
                Err(error) => {
                    this.update(cx, |service, cx| service.set_error(&error, cx))?;
                    Err(error)
                }
            }
        })
    }
}

fn is_toolchain_incompatible_code(code: &str) -> bool {
    code == crate::domain::ErrorCode::TOOLCHAIN_INCOMPATIBLE
        || code == "toolchain_missing"
        || code == "unsupported_jdk_target"
}

/// Initialize the global service. Called once from `kairo_core::init`.
pub fn init_global(cx: &mut App) {
    let http = global_http_client(cx);
    let settings = crate::settings::get(cx).clone();
    let service = cx.new(|_cx| KairoService::new(http, settings));
    cx.set_global(GlobalKairoService(service));
}
