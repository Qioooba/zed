# 05 — Zed core layout: insertion points for the Kairo / Java-JSP-Tomcat integration

> Scope: `E:\github\zed` (Zed editor, Rust workspace, ~255 crates).
> Branch: **`kairo/integration`** (created from upstream `main`; only `README.md` was
> modified at analysis time, by the harness's review-checklist rule).
> Goal: map the parts of Zed's core that matter for a legacy Java/JSP/Tomcat toolchain
> and name the *smallest* set of upstream edits needed to register new code.

---

## 1. Root `Cargo.toml` — workspace mechanics

File: `E:\github\zed\Cargo.toml`

| Concern | Exact location | Notes |
|---|---|---|
| Workspace declaration | line 1 `[workspace]`, `resolver = "2"` | — |
| Member list | lines 3–277 `members = [ ... ]` | One quoted path per line, alphabetically sorted (with comment-separated sections for `# Extensions` at 264–267 and `# Tooling` at 273–276). Paths are relative to the workspace root. |
| Default build target | line 278 `default-members = ["crates/zed"]` | `cargo build` at the root builds only Zed; new members are compiled because `crates/zed` depends on them. |
| Shared package metadata | lines 280–282 `[workspace.package]`: `publish = false`, `edition = "2024"` | New crates inherit with `edition.workspace = true`, `publish.workspace = true`. |
| Shared dependency graph | lines 284–532 `[workspace.dependencies]` | Two sections: *"Workspace member crates"* (290–532, `crate = { path = "crates/crate" }`) and *"External crates"* (from 538 onward, versioned/git deps). Members are again alphabetically sorted. |
| Shared lints | lines 1172–1177 `[workspace.lints.rust]` / `[workspace.lints.clippy]` | `unexpected_cfgs = allow`; `dbg_macro`, `todo`, `redundant_clone`, `disallowed_methods` are `deny`. Crates opt in with `[lints] workspace = true`. |

### 1.1 Naming conventions (observed)

- Crate directories and package names are **lowercase `snake_case`**, no hyphens:
  `project`, `terminal_view`, `toolchain_selector`, `lsp_store` (module), `debugger_ui`.
- `*ui` suffix = UI/panel crate (`terminal_view`, `debugger_ui`, `tasks_ui`, `settings_ui`).
- `*store` / `*registry` = app-level state entity (`WorkspaceStore`, `SettingsStore`,
  `LanguageRegistry`, `TaskStore`).
- `*tools` / `*adapters` = protocol glue (`acp_tools`, `dap_adapters`).
- Library roots are **not** `lib.rs`: `[lib] path = "src/<crate_name>.rs"` is universal.
  This is an explicit repo rule in `AGENTS.md`: *"Never create files with `mod.rs` paths —
  prefer `src/some_module.rs`"*, and *"prefer specifying the library root path in
  `Cargo.toml` using `[lib] path = '...rs'`"*.

### 1.2 Adding a new crate with minimal disruption

Four touch points, two of them in the same file:

1. `Cargo.toml` `members` — add one line, e.g.
   `"kairo/crates/kairo_core",` (or `"crates/kairo_core",` if you prefer the
   conventional location; `members` already accepts non-`crates/` paths such as
   `"extensions/glsl"` and `"tooling/xtask"`, so a `kairo/` subtree is legal and keeps
   new files visible in one place, matching `kairo/README.md`).
2. `Cargo.toml` `[workspace.dependencies]` — add one alias line:
   `kairo_core = { path = "kairo/crates/kairo_core" }` (in the member-crates section).
3. `crates/zed/Cargo.toml` `[dependencies]` — add `kairo_core.workspace = true`.
4. `crates/zed/src/main.rs` — add one `kairo_core::init(cx);` line in the startup
   `app.run` closure (see §3). `Cargo.lock` is regenerated automatically by Cargo.

A crate's own `Cargo.toml` boilerplate to copy (from `crates/task/Cargo.toml`,
`crates/toolchain_selector/Cargo.toml`):

