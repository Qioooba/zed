# 01 — Rust static review (kairo_core + kairo_ui + registration edits)

> Static review of branch `kairo/integration`. No Rust toolchain was available, so
> nothing was compiled. Every claim below was checked against the **actual**
> Zed source in this checkout (`crates/gpui`, `crates/settings`,
> `crates/settings_macros`, `crates/settings_content`, `crates/http_client`,
> `crates/util`, `crates/client`, `crates/workspace`). Severity is ordered:
> **BLOCKER** = guaranteed compile error, **ERROR** = concrete logic/behavior
> bug, **WARNING** = likely runtime/design problem, **NIT** = style/hygiene.

---

## BLOCKER 1 — `observe_new` closure takes `&mut Window` instead of `Option<&mut Window>`

**File:** `kairo/crates/kairo_ui/src/kairo_ui.rs:16-17`

```rust
cx.observe_new(
    |workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>| {
```

**Problem:** The second parameter is passed as `Option<&mut Window>`, not
`&mut Window`. The closure's explicit type therefore does not implement the
required `Fn` trait.

**Evidence:** `crates/gpui/src/app.rs:2754-2757`:

```rust
pub fn observe_new<T: 'static>(
    &self,
    on_new: impl 'static + Fn(&mut T, Option<&mut Window>, &mut Context<T>),
) -> Subscription
```

Real call sites handle the `Option`, e.g. `crates/agent_ui/src/inline_assistant.rs:68-71`:

```rust
cx.observe_new(|_workspace: &mut Workspace, window, cx| {
    let Some(window) = window else { return; };
```

**Suggested fix** (omit the annotation or use `Option`, and unwrap where needed):

```rust
cx.observe_new(
    |workspace: &mut Workspace, _window, cx: &mut Context<Workspace>| {
        // `_window: Option<&mut Window>` — none of the registered actions need it.
        ...
    },
)
.detach();
```

---

## BLOCKER 2 — `detach_and_log_err` used without `TaskExt` in scope (≈16 call sites)

**Files:**
- `kairo/crates/kairo_ui/src/kairo_ui.rs:24,35,46,57,68,79,90,106,118,129,140`
- `kairo/crates/kairo_ui/src/kairo_panel.rs:167,170,173,176,179`

```rust
service.compile(false, cx).detach_and_log_err(cx);
```

**Problem:** `detach_and_log_err` is **not** an inherent method on `Task`; it is
a method of the `TaskExt` trait. Neither file imports `TaskExt`, so every call
is `no method named detach_and_log_err found for struct Task<...>`.

**Evidence:** `crates/gpui/src/executor.rs:52-83` defines
`pub trait TaskExt<T, E> { fn detach_and_log_err(self, cx: &App); }` and the
blanket impl for `Task<Result<T, E>>`. It is exposed through
`crates/gpui/src/prelude.rs:8` (`TaskExt as _`) but **not** at the `gpui` root.
Real users import it explicitly, e.g. `crates/agent/src/agent.rs:52-55`:

```rust
use gpui::{App, AppContext, AsyncApp, Context, Entity, EntityId, SharedString, Subscription, Task,
    TaskExt, WeakEntity};
```

**Suggested fix** — add the import to both files:

```rust
// kairo_ui.rs
use gpui::{App, Context, TaskExt, Window};
// kairo_panel.rs — add TaskExt to the existing gpui::{...} list
use gpui::{ ..., Task, TaskExt, Window, div, px };
```

---

## BLOCKER 3 — `FluentBuilder` is not exported from the `gpui` root

**File:** `kairo/crates/kairo_ui/src/kairo_panel.rs:6`

```rust
use gpui::{ ..., FluentBuilder, ... };
```

**Problem:** Unresolved import. `FluentBuilder` is defined in the private
`gpui::util` module and is **not** re-exported at the crate root.

