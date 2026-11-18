//! Zed extension contributing Java/JSP languages, JDT LS attachment, and the
//! Kairo JDI↔JDWP debug adapter. Heavy processes remain owned by the Kairo
//! runtime agent; this extension only resolves how Zed should talk to them.

use zed::settings::LspSettings;
use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugTaskDefinition, LanguageServerId,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
    serde_json::{self, Value, json},
};

struct KairoJavaExtension;

impl zed::Extension for KairoJavaExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        // The Kairo service writes .zed/kairo-integration.json into the worktree
        // with the exact launcher resolved from the agent's launch descriptor.
        // A WASM extension can read worktree files (but not host env vars).
        if let Ok(text) = worktree.read_text_file(".zed/kairo-integration.json")
            && let Ok(descriptor) = serde_json::from_str::<Value>(&text)
            && let Some(jdtls) = descriptor.get("jdtls")
        {
            let command = jdtls
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if !command.is_empty() {
                let args = jdtls
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|value| value.as_str().map(str::to_owned))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let env = jdtls
                    .get("env")
                    .and_then(Value::as_object)
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(|(key, value)| {
                                value
                                    .as_str()
                                    .map(|value| (key.clone(), value.to_owned()))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                return Ok(zed::Command { command, args, env });
            }
        }

        // An explicit user override always wins.
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        if let Some(binary) = settings.binary
            && let Some(path) = binary.path
        {
            return Ok(zed::Command {
                command: path,
                args: binary.arguments.unwrap_or_default(),
                env: binary
                    .env
                    .map(|env| env.into_iter().collect())
                    .unwrap_or_default(),
            });
        }

        // Last resort: a JDT LS installed on PATH.
        Ok(zed::Command {
            command: "jdtls".to_owned(),
            args: vec!["--stdio".to_owned()],
            env: vec![],
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        Ok(settings.initialization_options)
    }

    fn get_dap_binary(
        &mut self,
        _adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> zed::Result<DebugAdapterBinary> {
        let parsed: Value = serde_json::from_str(&config.config)
            .map_err(|error| format!("invalid Kairo debug config: {error}"))?;

        // Read the worktree descriptor written by kairo_core. It contains the
        // bridge jar path and the real JDWP port reported by the agent.
        let descriptor = worktree
            .read_text_file(".zed/kairo-integration.json")
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        let debug = descriptor.and_then(|value| value.get("debug").cloned());
        let jar_path = user_provided_debug_adapter_path.or_else(|| {
            parsed
                .get("jarPath")
                .and_then(Value::as_str)
                .map(str::to_owned)
        }).or_else(|| {
            debug
                .as_ref()
                .and_then(|value| value.get("jarPath"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        }).ok_or_else(|| {
            "Kairo JDI bridge not found: set dap.java.binary or run Kairo: Debug first".to_owned()
        })?;
        let port = parsed
            .get("port")
            .and_then(Value::as_u64)
            .or_else(|| {
                debug
                    .as_ref()
                    .and_then(|value| value.get("jdwpPort"))
                    .and_then(Value::as_u64)
            })
            .unwrap_or(8000)
            .min(u16::MAX as u64) as u16;
        let host = debug
            .as_ref()
            .and_then(|value| value.get("host"))
            .and_then(Value::as_str)
            .unwrap_or("127.0.0.1");
        let java = worktree.which("java").unwrap_or_else(|| "java".to_owned());

        // The bridge speaks Content-Length framed DAP OVER STDIO, so Zed must
        // spawn it and own the pipes — never TCP-attach to a separate process.
        Ok(DebugAdapterBinary {
            command: Some(java),
            arguments: vec![
                "-jar".to_owned(),
                jar_path,
                host.to_owned(),
                port.to_string(),
            ],
            envs: vec![],
            cwd: Some(worktree.root_path()),
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration: json!({
                    "request": "attach",
                    "type": "java",
                    "hostName": host,
                    "port": port,
                    "sourcePaths": [worktree.root_path()]
                })
                .to_string(),
                request: StartDebuggingRequestArgumentsRequest::Attach,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        _config: Value,
    ) -> zed::Result<StartDebuggingRequestArgumentsRequest> {
        Ok(StartDebuggingRequestArgumentsRequest::Attach)
    }
}

zed::register_extension!(KairoJavaExtension);
