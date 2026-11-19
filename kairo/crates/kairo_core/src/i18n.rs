//! Bilingual string table for the Kairo panel.
//!
//! Zed extensions cannot replace another crate's UI strings at runtime, so the
//! Kairo UI reads its user-facing text from this self-contained table. The
//! selected language comes from `kairo.language` in the user's settings.

use gpui::App;

use crate::settings;

/// Languages supported by the Kairo UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    ZhCn,
}

impl Lang {
    /// Resolve the active language from Kairo settings.
    pub fn from_settings(cx: &App) -> Self {
        let language = settings::get(cx).language.as_str();
        match language.to_ascii_lowercase().replace('_', "-").as_str() {
            "zh" | "zh-cn" | "zh-hans" | "cn" => Lang::ZhCn,
            _ => Lang::En,
        }
    }
}

/// All user-facing strings rendered by the Kairo panel and its secondary views.
#[derive(Clone, Copy, Debug)]
pub struct KairoStrings {
    // Action buttons.
    pub compile: &'static str,
    pub clean_build: &'static str,
    pub build_and_deploy: &'static str,
    pub deploy: &'static str,
    pub hot_deploy: &'static str,
    pub start_server: &'static str,
    pub debug: &'static str,
    pub stop_server: &'static str,
    pub select_project: &'static str,
    pub open_app: &'static str,
    pub scan_project: &'static str,
    pub save_with_encoding: &'static str,
    pub refresh: &'static str,

    // View tabs and tooltips.
    pub status: &'static str,
    pub events: &'static str,
    pub deploys: &'static str,
    pub maven: &'static str,
    pub sql: &'static str,
    pub tests: &'static str,
    pub svn: &'static str,
    pub icon_tooltip: &'static str,

    // Error and status labels.
    pub error_prefix: &'static str,
    pub no_agent: &'static str,
    pub no_project: &'static str,
    pub toolchain_incompatible_guidance: &'static str,
    pub agent_not_running: &'static str,
    pub operation_unavailable: &'static str,
    pub operation_failed: &'static str,
    pub not_set: &'static str,
    pub agent_default: &'static str,
    pub not_detected: &'static str,
    pub idle: &'static str,
    pub no_events_yet: &'static str,
    pub no_warnings: &'static str,
    pub has_warnings: &'static str,
    pub unverified: &'static str,
    pub fingerprinted: &'static str,
    pub jdk6_available: &'static str,
    pub jdk6_missing: &'static str,

    // Section headings and hints.
    pub encoding_label: &'static str,
    pub hot_reload_label: &'static str,
    pub projects_label: &'static str,
    pub toolchains_label: &'static str,
    pub servers_label: &'static str,
    pub tomcat_logs_label: &'static str,
    pub detected_layouts_label: &'static str,
    pub last_result_label: &'static str,
    pub events_label: &'static str,
    pub deployment_history_hint: &'static str,
    pub maven_hint: &'static str,
    pub sql_hint: &'static str,
    pub tests_hint: &'static str,
    pub svn_hint: &'static str,
}