**Evidence:** `crates/gpui/src/gpui.rs:60` declares `mod util;` (private), and
line 151 re-exports only `pub use util::{FutureExt, Timeout};`. The trait is
exposed via `crates/gpui/src/prelude.rs:8` (`util::FluentBuilder`) and re-exported
by `ui` (`crates/ui/src/prelude.rs:3` `pub use gpui::prelude::*;`). Real call
sites use `gpui::prelude::FluentBuilder` or `ui::FluentBuilder`, e.g.
`crates/project/src/lsp_command.rs:16`.

**Suggested fix:**

```rust
use gpui::{App, ..., Styled, Task, Window, div, px, prelude::FluentBuilder};
```

---

## BLOCKER 4 — `on_click` on a plain `Div` (trait missing + needs `.id()`)

**File:** `kairo/crates/kairo_ui/src/kairo_panel.rs:214-223`

```rust
div()
    .px_2()
    .py_1()
    .rounded_md()
    .border_1()
    .on_click(cx.listener(move |_panel, _event, _window, cx| { ... }))
    .child(label)
```

**Problem:** The fluent `on_click` is **not** available on `Div`:
1. `StatefulInteractiveElement` (which owns the fluent `on_click`) is not imported.
2. `Div` does not implement `StatefulInteractiveElement` — only `Stateful<E>` and
   `Img` do, and an element id is required for click handling.

**Evidence:** `crates/gpui/src/elements/div.rs:1637` defines `fn on_click(...)` inside
`pub trait StatefulInteractiveElement` (lines 1353-1762). Implementations are
`impl<E> StatefulInteractiveElement for Stateful<E>` at line 4185 and
`impl StatefulInteractiveElement for Img {}` at `img.rs:521`. `Div` only gets
`impl InteractiveElement for Div` (line 1979), and `InteractiveElement`
(792-1349) has **no** `on_click`. The doc at `div.rs:2228` states an element id
is required for stateful interactivity such as `on_click`.

**Suggested fix** (give the element an id and import the trait):

```rust
use gpui::{..., InteractiveElement, StatefulInteractiveElement, ...};

div()
    .id(label)                       // -> Stateful<Div>
    .px_2().py_1().rounded_md().border_1()
    .on_click(cx.listener(move |_panel, _event, _window, cx| { ... }))
    .child(label)
```

(`Stateful<E>` implements `ParentElement` at `div.rs:4277`, so `.child` still
works after `.on_click()`.)

---

## BLOCKER 5 — `WeakEntity::read` does not exist (`start_log_polling`)

**File:** `kairo/crates/kairo_core/src/service.rs:324`

```rust
let Some(client) = this.read(cx).client.clone() else {
    return;
};
```

**Problem:** `this` is `WeakEntity<KairoService>` (the `Context::spawn` closure
receives `WeakEntity<T>`). `WeakEntity` has **no** `read` method.

**Evidence:** `crates/gpui/src/app/entity_map.rs:870-934` — the full
`impl<T: 'static> WeakEntity<T>` provides only `upgrade`, `update`, `update_in`,
`read_with`, `new_invalid`. (`read` at line 565 is on `Entity<T>`, and the
`read` at `window.rs:7929` is on a window root-view handle.) A repo-wide search
finds no `WeakEntity::read`.

**Suggested fix:**

```rust
let Ok(Some(client)) = this.read_with(cx, |service, _| service.client.clone()) else {
    return;
};
```

---

## BLOCKER 6 — `WeakEntity::read` does not exist (`Panel::enabled`)

**File:** `kairo/crates/kairo_ui/src/kairo_panel.rs:100-102`

```rust
fn enabled(&self, cx: &App) -> bool {
    self.workspace.read(cx).is_ok()
}
```

**Problem:** `self.workspace` is `gpui::WeakEntity<Workspace>`; no `read` method
exists (same evidence as BLOCKER 5).

**Suggested fix:**

```rust
fn enabled(&self, _cx: &App) -> bool {
    self.workspace.is_upgradable()   // AnyWeakEntity::is_upgradable, entity_map.rs:684
}
```

---

## ERROR 1 — "Start Server" button starts the *agent*, not the server

**File:** `kairo/crates/kairo_ui/src/kairo_panel.rs:175-177`

```rust
.child(self.action_button("Start Server", cx, |service, cx| {
    service.start_agent(cx).detach_and_log_err(cx);
}))
```