```toml
[package]
name = "kairo_core"
version = "0.1.0"
edition.workspace = true
publish.workspace = true
license = "GPL-3.0-or-later"

[lints]
workspace = true

[lib]
path = "src/kairo_core.rs"
doctest = false

[features]
test-support = ["gpui/test-support"]   # only if needed

[dependencies]
gpui.workspace = true
serde.workspace = true
util.workspace = true
...
```

> Do **not** add `mod.rs` files; declare submodules from the crate root
> (`src/kairo_core.rs`) as `pub mod client; pub mod settings; ...`, exactly like
> `crates/project/src/project.rs` lines 1–34.

---

## 2. Representative crates to model after

| Crate | `Cargo.toml` | Library root | What it demonstrates |
|---|---|---|---|
| `project` | `crates/project/Cargo.toml` — `[lib] path = "src/project.rs"`, `[[test]]` at `tests/integration/project_tests.rs` requiring `test-support` | `crates/project/src/project.rs` | Big "service" crate: 25+ `pub mod` declarations at top, entity stores as fields, `Project::init` / `Project::local` constructors, feature-gated `test-support` list. |
| `workspace` | `crates/workspace/Cargo.toml` | `crates/workspace/src/workspace.rs` | App state (`AppState`, `WorkspaceStore`), docks, panel registry, `TerminalProvider` trait, task scheduling. |
| `settings` | `crates/settings/Cargo.toml` — `[lib] path = "src/settings.rs"` | `crates/settings/src/settings.rs` | `Settings` trait, global `SettingsStore`, register-on-init pattern. |
| `task` | `crates/task/Cargo.toml` — `[lib] path = "src/task.rs"`, tiny dep set | `crates/task/src/task.rs` | Clean data-only crate: `TaskId`, `SpawnInTerminal`, `TaskTemplate`, `TaskSourceKind` serialization. |
| `language` | `crates/language/Cargo.toml` | `crates/language/src/lib.rs` | `LanguageRegistry`, `Language`, `Toolchain`, grammar registration. |
| `terminal` | `crates/terminal/Cargo.toml` — `[lib] path = "src/terminal.rs"` | `crates/terminal/src/terminal.rs` | PTY/subprocess life-cycle, `TerminalMode`, `spawn_task_subprocess`. |
| `terminal_view` | `crates/terminal_view/Cargo.toml` | `crates/terminal_view/src/terminal_view.rs` | UI crate that registers a dock panel and actions: `terminal_view::init` → `terminal_panel::init`. |
| `paths` | `crates/paths/Cargo.toml` — `[lib] path = "src/paths.rs"` | `crates/paths/src/paths.rs` | Application/data/config paths; `APP_NAME` constant with explicit fork note at lines 14–18. |
| `util` | `crates/util/Cargo.toml` — `[lib] path = "src/util.rs"` | `crates/util/src/util.rs` | Process spawning, shell helpers, path utilities; flat module list at `src/util.rs` lines 4–31. |

**Layout pattern:** one root file named after the crate, flat `src/*.rs` modules,
no `mod.rs`, `pub mod` re-exports from the root, paired `test-support` features that
forward to dependencies' `test-support`.

---

## 3. App-level service initialization

### 3.1 The one startup closure

File: `crates/zed/src/main.rs`. The `Application::run` callback starts at **line 491**:

```rust
app.run(move |cx| {
    cx.set_global(app_db);                       // 492
    ...
    trusted_worktrees::init(db_trusted_paths, cx); // 500
    menu::init(); zed_actions::init();           // 501-502
    settings::init(cx);                          // 509
    <dyn Fs>::set_global(fs.clone(), cx);        // 529
    extension::init(cx);                         // 536
    Client::set_global(client.clone(), cx);      // 597
    zed::init(cx);                               // 599
    project::Project::init(&client, cx);         // 602
    ...
    let app_state = Arc::new(AppState { ... });  // 657
    AppState::set_global(app_state.clone(), cx); // 667
    workspace::init(app_state.clone(), cx);      // 749
    project_panel::init(cx);                     // 759
    ...
    terminal_view::init(cx);                     // 776
    ...
    initialize_workspace(app_state.clone(), cx); // 888
});
```

