//! Kairo commands. Action types are registered automatically by the `Action`
//! derive's inventory mechanism; keybindings are bound at runtime so no
//! upstream keymap file has to be edited.

use gpui::{App, KeyBinding, actions};

actions!(
    kairo,
    [
        /// Compile the active legacy project with its configured JDK/Ant.
        Compile,
        /// Clean and recompile the active project.
        CleanBuild,
        /// Compile, then deploy to the running server.
        BuildAndDeploy,
        /// Deploy the latest build to the configured Tomcat context.
        Deploy,
        /// Synchronize changed files using the hot-reload decision logic.
        HotDeploy,
        /// Start Tomcat 6 for the active project.
        StartServer,
        /// Start Tomcat 6 with JDWP enabled and attach the debugger.
        StartDebugServer,
        /// Stop the running Tomcat instance.
        StopServer,
        /// Restart the running Tomcat instance (new process generation).
        RestartServer,
        /// Open the running application in the system browser.
        OpenApplication,
        /// Scan the workspace for a legacy Java Web project layout.
        ScanProject,
        /// Show or hide the Kairo control panel.
        TogglePanel,
        /// Refresh toolchain/server/build status from the agent.
        RefreshStatus,
        /// Reconnect to the runtime agent.
        ReconnectAgent,
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-alt-b", Compile, None),
        KeyBinding::new("ctrl-alt-shift-b", CleanBuild, None),
        KeyBinding::new("ctrl-alt-d", Deploy, None),
        KeyBinding::new("ctrl-alt-r", HotDeploy, None),
        KeyBinding::new("ctrl-alt-s", StartServer, None),
        KeyBinding::new("ctrl-alt-shift-s", StopServer, None),
        KeyBinding::new("ctrl-alt-shift-d", StartDebugServer, None),
        KeyBinding::new("ctrl-alt-p", TogglePanel, None),
    ]);
}