**Problem:** Wrong action → method wiring. The keyboard action `StartServer`
correctly calls `service.start_server(false, cx)` (`kairo_ui.rs:79`), but the
panel button labelled "Start Server" only spawns the runtime agent and never
starts Tomcat. The user clicks "Start Server" and no server is created.

**Suggested fix:**

```rust
.child(self.action_button("Start Server", cx, |service, cx| {
    service.start_server(false, cx).detach_and_log_err(cx);
}))
```

(If agent autostart is desired, chain it or rely on `auto_start_agent`.)

---

## ERROR 2 — Panel never observes the service, so it never re-renders

**Files:** `kairo/crates/kairo_ui/src/kairo_panel.rs:18-45` (struct + `new`/`load`)

**Problem:** `render()` reads `KairoService::global(cx)` and the service calls
`cx.notify()` on every state change, but `KairoPanel` registers **no**
`cx.observe(&service, ...)` subscription. GPUI does not track which entities an
element read during `render`; only the observed entity's `notify()` drives a
re-render. Consequently the status/toolchain/server/log text is frozen at the
first frame until some unrelated invalidation occurs.

**Evidence:** `Context::notify` (`app.rs:3273`) invalidates only observers;
`cx.observe` is the standard mechanism (see any Zed panel, e.g.
`crates/agent_ui/src/agent_panel.rs`, which keeps `_subscriptions`).

**Suggested fix** — hold the subscription and notify the panel on service changes:

```rust
pub struct KairoPanel {
    workspace: gpui::WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<gpui::Subscription>,
}

impl KairoPanel {
    pub fn new(workspace: ..., _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let service = KairoService::global(cx);
        let subscriptions = vec![
            cx.observe(&service, |_, _, cx| cx.notify()),
        ];
        Self { workspace, focus_handle: cx.focus_handle(), _subscriptions: subscriptions }
    }
}
```

---

## ERROR 3 — Deploy sends an empty `buildId` when there is no successful build

**File:** `kairo/crates/kairo_core/src/service.rs:266-271`

```rust
let build_id = self
    .builds
    .first()
    .filter(|build| build.state == "success")
    .map(|build| build.id.clone())
    .unwrap_or_default();
```

**Problem:** `.unwrap_or_default()` turns "no successful build" into `""`. The
task continues and POSTs `{"buildId": ""}` to `/api/v1/deployments`, producing a
confusing agent error instead of an actionable local message. The same guard
pattern is used correctly for `client` and `active_project_id` via `Task::ready(Err(...))`.

**Suggested fix:**

```rust
let Some(build_id) = self
    .builds
    .iter()
    .find(|build| build.state == "success")
    .map(|build| build.id.clone())
else {
    return Task::ready(Err(anyhow!("no successful Kairo build to deploy")));
};
```

(Also note `self.builds.first()` only inspects the most recent build; `iter().find(...)`
is more correct.)

---

## ERROR 4 — Several declared actions are placeholders / mis-wired

**File:** `kairo/crates/kairo_ui/src/kairo_ui.rs:39-49, 61-71, 111-121`

- `BuildAndDeploy` (39-49) only calls `compile(false, ...)` — never deploys.
- `HotDeploy` (61-71) only calls `deploy(...)` — no incremental compile / honesty
  about `restart_required`.
- `RestartServer` (111-121) calls `refresh(...)` — never `POST /servers/{id}/restart`.
- `kairo::OpenApplication` and `kairo::ScanProject` are declared in
  `actions.rs:28,30` but have **no** `register_action` handler at all.

**Evidence:** The repo's own `kairo/docs/review/GAPS-BACKLOG.md:11-15` lists
exactly these as A1–A5 ("has no handler", "only compiles", "only calls deploy",
"only calls refresh").

**Suggested fix:** Add the missing service methods (`build_and_deploy`,
`hot_deploy`, `restart_server`, `scan_project`, `open_application`) using the
existing client methods (`client.rs:128,158,91,...`) and wire them to the
actions. This is required for the feature claims in `PROGRESS.md` to hold.

