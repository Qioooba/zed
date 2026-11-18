//! Kairo commands. Action types are registered automatically by the `Action`
//! derive's inventory mechanism; keybindings are bound at runtime so no
//! upstream keymap file has to be edited.
//!
//! Default chords were chosen from combinations that do not appear in Zed's
//! shipped keymaps (the previously used `ctrl-alt-b/d/s/p` all collided).

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
        /// Run an encoding-aware search (required for GBK projects).
        SearchEncoded,
        /// Show or hide the Kairo control panel.
        TogglePanel,
        /// Refresh toolchain/server/build status from the agent.
        RefreshStatus,
        /// Reconnect to (or start) the runtime agent.
        ReconnectAgent,
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-alt-shift-j", Compile, None),
        KeyBinding::new("ctrl-alt-shift-m", Deploy, None),
        KeyBinding::new("ctrl-alt-shift-i", HotDeploy, None),
        KeyBinding::new("ctrl-alt-shift-v", StartServer, None),
        KeyBinding::new("ctrl-alt-shift-o", StartDebugServer, None),
        KeyBinding::new("ctrl-alt-shift-k", TogglePanel, None),
    ]);
}