const EN: KairoStrings = KairoStrings {
    compile: "Compile",
    clean_build: "Clean Build",
    build_and_deploy: "Build & Deploy",
    deploy: "Deploy",
    hot_deploy: "Hot Deploy",
    start_server: "Start Server",
    debug: "Debug",
    stop_server: "Stop",
    select_project: "Select Project",
    open_app: "Open App",
    scan_project: "Scan Project",
    save_with_encoding: "Save with Encoding",
    refresh: "Refresh",
    status: "Status",
    events: "Events",
    deploys: "Deploys",
    maven: "Maven",
    sql: "SQL",
    tests: "Tests",
    svn: "SVN",
    icon_tooltip: "Kairo: legacy Java Web tools",
    error_prefix: "ERROR:",
    no_agent: "Kairo agent is not running",
    no_project: "No Kairo project is open",
    toolchain_incompatible_guidance: "The active toolchain is incompatible with this project's source level. Import a real JDK 6 and reconfigure the toolchain slots.",
    agent_not_running: "Kairo runtime agent is not running",
    operation_unavailable: "Kairo operation unavailable",
    operation_failed: "Kairo operation failed",
    not_set: "<not set>",
    agent_default: "<agent default>",
    not_detected: "not detected",
    idle: "idle",
    no_events_yet: "<no events yet>",
    no_warnings: "no warnings",
    has_warnings: "has warnings",
    unverified: "UNVERIFIED",
    fingerprinted: "fingerprinted",
    jdk6_available: "AVAILABLE (real JDK 6 imported)",
    jdk6_missing: "MISSING - 1.6 builds will fail closed",
    encoding_label: "Encoding",
    hot_reload_label: "Hot reload",
    projects_label: "Projects (active marked *)",
    toolchains_label: "Toolchains",
    servers_label: "Servers",
    tomcat_logs_label: "Tomcat logs",
    detected_layouts_label: "Detected layouts:",
    last_result_label: "Last operation result:",
    events_label: "Events (gap count: {})",
    deployment_history_hint: "Run Kairo: Deployment History",
    maven_hint: "Run Kairo: Maven Detect / Maven Dependencies",
    sql_hint: "Run Kairo: Sql Test",
    tests_hint: "Run Kairo: Detect Java / Java Run",
    svn_hint: "SVN via Zed tasks: Kairo: Svn Status / Svn Update / Svn Commit\n(tasks.json uses the real svn CLI on this machine)",
};

const ZH_CN: KairoStrings = KairoStrings {
    compile: "编译",
    clean_build: "清理构建",
    build_and_deploy: "构建并部署",
    deploy: "部署",
    hot_deploy: "热部署",
    start_server: "启动服务器",
    debug: "调试",
    stop_server: "停止",
    select_project: "选择项目",
    open_app: "打开应用",
    scan_project: "扫描项目",
    save_with_encoding: "按编码保存",
    refresh: "刷新",
    status: "状态",
    events: "事件",
    deploys: "部署",
    maven: "Maven",
    sql: "SQL",
    tests: "测试",
    svn: "SVN",
    icon_tooltip: "Kairo：旧版 Java Web 工具",
    error_prefix: "错误：",
    no_agent: "Kairo 代理未运行",
    no_project: "未打开 Kairo 项目",
    toolchain_incompatible_guidance: "当前工具链与项目源码级别不兼容。请导入真正的 JDK 6 并重新配置工具链设置。",
    agent_not_running: "Kairo 运行时代理未运行",
    operation_unavailable: "Kairo 操作不可用",
    operation_failed: "Kairo 操作失败",
    not_set: "<未设置>",
    agent_default: "<代理默认值>",
    not_detected: "未检测到",
    idle: "空闲",
    no_events_yet: "<暂无事件>",
    no_warnings: "无警告",
    has_warnings: "有警告",
    unverified: "未验证",
    fingerprinted: "已校验",
    jdk6_available: "可用（已导入真正的 JDK 6）",
    jdk6_missing: "缺失 - 1.6 构建将直接失败",
    encoding_label: "编码",
    hot_reload_label: "热重载",
    projects_label: "项目（* 表示当前项目）",
    toolchains_label: "工具链",
    servers_label: "服务器",
    tomcat_logs_label: "Tomcat 日志",
    detected_layouts_label: "检测到的布局：",
    last_result_label: "上次操作结果：",
    events_label: "事件（间隔计数：{}）",
    deployment_history_hint: "运行 Kairo：部署历史",
    maven_hint: "运行 Kairo：Maven 检测 / Maven 依赖",
    sql_hint: "运行 Kairo：Sql 测试",
    tests_hint: "运行 Kairo：检测 Java / 运行 Java",
    svn_hint: "通过 Zed 任务使用 SVN：Kairo：Svn Status / Svn Update / Svn Commit\n（tasks.json 使用本机的真实 svn 命令行）",
};

/// Return the string table for the language selected in Kairo settings.
pub fn strings(cx: &App) -> KairoStrings {
    match Lang::from_settings(cx) {
        Lang::En => EN,
        Lang::ZhCn => ZH_CN,
    }
}
