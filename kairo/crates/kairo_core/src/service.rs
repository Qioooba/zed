//! Application-level service that owns the Kairo agent and its derived state.

use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, Context, Entity, Global, Task};
use http_client::HttpClient;

use crate::{
    agent::{AgentHandle, AgentLaunchConfig, spawn_agent},
    client::{KairoClient, global_http_client},
    domain::{
        BuildRequest, BuildResult, DeployRequest, DeployResult, LogLine, Project, ServerResponse,
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
    active_workspace_id: Option<String>,
    active_project_id: Option<String>,
    status_message: String,
    last_error: Option<String>,
    agent_watch: Option<Task<()>>,
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
            active_workspace_id: None,
            active_project_id: None,
            status_message: "Kairo agent is not running".to_owned(),
            last_error: None,
            agent_watch: None,
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

    pub fn active_project_id(&self) -> Option<&str> {
        self.active_project_id.as_deref()
    }

    pub fn agent_base_url(&self) -> Option<String> {
        self.agent.as_ref().map(|agent| agent.base_url())
    }

    fn resolve_agent_path(&self) -> std::path::PathBuf {
        if !self.settings.agent_path.is_empty() {
            return std::path::PathBuf::from(&self.settings.agent_path);
        }
        if let Ok(path) = std::env::var("KAIRO_AGENT_PATH")
            && !path.is_empty()
        {
            return std::path::PathBuf::from(path);
        }
        // Default: the locally built agent in the companion Kairo IDE checkout.
        std::path::PathBuf::from(r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe")
    }

    fn resolve_data_dir(&self) -> std::path::PathBuf {
        if !self.settings.data_dir.is_empty() {
            return std::path::PathBuf::from(&self.settings.data_dir);
        }
        let base = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(std::env::temp_dir);
        base.join("kairo-zed")
    }

    /// Start the Go runtime agent and then load the initial catalogs.
    pub fn start_agent(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        if self.client.is_some() {
            return Task::ready(Ok(()));
        }

        let config = AgentLaunchConfig {
            agent_path: self.resolve_agent_path(),
            data_dir: self.resolve_data_dir(),
            bundled_dir: if self.settings.bundled_dir.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(&self.settings.bundled_dir))
            },
            bind_address: "127.0.0.1".to_owned(),
            log_level: "info".to_owned(),
            port: 0,
        };
        let executor = cx.background_executor().clone();
        let http = self.http.clone();

        cx.spawn(async move |this, cx| {
            let agent = spawn_agent(executor, config)
                .await
                .context("failed to start the Kairo runtime agent")?;
            let base_url = agent.base_url();
            let client = KairoClient::new(http, base_url.clone(), None);
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

            Ok(())
        })
    }

    /// Refresh workspaces, projects, toolchains, and servers from the agent.
    pub fn refresh(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        cx.spawn(async move |this, cx| {
            let workspaces = client.list_workspaces().await?;
            let projects = client.list_projects().await?;
            let toolchains = client.list_toolchains().await?;
            let mut servers = Vec::new();
            for workspace in &workspaces {
                if let Ok(server_list) = client.get_servers_for_workspace(&workspace.id).await {
                    servers.extend(server_list);
                }
            }
            this.update(cx, |service, cx| {
                service.workspaces = workspaces;
                service.projects = projects;
                service.toolchains = toolchains;
                service.servers = servers;
                service.last_error = None;
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn open_workspace(
        &mut self,
        root_path: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        let root_path = root_path.into();
        cx.spawn(async move |this, cx| {
            let workspace = client.open_workspace(root_path).await?;
            this.update(cx, |service, cx| {
                service.active_workspace_id = Some(workspace.id.clone());
                service.workspaces.push(workspace);
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn compile(&mut self, clean: bool, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return Task::ready(Err(anyhow!("no active Kairo project")));
        };
        cx.spawn(async move |this, cx| {
            let request = BuildRequest {
                project_id: project_id.clone(),
                clean,
                intent: Some("manual".to_owned()),
                selected_files: Vec::new(),
            };
            let build = client.start_build(&request).await?;
            this.update(cx, |service, cx| {
                service.builds.insert(0, build.clone());
                service.status_message = format!("Build {} is {}", build.id, build.status);
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn deploy(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return Task::ready(Err(anyhow!("no active Kairo project")));
        };
        let build_id = self
            .builds
            .first()
            .filter(|build| build.status == "succeeded")
            .map(|build| build.id.clone())
            .unwrap_or_default();
        cx.spawn(async move |this, cx| {
            let request = DeployRequest {
                project_id,
                build_id,
                scope: "incremental".to_owned(),
            };
            let result: DeployResult = client.deploy(&request).await?;
            this.update(cx, |service, cx| {
                service.status_message = format!("Deployment {} is {}", result.id, result.status);
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn start_server(&mut self, debug: bool, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        let Some(project_id) = self.active_project_id.clone() else {
            return Task::ready(Err(anyhow!("no active Kairo project")));
        };
        let debug_port = self.settings.debug_port;
        cx.spawn(async move |this, cx| {
            let mut request = StartServerRequest {
                project_id,
                debug,
                configuration: None,
            };
            if debug {
                request.configuration = Some(format!("debugPort={debug_port}"));
            }
            let server = client.start_server(&request).await?;
            this.update(cx, |service, cx| {
                service.servers.push(server.clone());
                service.status_message = format!("Server {} is {}", server.id, server.state);
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn stop_server(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        cx.spawn(async move |this, cx| {
            let server = client.stop_server(&server_id).await?;
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
        })
    }

    pub fn load_logs(&mut self, server_id: String, cx: &mut Context<Self>) -> Task<Result<()>> {
        let Ok(client) = self.client() else {
            return Task::ready(Err(anyhow!("Kairo runtime agent is not running")));
        };
        cx.spawn(async move |this, cx| {
            let logs = client.server_logs(&server_id, 200).await?;
            this.update(cx, |service, cx| {
                service.logs = logs;
                cx.notify();
            })?;
            Ok(())
        })
    }

    pub fn reload(&self, cx: &App) -> Entity<Self> {
        Self::global(cx)
    }
}

/// Initialize the global service. Called once from `kairo_core::init`.
pub fn init_global(cx: &mut App) {
    let http = global_http_client(cx);
    let settings = crate::settings::get(cx).clone();
    let service = cx.new(|_cx| KairoService::new(http, settings));
    cx.set_global(GlobalKairoService(service));
}
