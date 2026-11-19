//! A dock panel that surfaces Kairo agent state, build/deploy results, and
//! paged Tomcat logs. Logs are deliberately bounded — Kairo's UI spec forbids
//! unbounded log views on the target 4 GB machines.

use editor::Editor;
use gpui::{
    App, AsyncWindowContext, Context, EventEmitter, FocusHandle, Focusable, SharedString,
    Subscription, Window, div, prelude::*, px,
};
use icons::IconName;
use kairo_core::{actions, service::KairoService};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

const MAX_VISIBLE_LOG_LINES: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum KairoView {
    #[default]
    Status,
    Events,
    Deployments,
    Maven,
    Sql,
    Tests,
    Svn,
}

pub struct KairoPanel {
    workspace: gpui::WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    active_view: KairoView,
}

impl KairoPanel {
    pub fn new(
        workspace: gpui::WeakEntity<Workspace>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let service = KairoService::global(cx);
        let subscriptions = vec![cx.observe(&service, |_, _, cx| cx.notify())];
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
            active_view: KairoView::default(),
        }
    }

    pub async fn load(
        workspace: gpui::WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<gpui::Entity<Self>> {
        let weak_workspace = workspace.clone();
        let panel = workspace.update_in(&mut cx, |_workspace, window, cx| {
            cx.new(|cx| KairoPanel::new(weak_workspace.clone(), window, cx))
        })?;
        Ok(panel)
    }

    fn view_button(
        &self,
        label: &'static str,
        view: KairoView,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .id(label)
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .on_click(cx.listener(move |panel, _event, _window, cx| {
                panel.active_view = view;
                cx.notify();
            }))
            .child(label)
    }

    fn action_button(
        &self,
        label: &'static str,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut KairoService, &mut Context<KairoService>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .on_click(cx.listener(move |_panel, _event, _window, cx| {
                let service = KairoService::global(cx);
                service.update(cx, |service, cx| handler(service, cx));
            }))
            .child(label)
    }
}

impl Focusable for KairoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for KairoPanel {}

impl Panel for KairoPanel {
    fn persistent_name() -> &'static str {
        "KairoPanel"
    }

    fn panel_key() -> &'static str {
        "kairo_panel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Bottom | DockPosition::Right)
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> gpui::Pixels {
        px(280.)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::TerminalAlt)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Kairo: legacy Java Web tools")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(actions::TogglePanel)
    }

    fn activation_priority(&self) -> u32 {
        100
    }

    fn enabled(&self, cx: &App) -> bool {
        self.workspace.read_with(cx, |_, _| true).is_ok()
    }
}