---

## WARNING 1 — `secret` is never sent; secured agents will reject every call

**File:** `kairo/crates/kairo_core/src/client.rs:27-49, 257-295`

`KairoClient` stores `secret: Option<String>` but `request_json` never emits an
`X-Kairo-Secret` header. Per `kairo/docs/analysis/02-kairo-runtime-agent.md:82`,
only `health` and `endpoints` are public; all other endpoints require the shared
secret when the agent is configured with one. `spawn_agent` also never passes a
`-secret` flag, so a user-supplied external agent cannot be driven. The local
`VERIFICATION.md` run omitted the secret, so this was not exercised.

**Suggested fix:** send the header when present:

```rust
if let Some(secret) = &self.secret {
    builder = builder.header("X-Kairo-Secret", secret.as_str());
}
```

and thread a secret through `AgentLaunchConfig`/settings.

---

## WARNING 2 — Blocking filesystem I/O on the foreground executor

**File:** `kairo/crates/kairo_core/src/agent.rs:93-143`

`spawn_agent` is `async` but performs `std::fs::create_dir_all`, `remove_file`,
and `read_to_string` directly, and is awaited inside a foreground `cx.spawn`
(`service.rs:165-166`). This blocks the UI thread for up to 30 s while polling
the state file every 150 ms.

**Suggested fix:** move the blocking work to `executor.background_spawn(...)`
(or use `smol::fs`) and await the result on the foreground.

---

## WARNING 3 — Silent error swallowing in `refresh`

**File:** `kairo/crates/kairo_core/src/service.rs:198-201`

```rust
if let Ok(server_list) = client.get_servers_for_workspace(&workspace.id).await {
    servers.extend(server_list);
}
```

Per-project server-list failures are discarded with no logging, contradicting
the repo rule "never silently discard errors ... use `.log_err()` when you need
visibility".

**Suggested fix:**

```rust
match client.get_servers_for_workspace(&workspace.id).await {
    Ok(server_list) => servers.extend(server_list),
    Err(error) => log::warn!("failed to list Kairo servers for {}: {error:#}", workspace.id),
}
```

(Requires `log.workspace = true` in `kairo_core/Cargo.toml`.)

---

## WARNING 4 — `scan_project` posts an empty payload to a strict endpoint

**File:** `kairo/crates/kairo_core/src/client.rs:91-97`

```rust
self.post(&format!("/api/v1/workspaces/{workspace_id}/scan"), &serde_json::json!({}))
```

The analysis (`02-kairo-runtime-agent.md:93`) documents this endpoint as
`{rootPath?,deep?}` **(strict)**. An empty object may be rejected. The real
verification run does not record the request body, so this is unproven.

**Suggested fix:** send at least the known root path, e.g.
`&serde_json::json!({ "rootPath": root_path })`, and add the `deep` flag when
needed.

---

## WARNING 5 — Unused import

**File:** `kairo/crates/kairo_ui/src/kairo_panel.rs:7`

`Task` is imported but never used in the file (the only `Task` uses are in
`kairo_ui.rs`). With workspace lints this is at most a warning, but remove it.

---

## NIT 1 — Globally-scoped keybindings

**File:** `kairo/crates/kairo_core/src/actions.rs:41-52`

All `KeyBinding::new(..., None)` calls use `context: None`, registering global
bindings such as `ctrl-alt-b`, `ctrl-alt-s`, `ctrl-alt-d`. On Windows/Linux these
can collide with Zed defaults. Prefer a Kairo-specific predicate, e.g.
`Some("KairoPanel")`, once the panel has a key context.

## NIT 2 — Repo hygiene rules otherwise respected

- No `unwrap()`, `expect()`, `panic!`, or `let _ =` on fallible operations in
  the new crates (checked across `kairo/crates`).
- No `mod.rs` files; library roots are declared via `[lib] path = ...`.
- Full variable names are used throughout; `cx`/`fs`-style conventional
  abbreviations are acceptable.
- Root `README.md` already carries the required `[!IMPORTANT]` review lines
  (`PROGRESS.md:12` confirms this was done).

