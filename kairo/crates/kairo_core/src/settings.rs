//! Kairo settings, registered through Zed's settings system.

use gpui::App;
use settings::{RegisterSetting, Settings, SettingsContent};

#[derive(Clone, Debug, RegisterSetting)]
pub struct KairoSettings {
    pub agent_path: String,
    pub data_dir: String,
    pub bundled_dir: String,
    pub language_server_java_home: String,
    pub compiler_java_home: String,
    pub tomcat_java_home: String,
    pub source_level: String,
    pub target_level: String,
    pub encoding: String,
    pub tomcat_http_port: u16,
    pub debug_port: u16,
    pub auto_start_agent: bool,
}

impl Default for KairoSettings {
    fn default() -> Self {
        Self {
            agent_path: String::new(),
            data_dir: String::new(),
            bundled_dir: String::new(),
            language_server_java_home: String::new(),
            compiler_java_home: String::new(),
            tomcat_java_home: String::new(),
            source_level: "1.6".to_owned(),
            target_level: "1.6".to_owned(),
            encoding: "gbk".to_owned(),
            tomcat_http_port: 0,
            debug_port: 8000,
            auto_start_agent: true,
        }
    }
}

impl Settings for KairoSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let defaults = Self::default();
        let Some(kairo) = content.kairo.as_ref() else {
            return defaults;
        };
        Self {
            agent_path: kairo.agent_path.clone().unwrap_or(defaults.agent_path),
            data_dir: kairo.data_dir.clone().unwrap_or(defaults.data_dir),
            bundled_dir: kairo.bundled_dir.clone().unwrap_or(defaults.bundled_dir),
            language_server_java_home: kairo
                .language_server_java_home
                .clone()
                .unwrap_or(defaults.language_server_java_home),
            compiler_java_home: kairo
                .compiler_java_home
                .clone()
                .unwrap_or(defaults.compiler_java_home),
            tomcat_java_home: kairo
                .tomcat_java_home
                .clone()
                .unwrap_or(defaults.tomcat_java_home),
            source_level: kairo.source_level.clone().unwrap_or(defaults.source_level),
            target_level: kairo.target_level.clone().unwrap_or(defaults.target_level),
            encoding: kairo.encoding.clone().unwrap_or(defaults.encoding),
            tomcat_http_port: kairo.tomcat_http_port.unwrap_or(defaults.tomcat_http_port),
            debug_port: kairo.debug_port.unwrap_or(defaults.debug_port),
            auto_start_agent: kairo.auto_start_agent.unwrap_or(defaults.auto_start_agent),
        }
    }
}

pub fn register(cx: &mut App) {
    KairoSettings::register(cx);
}

pub fn get(cx: &App) -> &KairoSettings {
    KairoSettings::get_global(cx)
}