impl Render for KairoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let service = KairoService::global(cx);
        let state = service.read(cx);
        let settings = state.settings();
        let status = SharedString::from(state.status_message().to_owned());

        let toolchains = state
            .toolchains()
            .iter()
            .map(|toolchain| {
                format!(
                    "{} {} [{}] — {}",
                    toolchain.vendor,
                    toolchain.version,
                    toolchain.kind,
                    if toolchain.fingerprint.is_empty() {
                        "UNVERIFIED"
                    } else {
                        "fingerprinted"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let servers = state
            .servers()
            .iter()
            .map(|server| {
                format!(
                    "{}: {} (gen {}, http {})",
                    server.id, server.state, server.generation, server.ports.http
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let projects = state
            .projects()
            .iter()
            .map(|project| {
                let marker = if state.active_project_id() == Some(project.id.as_str()) {
                    "* "
                } else {
                    "  "
                };
                format!("{marker}{} ({})", project.name, project.id)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let logs = state
            .logs()
            .iter()
            .rev()
            .take(MAX_VISIBLE_LOG_LINES)
            .map(|line| line.line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let jdk6_status = if state
            .toolchains()
            .iter()
            .any(|toolchain| toolchain.version.starts_with("1.6"))
        {
            "AVAILABLE (real JDK 6 imported)"
        } else {
            "MISSING — 1.6 builds will fail closed"
        };
        let toolchain_slots = format!(
            "LS JRE: {}\nCompiler JDK: {}\nTomcat JDK: {}\nSource/Target: {}/{}\nEncoding: {}\nJDK 6 gate: {}",
            if settings.language_server_java_home.is_empty() {
                "<not set>"
            } else {
                &settings.language_server_java_home
            },
            if settings.compiler_java_home.is_empty() {
                "<not set — JDK 6 required for 1.6>"
            } else {
                &settings.compiler_java_home
            },
            if settings.tomcat_java_home.is_empty() {
                "<agent default>"
            } else {
                &settings.tomcat_java_home
            },
            settings.source_level,
            settings.target_level,
            settings.encoding,
            jdk6_status,
        );

        let last_result = state
            .last_result()
            .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
            .unwrap_or_default();
        let detected = state
            .detected()
            .iter()
            .map(|layout| {
                format!(
                    "{}: {} ({})",
                    layout.root_path,
                    layout.build_system,
                    if layout.warnings.is_empty() {
                        "no warnings"
                    } else {
                        "has warnings"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_3()
            .child(div().text_sm().child(status))
            .child(div().text_xs().child(SharedString::from(format!(
                        "Encoding: {} | Hot reload: {}",
                        state
                            .encoding()
                            .map(|encoding| encoding.encoding.clone())
                            .unwrap_or_else(|| "not detected".to_owned()),
                        if state.hot_reload_mode().is_empty() {
                            "idle"
                        } else {
                            state.hot_reload_mode()
                        }
                    ))))
            .when_some(state.last_error().map(SharedString::from), |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .child(SharedString::from(format!("ERROR: {error}"))),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.action_button("Compile", cx, |service, cx| {
                        service.compile(false, cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Clean Build", cx, |service, cx| {
                        service.compile(true, cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Build & Deploy", cx, |service, cx| {
                        service.build_and_deploy(cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Deploy", cx, |service, cx| {
                        service.deploy(cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Hot Deploy", cx, |service, cx| {
                        service.hot_deploy(None, cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Start Server", cx, |service, cx| {
                        service.start_server(false, cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Debug", cx, |service, cx| {
                        service.start_server(true, cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Stop", cx, |service, cx| {
                        let server_id = service.servers().last().map(|server| server.id.clone());
                        if let Some(server_id) = server_id {
                            service.stop_server(server_id, cx).detach_and_log_err(cx);
                        }
                    }))
                    .child(self.action_button("Select Project", cx, |service, cx| {
                        service.select_next_project(cx);
                    }))
                    .child(self.action_button("Open App", cx, |service, cx| {
                        if let Some(url) = service.application_url() {
                            cx.open_url(&url);
                        }
                    }))
                    .child(
                        div()
                            .id("kairo-scan-button")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .on_click(cx.listener(|panel, _event, _window, cx| {
                                let root = panel
                                    .workspace
                                    .read_with(cx, |workspace, app| {
                                        workspace.visible_worktrees(app).next().map(|tree| {
                                            tree.read(app).abs_path().to_string_lossy().into_owned()
                                        })
                                    })
                                    .ok()
                                    .flatten();
                                let Some(root) = root else { return };
                                let service = KairoService::global(cx);
                                service.update(cx, |service, cx| {
                                    service.bootstrap(root, cx).detach_and_log_err(cx);
                                });
                            }))
                            .child("Scan Project"),
                    )
                    .child(
                        div()
                            .id("kairo-save-encoding-button")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .on_click(cx.listener(|panel, _event, _window, cx| {
                                let path = panel
                                    .workspace
                                    .read_with(cx, |workspace, app| {
                                        workspace.active_item_as::<Editor>(app).and_then(|editor| {
                                            editor
                                                .read(app)
                                                .buffer()
                                                .read(app)
                                                .as_singleton()
                                                .and_then(|buffer| {
                                                    buffer.read(app).file().and_then(|file| {
                                                        file.file_system_abs_path(app)
                                                    })
                                                })
                                        })
                                    })
                                    .ok()
                                    .flatten();
                                let Some(path) = path else { return };
                                let text = panel
                                    .workspace
                                    .read_with(cx, |workspace, app| {
                                        workspace
                                            .active_item_as::<Editor>(app)
                                            .map(|editor| editor.read(app).text(app))
                                    })
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                let service = KairoService::global(cx);
                                service.update(cx, |service, cx| {
                                    service
                                        .save_buffer_with_encoding(
                                            path.to_string_lossy().into_owned(),
                                            text,
                                            cx,
                                        )
                                        .detach_and_log_err(cx);
                                });
                            }))
                            .child("Save with Encoding"),
                    )
                    .child(self.action_button("Refresh", cx, |service, cx| {
                        service.refresh(cx).detach_and_log_err(cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(self.view_button("Status", KairoView::Status, cx))
                    .child(self.view_button("Events", KairoView::Events, cx))
                    .child(self.view_button("Deploys", KairoView::Deployments, cx))
                    .child(self.view_button("Maven", KairoView::Maven, cx))
                    .child(self.view_button("SQL", KairoView::Sql, cx))
                    .child(self.view_button("Tests", KairoView::Tests, cx))
                    .child(self.view_button("SVN", KairoView::Svn, cx)),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(div().flex_1().child(SharedString::from(format!(
                        "Projects (active marked *)\n{projects}"
                    ))))
                    .child(
                        div()
                            .flex_1()
                            .child(SharedString::from(format!("Toolchains\n{toolchains}"))),
                    )
                    .child(div().flex_1().child(SharedString::from(toolchain_slots)))
                    .child(
                        div()
                            .flex_1()
                            .child(SharedString::from(format!("Servers\n{servers}"))),
                    ),
            )
            .child(
                div()
                    .id("kairo-log-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(SharedString::from(format!(
                        "Tomcat logs (last {MAX_VISIBLE_LOG_LINES})\n{logs}"
                    ))),
            )
            .when(!detected.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .child(SharedString::from(format!("Detected layouts:\n{detected}"))),
                )
            })
            .child(match self.active_view {
                KairoView::Status => div()
                    .id("kairo-last-result")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(SharedString::from(format!(
                        "Last operation result:\n{}",
                        last_result.chars().take(4000).collect::<String>()
                    )))
                    .into_any_element(),
                KairoView::Events => crate::kairo_views::events_view(cx).into_any_element(),
                KairoView::Deployments => {
                    crate::kairo_views::deployments_view(cx).into_any_element()
                }
                KairoView::Maven => crate::kairo_views::maven_view(cx).into_any_element(),
                KairoView::Sql => crate::kairo_views::sql_view(cx).into_any_element(),
                KairoView::Tests => crate::kairo_views::tests_view(cx).into_any_element(),
                KairoView::Svn => crate::kairo_views::svn_view(cx).into_any_element(),
            })
    }
}
