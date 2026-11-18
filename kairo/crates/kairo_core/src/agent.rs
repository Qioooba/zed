//! Lifecycle of the Kairo runtime-agent process.
//!
//! The agent is the single authority for all legacy-toolchain OS effects
//! (Kairo ADR-0002). Zed starts exactly one agent per workspace session, on a
//! loopback-only ephemeral port, and discovers the chosen port from the atomic
//! `<dataDir>/agent-state.json` file the agent writes when it becomes ready.

use std::{path::PathBuf, process::Stdio, time::Duration};

use anyhow::{Context as _, Result, bail};
use gpui::BackgroundExecutor;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct AgentLaunchConfig {
    pub agent_path: PathBuf,
    pub data_dir: PathBuf,
    pub bundled_dir: Option<PathBuf>,
    pub bind_address: String,
    pub log_level: String,
    /// Ephemeral port (0) is strongly preferred so multiple workspaces and
    /// multiple Zed windows cannot collide with each other or with Kairo IDE.
    pub port: u16,
}

impl Default for AgentLaunchConfig {
    fn default() -> Self {
        Self {
            agent_path: PathBuf::from("kairo-runtime"),
            data_dir: PathBuf::from("."),
            bundled_dir: None,
            bind_address: "127.0.0.1".to_owned(),
            log_level: "info".to_owned(),
            port: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentStateFile {
    port: u16,
    status: String,
    #[serde(default)]
    instance_id: String,
    #[serde(default)]
    error: String,
}

/// A running agent process. Dropping it terminates the whole process tree via
/// `util::process::Child`'s job object (Windows) / process group (Unix).
pub struct AgentHandle {
    child: util::process::Child,
    port: u16,
    instance_id: String,
    pid: u32,
}

impl AgentHandle {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn events_url(&self) -> String {
        format!("ws://127.0.0.1:{}/api/v1/events", self.port)
    }
}

/// A running `kairo-jdi-bridge.jar` process: DAP over loopback TCP ↔ JDWP.
pub struct BridgeHandle {
    child: util::process::Child,
    port: u16,
}

impl BridgeHandle {
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Launch the JDI bridge that Zed's DAP adapter attaches to. The bridge has its
/// own bounded JDWP attach retry, so we only need to wait for its TCP listener.
pub async fn spawn_debug_bridge(
    executor: BackgroundExecutor,
    java_home: Option<std::path::PathBuf>,
    jar_path: std::path::PathBuf,
    host: &str,
    port: u16,
) -> Result<BridgeHandle> {
    if !jar_path.exists() {
        bail!("Kairo JDI bridge not found at {}", jar_path.display());
    }
    let java_binary = java_home
        .map(|home| home.join("bin").join("java"))
        .unwrap_or_else(|| std::path::PathBuf::from("java"));
    let mut command = util::command::new_std_command(java_binary);
    command.arg("-jar").arg(&jar_path).arg(host).arg(port.to_string());

    let child = util::process::Child::spawn(command, Stdio::null(), Stdio::piped(), Stdio::piped())
        .context("failed to start the Kairo JDI bridge")?;

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if smol::net::TcpStream::connect((host, port)).await.is_ok() {
            return Ok(BridgeHandle { child, port });
        }
        if std::time::Instant::now() >= deadline {
            bail!("Kairo JDI bridge did not open its DAP port {port}");
        }
        executor.timer(Duration::from_millis(200)).await;
    }
}

/// Start the agent and wait until it reports `status: "ready"`.
pub async fn spawn_agent(
    executor: BackgroundExecutor,
    config: AgentLaunchConfig,
) -> Result<AgentHandle> {
    if config.bind_address != "127.0.0.1" && config.bind_address != "localhost" {
        bail!(
            "Kairo agent must bind to loopback only; refusing {}",
            config.bind_address
        );
    }

    smol::fs::create_dir_all(&config.data_dir)
        .await
        .with_context(|| format!("creating Kairo data dir {}", config.data_dir.display()))?;

    let state_path = config.data_dir.join("agent-state.json");
    // A stale state file from a previous agent would make us connect to a dead
    // port. Remove it before spawning (the agent recreates it atomically).
    if state_path.exists() {
        smol::fs::remove_file(&state_path)
            .await
            .with_context(|| format!("removing stale {}", state_path.display()))?;
    }

    let mut command = util::command::new_std_command(&config.agent_path);
    command
        .arg("-port")
        .arg(config.port.to_string())
        .arg("-bind")
        .arg(&config.bind_address)
        .arg("-data-dir")
        .arg(&config.data_dir)
        .arg("-log-level")
        .arg(&config.log_level);
    if let Some(bundled_dir) = &config.bundled_dir {
        command.arg("-bundled-dir").arg(bundled_dir);
    }

    let child = util::process::Child::spawn(command, Stdio::null(), Stdio::null(), Stdio::piped())
        .context("failed to start Kairo runtime agent")?;
    let pid = child.id();

    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let ready_state;
    loop {
        if let Ok(contents) = smol::fs::read_to_string(&state_path).await
            && let Ok(state) = serde_json::from_str::<AgentStateFile>(&contents)
        {
            if state.status == "ready" && state.port != 0 {
                ready_state = state;
                break;
            }
            if state.status == "failed" {
                bail!("Kairo agent failed to start: {}", state.error);
            }
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "timed out waiting for Kairo agent state file at {}",
                state_path.display()
            );
        }
        executor.timer(Duration::from_millis(150)).await;
    }

    Ok(AgentHandle {
        child,
        port: ready_state.port,
        instance_id: ready_state.instance_id,
        pid,
    })
}
