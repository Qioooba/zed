//! A dock panel that surfaces Kairo agent state, build/deploy results, and
//! paged Tomcat logs. Logs are deliberately bounded — Kairo's UI spec forbids
//! unbounded log views on the target 4 GB machines.

use gpui::{
    App, AsyncWindowContext, Context, EventEmitter, FluentBuilder, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Task, Window, div, px,
};
use icons::IconName;
use kairo_core::{actions, service::KairoService};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

const MAX_VISIBLE_LOG_LINES: usize = 200;

pub struct KairoPanel {
    workspace: gpui::WeakEntity<Workspace>,
    focus_handle: FocusHandle,
}

impl KairoPanel {
    pub fn new(
        workspace: gpui::WeakEntity<Workspace>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
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
        self.workspace.read(cx).is_ok()
    }
}

impl Render for KairoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let service = KairoService::global(cx);
        let state = service.read(cx);
        let status = SharedString::from(state.status_message().to_owned());
        let error = state
            .last_error()
            .map(SharedString::from)
            .unwrap_or_else(|| SharedString::from(""));

        let toolchains = state
            .toolchains()
            .iter()
            .map(|toolchain| {
                format!(
                    "{} {} ({})",
                    toolchain.vendor,
                    toolchain.version,
                    if toolchain.fingerprint.is_empty() {
                        "unverified"
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
                    "{}: {} (gen {}, port {})",
                    server.id, server.state, server.generation, server.ports.http
                )
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

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_3()
            .child(div().text_sm().child(status))
            .when(!error.is_empty(), |this| {
                this.child(div().text_xs().child(error))
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
                    .child(self.action_button("Deploy", cx, |service, cx| {
                        service.deploy(cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Start Server", cx, |service, cx| {
                        service.start_agent(cx).detach_and_log_err(cx);
                    }))
                    .child(self.action_button("Refresh", cx, |service, cx| {
                        service.refresh(cx).detach_and_log_err(cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .child(SharedString::from(format!(
                                "Toolchains\n{toolchains}"
                            ))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(SharedString::from(format!("Servers\n{servers}"))),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .child(SharedString::from(format!("Logs\n{logs}"))),
            )
    }
}

impl KairoPanel {
    fn action_button(
        &self,
        label: &'static str,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut KairoService, &mut Context<KairoService>) + 'static,
    ) -> impl IntoElement {
        div()
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
