//! Zed extension contributing Java/JSP languages, JDT LS attachment, and the
//! Kairo JDI↔JDWP debug adapter. Heavy processes remain owned by the Kairo
//! runtime agent; this extension only resolves how Zed should talk to them.

use zed::settings::LspSettings;
use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugTaskDefinition, LanguageServerId,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, TcpArgumentsTemplate,
    Worktree,
    serde_json::{Value, json},
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
        // A user can always override the binary (e.g. point at the JDT LS
        // distribution that the Kairo agent installed under bundled/jdtls).
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

        let _ = language_server_id;
        // The Go agent writes the resolved launcher path to KAIRO_JDTLS_LAUNCHER
        // when it prepares the distribution. Fall back to a PATH lookup so a
        // manually installed JDT LS also works.
        let launcher =
            std::env::var("KAIRO_JDTLS_LAUNCHER").unwrap_or_else(|_| "jdtls".to_owned());
        Ok(zed::Command {
            command: launcher,
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
        _user_provided_debug_adapter_path: Option<String>,
        _worktree: &Worktree,
    ) -> zed::Result<DebugAdapterBinary> {
        let parsed: Value = serde_json::from_str(&config.config)
            .map_err(|error| format!("invalid Kairo debug config: {error}"))?;
        let port = parsed
            .get("port")
            .and_then(Value::as_u64)
            .unwrap_or(8000)
            .min(u16::MAX as u64) as u16;

        // `kairo-jdi-bridge.jar` is started by the Kairo panel/agent and speaks
        // DAP over a loopback TCP socket, so Zed attaches instead of spawning.
        let connection = zed::resolve_tcp_template(TcpArgumentsTemplate {
            port: Some(port),
            host: None,
            timeout: None,
        })?;

        Ok(DebugAdapterBinary {
            command: None,
            arguments: vec![],
            envs: vec![],
            cwd: None,
            connection: Some(connection),
            request_args: StartDebuggingRequestArguments {
                configuration: json!({
                    "request": "attach",
                    "type": "java",
                    "hostName": "127.0.0.1",
                    "port": port
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
