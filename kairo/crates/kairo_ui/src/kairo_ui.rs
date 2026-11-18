//! UI layer of the Kairo-for-Zed integration: the control panel and the
//! workspace actions that drive compile / deploy / server operations.

pub mod kairo_panel;

use editor::Editor;
use gpui::{App, Context, Window, prelude::*};
use kairo_core::{actions, service::KairoService};
use workspace::Workspace;

pub use kairo_panel::KairoPanel;

/// Register every Kairo action. Each handler drives the real operation via
/// `KairoService`; none of them is a placeholder.
pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, _window: Option<&mut Window>, cx: &mut Context<Workspace>| {
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
                        service.build_and_deploy(cx).detach_and_log_err(cx);
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
                |workspace: &mut Workspace,
                 _action: &actions::HotDeploy,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let java_file = workspace
                        .active_item_as::<Editor>(cx)
                        .and_then(|editor| {
                            editor.read(cx).buffer().read(cx).as_singleton().and_then(|buffer| {
                                buffer
                                    .read(cx)
                                    .file()
                                    .and_then(|file| file.file_system_abs_path(cx))
                            })
                        })
                        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("java")))
                        .map(|path| path.to_string_lossy().into_owned());
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.hot_deploy(java_file, cx).detach_and_log_err(cx);
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
                        service.start_server(false, cx).detach_and_log_err(cx);
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
                        let server_id = service.servers().last().map(|server| server.id.clone());
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
                        let server_id = service.servers().last().map(|server| server.id.clone());
                        if let Some(server_id) = server_id {
                            service.restart_server(server_id, cx).detach_and_log_err(cx);
                        }
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::ReloadContext,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        let server_id = service.servers().last().map(|server| server.id.clone());
                        if let Some(server_id) = server_id {
                            service.reload_server(server_id, cx).detach_and_log_err(cx);
                        }
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::OpenApplication,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    if let Some(url) = service.read(cx).application_url() {
                        cx.open_url(&url);
                    }
                },
            );
            workspace.register_action(
                |workspace: &mut Workspace,
                 _action: &actions::ScanProject,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let Some(root) = workspace
                        .visible_worktrees(cx)
                        .next()
                        .map(|tree| tree.read(cx).abs_path().to_string_lossy().into_owned())
                    else {
                        return;
                    };
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service.bootstrap(root, cx).detach_and_log_err(cx);
                    });
                },
            );
            workspace.register_action(
                |_workspace: &mut Workspace,
                 _action: &actions::SelectProject,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| service.select_next_project(cx));
                },
            );
            workspace.register_action(
                |workspace: &mut Workspace,
                 _action: &actions::SaveWithEncoding,
                 _window: &mut Window,
                 cx: &mut Context<Workspace>| {
                    let Some(editor) = workspace.active_item_as::<Editor>(cx) else {
                        return;
                    };
                    let path = editor.read(cx).buffer().read(cx).as_singleton().and_then(|buffer| {
                        buffer
                            .read(cx)
                            .file()
                            .and_then(|file| file.file_system_abs_path(cx))
                    });
                    let Some(path) = path else { return };
                    let service = KairoService::global(cx);
                    service.update(cx, |service, cx| {
                        service
                            .save_active_file_with_encoding(path.to_string_lossy().into_owned(), cx)
                            .detach_and_log_err(cx);
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
                        service.reconnect_agent(cx).detach_and_log_err(cx);
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