This is the **single registration site** for global services. A new Kairo global
service should be introduced as a small `pub fn init(cx: &mut App)` in
`kairo_core/src/kairo_core.rs` and invoked with **one line** here (recommended spot:
next to `terminal_view::init(cx)` at ~line 776, after `AppState::set_global` so it can
read `AppState::global(cx)` / `SettingsStore::global(cx)`).

Two ways to expose the service, both used in-tree:

- **Global struct via `set_global`**, e.g. `AppState::set_global` (667),
  `Client::set_global` (597), `<dyn Fs>::set_global` (529),
  `agent::ThreadStore::init_global` → `cx.set_global(GlobalThreadStore(...))`
  (`crates/agent/src/thread_store.rs:18-20`). Access with `cx.global::<T>()`,
  `T::global(cx)`, `T::try_global(cx)`, or `cx.update_global::<T, _>(...)`.
- **Entity held by `AppState`**, e.g. `WorkspaceStore` created at `main.rs:576` and
  stored in `AppState` at 657–666. Prefer this if the service needs updates,
  subscriptions, or window-aware async work.

There is already a dedicated per-project state container; a Kairo service can also
live **per-`Project`** by adding a field to `Project` and constructing it in
`Project::local` (`crates/project/src/project.rs:1303`, store construction at
1313–1430). That is an *upstream-owned file edit*, so for a small patch surface prefer
a global service keyed by `WorktreeId`/project handle instead.

### 3.2 `Project` / `LspStore` construction

| Symbol | Location | Role |
|---|---|---|
| `Project::init(client, cx)` | `crates/project/src/project.rs:1267` | Registers RPC handlers, then calls `WorktreeStore::init`, `BufferStore::init`, **`LspStore::init(&client)` (1293)**, `GitStore::init`, `TaskStore::init`, `ToolchainStore::init`, `DapStore::init`, etc. |
| `Project::local(...)` | `crates/project/src/project.rs:1303` | Builds a local `Entity<Project>` with worktree/buffer/git/DAP/toolchain stores; creates `LspStore::new_local(...)` at **1430**. |
| `LspStore::init(&AnyProtoClient)` | `crates/project/src/lsp_store.rs:4845` | Registers all LSP RPC handlers. |
| `LspStore::new_local(...)` | `crates/project/src/lsp_store.rs:4948` | Constructs the language-server store for a local project. |
| `LanguageRegistry::register_language` / `LanguageRegistry::add` | `crates/language/src/language_registry.rs:439` / `599` | Where Java/JSP grammar + LSP adapter would be added; `languages::init(...)` is called at `main.rs:574`. |

### 3.3 Settings and keybindings without touching upstream assets

