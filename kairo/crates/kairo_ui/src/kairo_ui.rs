//! UI layer of the Kairo-for-Zed integration: the control panel and the
//! workspace actions that drive compile / deploy / server operations.

pub mod kairo_panel;

use gpui::{App, Context, Window};
use kairo_core::{actions, service::KairoService};
use workspace::Workspace;

pub use kairo_panel::KairoPanel;

/// Register Kairo workspace actions. The panel itself is loaded from
/// `initialize_panels` in `crates/zed/src/zed.rs` (one tagged upstream line).
pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, _window: &mut Window, cx: &mut Context<Workspace>| {
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::Compile,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.compile(false, cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::CleanBuild,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.compile(true, cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::BuildAndDeploy,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.compile(false, cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::Deploy,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.deploy(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::HotDeploy,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.deploy(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::StartServer,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.start_agent(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::StartDebugServer,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.start_server(true, cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::StopServer,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        let server_id = service
                            .servers()
                            .last()
                            .map(|server| server.id.clone());
                        if let Some(server_id) = server_id {
                            service.stop_server(server_id, cx).detach_and_log_err(cx);
                        }
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::RestartServer,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.refresh(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::RefreshStatus,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.refresh(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::ReconnectAgent,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.start_agent(cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |workspace: &mut Workspace,
                 _action: &actions::TogglePanel,
                 window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    workspace.toggle_panel_focus::<KairoPanel>(window, cx);
                },
            );
        },
    )
    .detach();
}
