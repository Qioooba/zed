//! Secondary Kairo views: Events, Deployments, Maven, SQL, JUnit, SVN.
//! Each view is a section that can be embedded in the Kairo panel.

use gpui::{Context, SharedString, div, prelude::*};
use kairo_core::{i18n, service::KairoService};

use crate::kairo_panel::KairoPanel;

pub fn events_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let service = KairoService::global(cx);
    let state = service.read(cx);
    let strings = i18n::strings(cx);
    let lines = state
        .events()
        .iter()
        .rev()
        .take(100)
        .map(|event| {
            format!(
                "#{} [{}] {} {}",
                event.sequence, event.event_type, event.message, event.time
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    div()
        .id("kairo-events-view")
        .flex_1()
        .overflow_y_scroll()
        .child(SharedString::from(format!(
            "{}\n{}",
            strings
                .events_label
                .replace("{}", &state.event_gap_count().to_string()),
            if lines.is_empty() {
                strings.no_events_yet
            } else {
                &lines
            }
        )))
}

pub fn deployments_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let service = KairoService::global(cx);
    let state = service.read(cx);
    let strings = i18n::strings(cx);
    let result = state
        .last_result()
        .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
        .unwrap_or_else(|| strings.deployment_history_hint.to_owned());
    div()
        .id("kairo-deployments-view")
        .flex_1()
        .overflow_y_scroll()
        .child(SharedString::from(result))
}

pub fn maven_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let service = KairoService::global(cx);
    let state = service.read(cx);
    let strings = i18n::strings(cx);
    let result = state
        .last_result()
        .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
        .unwrap_or_else(|| strings.maven_hint.to_owned());
    div()
        .id("kairo-maven-view")
        .flex_1()
        .overflow_y_scroll()
        .child(SharedString::from(result))
}

pub fn sql_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let service = KairoService::global(cx);
    let state = service.read(cx);
    let strings = i18n::strings(cx);
    let result = state
        .last_result()
        .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
        .unwrap_or_else(|| strings.sql_hint.to_owned());
    div()
        .id("kairo-sql-view")
        .flex_1()
        .overflow_y_scroll()
        .child(SharedString::from(result))
}

pub fn tests_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let service = KairoService::global(cx);
    let state = service.read(cx);
    let strings = i18n::strings(cx);
    let result = state
        .last_result()
        .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
        .unwrap_or_else(|| strings.tests_hint.to_owned());
    div()
        .id("kairo-tests-view")
        .flex_1()
        .overflow_y_scroll()
        .child(SharedString::from(result))
}

pub fn svn_view(cx: &mut Context<KairoPanel>) -> impl gpui::IntoElement {
    let strings = i18n::strings(cx);
    div()
        .id("kairo-svn-view")
        .flex_1()
        .child(SharedString::from(strings.svn_hint))
}