- Implement the `Settings` trait (`crates/settings/src/settings_store.rs:69`) for
  `KairoSettings`; call `KairoSettings::register(cx)` from `kairo_core::init`.
  `Settings::register` (line 86) calls `SettingsStore::update_global` →
  `register_setting::<Self>()`, so **no edit to `assets/settings/default.json` is
  required** (though a default is needed in the struct's `Default`/`from_settings`).
- Register Kairo actions (`#[derive(Action)]` / `actions!(kairo, [...])`) and bind
  keys from the crate at runtime with `cx.bind_keys([KeyBinding::new(...)])`
  (pattern in `crates/agent_ui/src/agent_panel.rs:9515`, `crates/call_hierarchy/...:2057`).
  This avoids editing `assets/keymaps/default-*.json`.

---

## 4. Terminal & task execution (build / deploy)

| Layer | Symbol / path | Notes |
|---|---|---|
| Task definitions | `crates/task/src/task.rs`: `TaskId` (38), `SpawnInTerminal` (42), `TaskTemplate`, `TaskTemplates`, `TaskContext`, `TaskVariables`, `SaveStrategy`, `RevealStrategy` | `SpawnInTerminal` carries `command`, `args`, `cwd`, `env`, `shell`, `use_new_terminal`, `reveal`, `hide`, `show_summary`. This is the currency of task execution. |
| Task inventory | `crates/project/src/task_inventory.rs:43` `Inventory`, `crates/project/src/task_store.rs:240` `TaskStore::task_inventory()` | Where static/dynamic task sources are collected. |
| Task scheduling | `crates/workspace/src/tasks.rs`: `Workspace::schedule_task` (44), `schedule_resolved_task` (70), `schedule_resolved_task_internal` (107), `Workspace::spawn_in_terminal` (228) | `schedule_resolved_task_internal` calls `self.terminal_provider.as_ref().map(|p| p.spawn(spawn_in_terminal, window, cx))` at 133–144, then `cx.background_spawn(spawn_task).await` at 146. |
| Provider hook | `crates/workspace/src/workspace.rs:327` `trait TerminalProvider`, setter at 3545 `Workspace::set_terminal_provider` | Implement this in the Kairo UI crate if Kairo should launch builds as ordinary Zed tasks; only one provider is installed (currently `TerminalPanel`, see below). |
| Terminal backend | `crates/terminal/src/terminal.rs`: `TerminalMode` (936), `TerminalMode::task(SpawnInTerminal)` (965), `Terminal::new` (1095), `spawn_task_subprocess` (3376) | `spawn_task_subprocess` builds a command with `util::command::new_std_command` and spawns via `util::process::Child::spawn(command, Stdio::null(), Stdio::piped(), Stdio::piped())` (3388–3396). |
| Terminal panel / provider impl | `crates/terminal_view/src/terminal_panel.rs`: `init` (55), `load` (237), `set_terminal_provider(...)` (247), `impl Panel` (1692), `persistent_name` (1796), `panel_key` (1800) | `TerminalPanel::load` installs the workspace's `TerminalProvider`; actions are registered via `cx.observe_new` in `init`. |
| Tasks UI | `crates/tasks_ui` (member at root `Cargo.toml:218`), `tasks_ui::init(cx)` called at `main.rs:761` | Panels/menus for running tasks; model for a Kairo "Run" menu. |

**Kairo options:**

- **Recommended:** keep the Go runtime agent as the backend and model each Kairo
  operation (compile, hot-deploy, start/stop Tomcat, attach debug) as a
  `SpawnInTerminal`/`TaskTemplate` surfaced through `TaskInventory`, so it shows up
  in the command palette and reuses Zed's PTY + task lifecycle.
- For operations managed by the Go agent (long-lived server, JDI bridge), drive the
  agent over HTTP from `kairo_core` and **display logs in the Kairo panel**, not via
  `TerminalProvider` (avoids fighting the single-provider slot).

---

## 5. Spawning external processes

There is **no `crates/process` crate** in this checkout; the equivalent lives in `util`.

| API | Exact path / symbol | Use |
|---|---|---|
| Portable command builder | `crates/util/src/command.rs`: `new_command` (16), `pub struct Command` (28), `Command::spawn` (113), `output` (117), `status` (121); re-exports `gpui_util::new_std_command` (14) | Cross-platform wrapper over `smol::process::Command`; on Windows applies `CREATE_NO_WINDOW`. `Child = smol::process::Child` (21). macOS has a specialized `command::darwin` implementation (lines 5–9). |
| Process-tree cleanup | `crates/util/src/process.rs`: `pub struct Child` (11), `Child::spawn(command, stdin, stdout, stderr)` (33 non-Windows / 56 Windows) | Starts a new session on Unix, assigns a Job Object on Windows so descendants (e.g. Tomcat, `kairo-jdi-bridge.jar`) are reaped on drop. **Use this for the Go runtime agent and server processes.** |
| Shell command construction | `crates/util/src/shell_builder.rs` (`ShellBuilder`), `crates/util/src/shell.rs` (`Shell`, `ShellKind`, `get_system_shell`) | Task shell semantics; also re-exported from `crates/task/src/task.rs:29-30`. |
| Paths | `crates/util/src/paths.rs` (`SanitizedPath`, `home_dir`) and `crates/paths/src/paths.rs` (`APP_NAME`, `data_dir()`, `config_dir()`, `languages_dir()`) | `paths` is the app-level crate; `util::paths` holds generic helpers. |
| Async scheduling | `cx.spawn(...)` / `cx.spawn_in(window, ...)`, `cx.background_spawn(...)`, `Task`, `cx.background_executor().timer(...)` | Foreground entity updates vs. CPU/IO work. Prefer GPUI executor timers over `smol::Timer` in tests (repo `AGENTS.md`). |

A typical Kairo agent launch would therefore be:

```rust
let mut command = util::command::new_command(&kairo_agent_path);
command.arg("--config").arg(config_path);
let child = util::process::Child::spawn(
    command.into(),                                  // or std::process::Command
    Stdio::null(), Stdio::piped(), Stdio::piped(),
)?;
let status: Task<Result<ExitStatus>> =
    cx.background_spawn(async move { child.wait().await.context("agent exited") });
```

Keep the `Task` in a struct field or `.detach()` it deliberately — dropped tasks are
cancelled (repo `AGENTS.md`, "Concurrency").

---

## 6. Panel / dock view for a Kairo control panel

### 6.1 The `Panel` contract

File: `crates/workspace/src/dock.rs` (also re-exported through `workspace`).

`pub trait Panel: Focusable + EventEmitter<PanelEvent> + Render + Sized` (line 36).
Required / commonly overridden methods:

| Method (line) | Kairo implementation note |
|---|---|
| `persistent_name()` (37) | Stable serialization name, e.g. `"KairoPanel"`. |
| `panel_key()` (38) | Settings key, e.g. `"kairo_panel"`. |
| `activation_focus_handle(&self, cx)` (44) | Focus target (log filter / action button); must be a focus descendant of root. |
| `position`, `position_is_valid`, `set_position` (47–49) | Usually `DockPosition::Bottom`/`Right`. |
| `default_size`, `min_size`, `initial_size_state` (50–56) | Panel height. |
| `icon`, `icon_tooltip`, `toggle_action` (71–73) | Status-bar button; `toggle_action()` returns a boxed Kairo `TogglePanel` action. |
| `starts_open`, `set_active`, `is_zoomed`, `set_zoomed` (77–84) | — |
| `activation_priority()` (91) | Lower priority than terminal/debugger panels. |
| `enabled(&self, cx)` (92) | Can hide panel when no Kairo project is open. |

`PanelHandle` (line 106) is blanket-implemented for `Entity<T> where T: Panel`
(line 154), so `workspace.add_panel(entity, window, cx)` just works.

### 6.2 Registration flow (exact insertion points)

| Step | Symbol / file |
|---|---|
| Add panel to a dock | `Workspace::add_panel` → `dock.add_panel(panel.clone(), self.weak_self.clone(), window, cx)` at `crates/workspace/src/workspace.rs:2948`. |
| Query panel | `Workspace::panel::<P>(cx)` at `workspace.rs:5071`; `Workspace::toggle_panel_focus::<P>(...)`, `close_panel::<P>(...)` used by `terminal_panel::init` (`terminal_panel.rs:60-70`). |
| Per-workspace actions | `cx.observe_new(|workspace: &mut Workspace, ...| workspace.register_action(...))` — `terminal_panel.rs:55-75`, `terminal_view.rs:113`. |
| Async load + add | `crates/zed/src/zed.rs:780-811` inside `initialize_workspace` (`zed.rs:430`). Panels are created with `Panel::load(workspace: WeakEntity<Workspace>, cx: AsyncWindowContext)` and added with helper `add_panel_when_ready` (`zed.rs:788`) which calls `workspace.add_panel(panel, window, cx)` at **794**. `futures::join!` at 803–811 loads project/outline/terminal/git/collab/debug panels, then `workspace.finish_dock_restoration(cx)` at 814. |
| Example loader | `TerminalPanel::load` at `crates/terminal_view/src/terminal_panel.rs:237-275`: create entity via `workspace.update_in`, restore serialized state, return `Entity<Self>`. |

**Smallest panel integration (2 upstream lines):**

1. `crates/zed/Cargo.toml`: `kairo_ui.workspace = true`.
2. `crates/zed/src/zed.rs` inside the `futures::join!` block (~line 803):
   ```rust
   add_panel_when_ready(kairo_ui::KairoPanel::load(workspace_handle.clone(), cx.clone()), workspace_handle.clone(), cx.clone()),
   ```
   (`kairo_ui::init(cx)` can itself register actions via `cx.observe_new`, so no
   further `zed.rs` edits are needed.)

The panel entity should hold `WeakEntity<Project>`/`WeakEntity<Workspace>` and a
handle to the `kairo_core` global service; it renders build status, server state,
and streaming logs. Model it on `TerminalPanel` / `DebugPanel` /
`crates/git_ui/src/git_panel.rs` (`impl Panel`, `panel_key` at 10202) and
`crates/project_panel`.

---

## 7. Version / sync strategy (rebasing on upstream Zed)

### 7.1 File ownership map

| File / path | Owner | Conflict risk when syncing upstream | Action |
|---|---|---|---|
| `kairo/**` (new crates, docs, fixtures) | **Kairo** | None — upstream never touches it | Create freely; this document lives here. |
| `Cargo.toml` (root) | upstream | **High** (active file, 2 insertions) | Add 1 `members` line + 1 `[workspace.dependencies]` line. Keep them in the correct sorted section to minimize textual drift. |
| `Cargo.lock` | upstream (generated) | Medium — regenerated by Cargo | Commit the generated diff; resolve mechanically on rebase. |
| `crates/zed/Cargo.toml` | upstream | Medium (1 dependency line) | Add `kairo_core.workspace = true` / `kairo_ui.workspace = true`. |
| `crates/zed/src/main.rs` | upstream | **High** (2 lines) | Add `kairo_core::init(cx);` + `kairo_ui::init(cx);` in the `app.run` closure (~776). Do not restructure the closure. |
| `crates/zed/src/zed.rs` | upstream | **High** (1 line) | Add `KairoPanel::load(...)` call inside `futures::join!` (~803). |
| `assets/settings/default.json`, `assets/keymaps/default-*.json` | upstream | **Very high** (large, frequently edited) | **Do not edit.** Register settings via `Settings::register` and keys via `cx.bind_keys` from Kairo crates. |
| `crates/project/src/project.rs`, `crates/project/src/lsp_store.rs` | upstream | Very high | Avoid editing. Register JDT LS/JSP through `LanguageRegistry` / `languages::init` paths or a Zed extension instead. |
| `crates/languages/src/*`, `extensions/*` | upstream | High | Prefer a separate Kairo-owned extension manifest/package over editing built-in language definitions. |
| `README.md` | upstream + harness checklist | n/a | Already contains the `> [!IMPORTANT]` review-checklist lines; leave them in place (AGENTS.md forbids removing them). |

### 7.2 Recommended integration shape

```
kairo/
  crates/
    kairo_core/        # HTTP client to Go agent, domain model, Settings, service entity,
                       #   actions, task definitions, process lifecycle
    kairo_ui/          # KairoPanel (workspace::Panel), status/log views, menu items
  docs/analysis/...    # these reports
```

Upstream patch surface (target): **5 lines across 4 files + regenerated `Cargo.lock`**:

1. root `Cargo.toml`: `members` += `"kairo/crates/kairo_core"`, `"kairo/crates/kairo_ui"` (2 lines);
   `[workspace.dependencies]` += 2 alias lines (can be folded into the same 2 lines if
   each alias is the only addition — count as 2–4 lines total).
2. `crates/zed/Cargo.toml`: 2 dependency lines.
3. `crates/zed/src/main.rs`: 2 `init` lines.
4. `crates/zed/src/zed.rs`: 1 panel-load line.

All other functionality (settings schema, keybindings, actions, tasks, LSP/DAP glue,
panel content) lives in the two new crates and is reachable solely through those
registration lines. Keep a single `kairo_bridge` feature flag in `crates/zed/Cargo.toml`
only if compile-time opt-out is required; otherwise avoid feature plumbing.

### 7.3 Rebase workflow

1. Keep `kairo/integration` as the only working branch; never commit fixes upstream.
2. On upstream sync: `git fetch upstream && git rebase upstream/main`.
3. Expect conflicts only in the four upstream files above; reapply the 5–9 marked
   lines (tag them with a consistent trailing comment, e.g. `// kairo: integration`,
   so they are trivially greppable after a rebase — note root `Cargo.toml` cannot hold
   Rust comments but Cargo `#` comments are fine).
4. Run `cargo check -p kairo_core -p kairo_ui -p zed` and `./script/clippy` after each
   rebase (repo build guideline: use `./script/clippy`, not bare `cargo clippy`).
5. Verify end-to-end against `legacy-sample` with real Tomcat 6 as described in
   `kairo/docs/PROGRESS.md`.

---

## Appendix — quick reference of exact symbols

| Need | Symbol | File:line |
|---|---|---|
| Register global service | `AppState::set_global`, `Client::set_global`, `cx.set_global` | `crates/zed/src/main.rs:492,597,667` |
| Project init/local | `Project::init`, `Project::local` | `crates/project/src/project.rs:1267,1303` |
| LSP store | `LspStore::init`, `LspStore::new_local` | `crates/project/src/lsp_store.rs:4845,4948` |
| Language registration | `LanguageRegistry::register_language`, `::add` | `crates/language/src/language_registry.rs:439,599` |
| Settings | `Settings` trait, `Settings::register` | `crates/settings/src/settings_store.rs:69,86` |
| Keybindings | `cx.bind_keys` | `crates/agent_ui/src/agent_panel.rs:9515` |
| Task scheduling | `Workspace::schedule_task`, `Workspace::spawn_in_terminal` | `crates/workspace/src/tasks.rs:44,228` |
| Task -> terminal bridge | `TerminalProvider::spawn`, `Workspace::set_terminal_provider` | `crates/workspace/src/workspace.rs:327,3545` |
| Task process spawn | `spawn_task_subprocess` | `crates/terminal/src/terminal.rs:3376` |
| Safe process spawn | `util::process::Child::spawn` | `crates/util/src/process.rs:33,56` |
| Command builder | `util::command::new_command`, `new_std_command` | `crates/util/src/command.rs:14,16` |
| Panel trait | `workspace::Panel` | `crates/workspace/src/dock.rs:36` |
| Add panel | `Workspace::add_panel` | `crates/workspace/src/workspace.rs:2948` |
| Panel load/add at startup | `initialize_workspace`, `add_panel_when_ready` | `crates/zed/src/zed.rs:430,788,794` |
| Example panel | `TerminalPanel::load`, `impl Panel` | `crates/terminal_view/src/terminal_panel.rs:237,1692` |
| Workspace init site | `workspace::init` | `crates/zed/src/main.rs:749`, def `workspace.rs:1046` |
