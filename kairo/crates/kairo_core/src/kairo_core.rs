//! Core of the Kairo-for-Zed integration: settings, actions, the runtime-agent
//! client, and the global service. This crate contains no UI.

pub mod actions;
pub mod agent;
pub mod client;
pub mod domain;
pub mod service;
pub mod settings;

use gpui::App;

/// Register Kairo settings, keybindings, and the global service.
pub fn init(cx: &mut App) {
    settings::register(cx);
    actions::bind_keys(cx);
    service::init_global(cx);
}
