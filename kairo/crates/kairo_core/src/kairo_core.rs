//! Core of the Kairo-for-Zed integration: settings, actions, the runtime-agent
//! client, and the global service. This crate contains no UI.

pub mod actions;
pub mod agent;
pub mod client;
pub mod domain;
pub mod endpoints;
pub mod events;
pub mod i18n;
pub mod save_hook;
pub mod service;
pub mod service_ext;
pub mod settings;

use gpui::App;

/// Register Kairo settings, keybindings, and the global service.
pub fn init(cx: &mut App) {
    settings::register(cx);
    actions::bind_keys(cx);
    service::init_global(cx);
    let service = service::KairoService::global(cx);
    let state = service.read(cx);
    if let Some(base_url) = state.agent_base_url() {
        save_hook::configure(base_url, state.settings().encoding.clone());
    }
}