---

## Registration edits — verified OK

- `Cargo.toml:270-272,309-310`: both crates are workspace members and declared
  in `[workspace.dependencies]`. All `*.workspace = true` deps used by the new
  crates (`anyhow`, `client`, `dirs`, `futures`, `gpui`, `http_client`, `serde`,
  `serde_json`, `settings`, `util`, `icons`, `kairo_core`, `workspace`) exist in
  `[workspace.dependencies]`. No dependency cycle (`kairo_ui -> kairo_core` only;
  `zed -> {kairo_core, kairo_ui}`).
- `settings::RegisterSetting` derive (`settings_macros.rs:85-105`) requires only
  `impl Settings` (present) and `settings` in dependencies (present); the
  generated `settings::private::*` paths resolve (`settings/src/settings.rs:21-25`).
- `SettingsContent.kairo: Option<KairoSettingsContent>`
  (`settings_content.rs:203`) and the `KairoSettingsContent` derives
  (`settings_content.rs:1099-1126`: `Clone, Default, Serialize, Deserialize,
  JsonSchema, MergeFrom, Debug, PartialEq` + `#[with_fallible_options]`) satisfy
  `SettingsContent`'s own derives (`Debug, PartialEq, Default, Clone, Serialize,
  JsonSchema, MergeFrom`, line 173). `KairoSettings::from_settings`
  (`kairo_core/src/settings.rs:41-81`) accesses matching field names. Settings
  registration order in `kairo_core::init` (register → bind keys → init global)
  is correct.
- `http_client` API usage is consistent: `HttpClient::send` returns
  `BoxFuture<Result<Response<AsyncBody>>>` (`http_client.rs:123-126`);
  `AsyncBody: From<String>` + `empty()` (`async_body.rs:32,72`); `AsyncBody:
  AsyncRead` (`async_body.rs:114`) so `AsyncReadExt::read_to_end` works;
  `client::Client::global(cx).http_client()` returns `Arc<HttpClientWithUrl>`,
  which coerces to `Arc<dyn HttpClient>` (`http_client.rs:344`). Let-chains are
  valid on Rust 1.98 / edition 2024.
- `util::process::Child::spawn` (`util/src/process.rs:33,56`) and
  `util::command::new_std_command` (`util/src/command.rs:14`) signatures match;
  `dirs::state_dir()` is used elsewhere unconditionally
  (`crates/paths/src/paths.rs:180`, behind `cfg!` not `#[cfg]`), so it is
  available on all targets.
- `Panel` required methods (`crates/workspace/src/dock.rs:36-104`) are all
  implemented; `DockPosition::{Bottom, Right}` exist (line 323-327);
  `icons::IconName` == `ui::IconName` via `ui/src/components/icon.rs:14`;
  `IconName::TerminalAlt` exists (`icons/src/icons.rs:259`).
  **But** see BLOCKER 6 (`WeakEntity::read` in `enabled`) and ERROR 2
  (missing observation).

---

## Verdict

**This branch will not compile.** There are at least six independent,
guaranteed compile errors: the `observe_new` window parameter (BLOCKER 1),
`TaskExt` not in scope for `detach_and_log_err` (BLOCKER 2), the `FluentBuilder`
import (BLOCKER 3), `on_click` on an un-id'd `Div` without the trait (BLOCKER 4),
and two uses of the non-existent `WeakEntity::read` (BLOCKERS 5 and 6). Any one
of these aborts `cargo check -p kairo_core -p kairo_ui -p zed`.

Beyond compilation, the panel is wired to the wrong method for "Start Server"
(ERROR 1) and will never refresh its displayed state (ERROR 2), and several
declared actions are placeholders (ERROR 4). The settings, HTTP, process,
workspace-registration and DTO layers are otherwise consistent with the checked
out Zed source; the field names match the real agent responses documented in
`VERIFICATION.md` and `docs/analysis/02-kairo-runtime-agent.md`.

**Confidence:** high on all six blockers (each backed by a concrete upstream
signature); medium on the warnings (they depend on the real agent's runtime
configuration, which was not re-exercised here).
