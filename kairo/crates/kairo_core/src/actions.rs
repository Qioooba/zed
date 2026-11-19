//! Kairo commands. Action types are registered automatically by the `Action`
//! derive's inventory mechanism; keybindings are bound at runtime so no
//! upstream keymap file has to be edited.
//!
//! Default chords were checked against the three shipped keymaps; the earlier
//! `ctrl-alt-b/d/s/p/i/o` choices collided and were replaced.

use gpui::{App, KeyBinding, actions};

actions!(
    kairo,
    [
        /// Compile the active legacy project with its configured JDK/Ant.
        Compile,
        /// Clean and recompile the active project.
        CleanBuild,
        /// Compile, then deploy to the running server when the build succeeds.
        BuildAndDeploy,
        /// Deploy the latest successful build to the configured Tomcat context.
        Deploy,
        /// Synchronize changed files using the hot-reload decision logic.
        HotDeploy,
        /// Start Tomcat 6 for the active project.
        StartServer,
        /// Verify the Java 6 gate, start Tomcat with JDWP and attach the debugger.
        StartDebugServer,
        /// Stop the running Tomcat instance.
        StopServer,
        /// Restart the running Tomcat instance (new process generation).
        RestartServer,
        /// Trigger a verified Tomcat context reload.
        ReloadContext,
        /// Open the running application in the system browser.
        OpenApplication,
        /// Scan the workspace for a legacy Java Web project layout.
        ScanProject,
        /// Select which detected Kairo project is active.
        SelectProject,
        /// Save the active GBK/legacy file with byte-faithful encoding.
        SaveWithEncoding,
        /// Show or hide the Kairo control panel.
        TogglePanel,
        /// Refresh toolchain/server/build status from the agent.
        RefreshStatus,
        /// Reconnect to (or start) the runtime agent.
        ReconnectAgent,
        /// Query agent discovery information.
        Endpoints,
        /// Restart the runtime agent with port handoff.
        RestartAgent,
        /// Query operation/idempotency status.
        OperationStatus,
        /// Import a project through the legacy alias endpoint.
        LegacyImport,
        /// Unregister the active project.
        UnregisterProject,
        /// List run configurations.
        ListRunConfigurations,
        /// Save run configurations.
        SaveRunConfigurations,
        /// Launch a run configuration.
        LaunchRunConfiguration,
        /// Poll asynchronous launch jobs.
        LaunchJobs,
        /// Cancel the current build.
        CancelBuild,
        /// Run an arbitrary custom build command.
        CustomBuild,
        /// Poll custom build status.
        CustomBuildStatus,
        /// Analyze an Ant build.xml classpath.
        AnalyzeAntClasspath,
        /// Show deployment history.
        DeploymentHistory,
        /// Show one deployment.
        Deployment,
        /// Recover a crashed server.
        RecoverServer,
        /// List recoverable servers.
        RecoverableServers,
        /// Diagnose a TCP port.
        PortDiagnostics,
        /// Show JDT LS status.
        JdtlsStatus,
        /// Prepare the JDT LS distribution.
        PrepareJdtls,
        /// Connect an SSH JDWP tunnel.
        DebugTunnelConnect,
        /// Disconnect the SSH JDWP tunnel.
        DebugTunnelDisconnect,
        /// Provision the host JDK required by the JDI bridge.
        ProvisionDebugJdk,
        /// Run a Java main/test class.
        RunJava,
        /// Detect main/test methods in a Java file.
        DetectJava,
        /// Redefine a loaded class (JDWP hot swap).
        HotSwap,
        /// Detect a Maven project.
        MavenDetect,
        /// Show the Maven dependency tree.
        MavenDependencies,
        /// Run a Maven lifecycle task.
        MavenRun,
        /// Test an Oracle SQL connection.
        SqlTest,
        /// Show the agent audit log.
        Audit,
        /// Encoding-aware full-text search.
        SearchEncoded,
        /// Detect the active file's on-disk encoding.
        DetectFileEncoding,
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-alt-shift-j", Compile, None),
        KeyBinding::new("ctrl-alt-shift-m", Deploy, None),
        KeyBinding::new("ctrl-alt-shift-u", HotDeploy, None),
        KeyBinding::new("ctrl-alt-shift-v", StartServer, None),
        KeyBinding::new("ctrl-alt-shift-y", StartDebugServer, None),
        KeyBinding::new("ctrl-alt-shift-k", TogglePanel, None),
    ]);
}
