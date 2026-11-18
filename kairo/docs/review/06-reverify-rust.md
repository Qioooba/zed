# 06 — Second-pass Rust re-verification (kairo_core + kairo_ui)

> Independent re-check of the CURRENT source on branch `kairo/integration` against
> the prior report `01-rust-static-review.md` and the **actual** Zed source in
> this checkout (`crates/gpui`, `crates/workspace`, `crates/editor`,
> `crates/language`, `crates/multi_buffer`, `crates/http_client`, `crates/scheduler`,
> `crates/settings_content`, `crates/icons`, `crates/ui`).
>
> No Rust toolchain is installed on this machine (`rustc`/`cargo` not found),
> so this remains a static, signature-level verification. Every claim below is
> backed by a concrete upstream signature read from the checkout.

---

## Verdict

**COMPILES-LIKELY.** All six prior BLOCKERs and all four prior ERRORs are
fixed in the current source. No new guaranteed compile error was introduced by
the fixes: every changed call site was checked against the real Zed API
(signatures, trait bounds, trait-object method resolution, borrow scoping, and
edition-2024 let-chains). Two non-blocking items remain from the prior
WARNING list (one partial fix, one unfixed unused import); neither denies
compilation under the workspace lint configuration.

**Remaining blocker count: 0.**

---

## Prior BLOCKERs — all fixed

| # | Issue | Current state | Evidence |
|---|-------|---------------|----------|
| 1 | `observe_new` window param | **FIXED** — `kairo_ui.rs:17` now takes `_window: Option<&mut Window>`. | `gpui/src/app.rs:2754-2757` requires `Fn(&mut T, Option<&mut Window>, &mut Context<T>)`. Exact match; closure captures nothing so `'static` holds. |
| 2 | `TaskExt` not in scope for `detach_and_log_err` | **FIXED** in all three files. `kairo_ui.rs:7` and `kairo_panel.rs:7` import `prelude::*`; `service.rs:14` imports `TaskExt as _`. | `gpui/src/prelude.rs:8` re-exports `TaskExt as _`. `gpui/src/executor.rs:55-83` defines the trait/blanket impl; bounds `E: Display + Debug` satisfied by `anyhow::Error`. |
| 3 | `FluentBuilder` import path | **FIXED** — no explicit import; `prelude::*` (`kairo_panel.rs:7`) brings it in. | `gpui/src/prelude.rs:8` re-exports `util::FluentBuilder`. `gpui/src/gpui.rs:151` exports only `FutureExt`/`Timeout` at root — confirming the prior report's concern, now avoided. |
| 4 | `on_click` on un-id'd `Div` | **FIXED** — `kairo_panel.rs:56-66` uses `.id(label)` then `.on_click(...)`. | `div.rs:803` `fn id(...) -> Stateful<Self>`; `From<&'static str> for ElementId` at `window.rs:8188`; `Stateful<E>` implements `StatefulInteractiveElement` (`div.rs:4185`) whose `on_click` (`div.rs:1637`) takes `Fn(&ClickEvent, &mut Window, &mut App)`. `cx.listener` returns exactly that (`app/context.rs:241-244`). `Stateful<E>: ParentElement` so `.child(label)` still works; `&'static str: Element` (`text.rs:246`) ⇒ `IntoElement`. |
| 5 | `WeakEntity::read` in `start_log_polling` | **FIXED** — `service.rs:813-818` uses `this.read_with(cx, ...).ok()`. | `gpui/src/app/entity_map.rs:918` `WeakEntity::read_with<C,R>(&self, &C, impl FnOnce(&T,&App)->R) -> Result<R>`. No `WeakEntity::read` exists. |
| 6 | `WeakEntity::read` in `Panel::enabled` | **FIXED** — `kairo_panel.rs:123-125` uses `self.workspace.read_with(cx, |_, _| true).is_ok()`. | Same `read_with` signature; `App: AppContext` (`app.rs:3450`). |

---

## Prior ERRORs — all fixed

| # | Issue | Current state | Evidence |
|---|-------|---------------|----------|
| 1 | "Start Server" button started the agent | **FIXED** — `kairo_panel.rs:275-277` calls `service.start_server(false, cx)`. | `KairoService::start_server(bool, &mut Context<Self>) -> Task<Result<()>>` at `service.rs:609`. |
| 2 | Panel never observed the service | **FIXED** — `kairo_panel.rs:21,30-36` stores `_subscriptions: Vec<Subscription>` and registers `cx.observe(&service, |_, _, cx| cx.notify())`. | `Context<T>::observe(&mut self, &Entity<W>, impl FnMut(&mut T, Entity<W>, &mut Context<T>))` at `app/context.rs:63-76` — 3-arg closure matches. `Subscription` at `gpui::` root. |
| 3 | Deploy posted empty `buildId` | **FIXED** — `service.rs:515-524` uses `iter().find(|b| b.state == "success")` with `let … else { return Task::ready(Err(anyhow!(…))) }`. | `Task::ready` exists (`scheduler/src/executor.rs:577,686`, re-exported through `gpui::executor`). Return type `Task<Result<()>>` inferred correctly. |
| 4 | Placeholder / missing action handlers | **FIXED** — all 18 actions in `actions.rs:10-50` have `register_action` handlers in `kairo_ui.rs:18-241`, each calling a real service method. | Verified: `BuildAndDeploy → build_and_deploy` (`service.rs:400`), `HotDeploy → hot_deploy(None, …)` (`service.rs:556`, delegates to `deploy` when no Java file), `RestartServer → restart_server` (`service.rs:735`), `OpenApplication → application_url()` + `cx.open_url(&url)` (`service.rs:165`, `app.rs:2050`), `ScanProject → start_agent + open_workspace` (`service.rs:209,335`). `Workspace::register_action<A: Action>(impl Fn(&mut Self,&A,&mut Window,&mut Context<Self>))` at `workspace.rs:8943-8946` — all closures match. |

No other declared action is unhandled: Compile, CleanBuild, BuildAndDeploy, Deploy, HotDeploy, StartServer, StartDebugServer, StopServer, RestartServer, ReloadContext, OpenApplication, ScanProject, SelectProject, SaveWithEncoding, SearchEncoded, RefreshStatus, ReconnectAgent, TogglePanel — 18/18.

---

## Prior WARNINGs

| # | Issue | Current state |
|---|-------|---------------|
| 1 | Secret header never sent | **FIXED (primary)** — `client.rs:312-314` (`put`) and `client.rs:333-335` (`request_json`) add `X-Kairo-Secret` when `self.secret` is `Some`. Residual design gap (not a compile error): `spawn_agent` still does not pass a `-secret` flag; the secret is sourced only from env `KAIRO_LOCAL_SECRET` (`service.rs:229`). |
| 2 | Blocking fs I/O on foreground | **FIXED** — `agent.rs:138-172` uses `smol::fs::{create_dir_all, remove_file, read_to_string}` (all async). `smol.workspace = true` present at `kairo_core/Cargo.toml:26`. |
| 3 | Silent error swallowing in `refresh` | **FIXED** — `service.rs:263-268` uses `match` with `log::warn!`. `log.workspace = true` present at `kairo_core/Cargo.toml:22`. |
| 4 | `scan_project` empty payload | **FIXED** — `client.rs:91-97` posts `{"rootPath": root_path}`. |
| 5 | Unused `Task` import | **NOT FIXED** — `kairo_panel.rs:7` still imports `Task`; grep confirms no other `Task` use in that file. Warning only: `[workspace.lints.rust]` (`Cargo.toml:1179-1180`) does not deny `unused_imports` or `warnings`. |

NIT 1 (globally scoped keybindings, `actions.rs:54-59` still uses `context: None`) is unchanged; out of scope for this pass and non-blocking.

---

## New compile-error scan (fixes only)

Every API touched by the fixes was checked against real source; no new guaranteed error found:

- **`window.rs` API chain** (`kairo_ui.rs:153-156, 182-190`): `Workspace::visible_worktrees(&self,&App) -> impl Iterator<Item=Entity<Worktree>>` (`workspace.rs:3781-3786`); `Worktree::abs_path() -> Arc<Path>` (`worktree.rs:871`) → `to_string_lossy()`; `Workspace::active_item_as::<Editor>(&App) -> Option<Entity<Editor>>` (`workspace.rs:4554`); `Editor::buffer() -> &Entity<MultiBuffer>` (`editor.rs:3107`); `MultiBuffer::as_singleton() -> Option<Entity<Buffer>>` (`multi_buffer.rs:1436`); `Buffer::file() -> Option<&Arc<dyn File>>` (`language/buffer.rs:1523`).
- **`file.file_system_abs_path(cx)`** on `&Arc<dyn File>`: `language::File::file_system_abs_path(&self,&App) -> Option<PathBuf>` (`language/buffer.rs:381`). No `use language::File` in `kairo_ui.rs`, but this is a trait-object method call — rustc treats principal-trait methods as inherent on `dyn Trait`, so no import is required (confirmed against Rust Reference method resolution and rustc issue #51402).
- **`workspace.toggle_panel_focus::<KairoPanel>(window, cx)`** (`workspace.rs:4934-4938`): expects `&mut Window, &mut Context<Workspace>` — matches the `TogglePanel` closure.
- **`cx.open_url(&url)`** (`app.rs:2050`, `fn open_url(&self, url: &str)`) — `&String` coerces to `&str`.
- **Entity update closures**: `Entity::update` is `FnOnce` (`entity_map.rs:577-583`), so moving `root` into the `ScanProject`/`SaveWithEncoding` closures is legal. Immutable borrows from `read`/`visible_worktrees` end before the `update(cx, …)` mut borrow.
- **`KairoPanel::load`** (`kairo_panel.rs:39-48`): `WeakEntity::update_in<C: AppContext>` (`entity_map.rs:899-912`) accepts `AsyncWindowContext: AppContext` (`async_context.rs:396`); `Context<T>: AppContext` provides `new` (`context.rs:756-760`); `AsyncWindowContext: Clone` (`async_context.rs:295`). Call site `zed.rs:784` passes `(WeakEntity<Workspace>, AsyncWindowContext)` — matches.
- **`Panel` trait** (`dock.rs:36-104`): all implemented methods (`persistent_name`, `panel_key`, `position`, `position_is_valid`, `set_position`, `default_size`, `icon`, `icon_tooltip`, `toggle_action`, `activation_priority`, `enabled`) match current signatures; defaults cover the rest. `icons::IconName == ui::IconName` (`ui/src/components/icon.rs:14` `pub use icons::*`), `IconName::TerminalAlt` exists (`icons.rs:259`).
- **`Render` body** (`kairo_panel.rs:128-316`): `SharedString::from(&str)` (`gpui_shared_string.rs:130`) resolves `.map(SharedString::from)`; `FluentBuilder::when_some` (`gpui/src/util.rs:42`); `&'static str: IntoElement` via `impl Element for &'static str` (`text.rs:246`) + blanket `IntoElement` (`element.rs:145`).
- **HTTP client** (`client.rs`): `http_client` re-exports `Method`, `Request` (`http_client.rs:5`); `HttpRequest::builder().uri(&uri).header(...)` accepts `&str`; `AsyncBody: From<String>` and `AsyncBody: AsyncRead + Unpin` (`async_body.rs:14,114`); let-chains valid on edition 2024 (`Cargo.toml:287`).
- **Settings/deps**: `KairoSettingsContent` fields (`settings_content/src/settings_content.rs:1101-1126`) match `from_settings` (`settings.rs:42-80`); both crates are workspace members/deps (`Cargo.toml:270-271,309-310`; `zed/Cargo.toml:205-206`); all `*.workspace = true` deps (`log`, `smol`, `editor`, `client`, `http_client`, `dirs`, `futures`, `gpui`, `settings`, `icons`, `kairo_core`, `workspace`, `anyhow`, `serde`, `serde_json`, `util`) exist in `[workspace.dependencies]`.
- **Hygiene**: no `unwrap()`, `expect()`, `panic!`, `let _ =`, `todo!`, or `dbg!` anywhere under `kairo/crates`. No `mod.rs` files; library roots declared via `[lib] path`.

---

## Remaining non-blocking items

1. `kairo/crates/kairo_ui/src/kairo_panel.rs:7` — remove unused `Task` import (prior WARNING 5). No lint denies it.
2. `kairo/crates/kairo_core/src/service.rs:229` + `kairo/crates/kairo_core/src/agent.rs:151-163` — for externally started agents configured with a secret, no `-secret` flag is threaded through `AgentLaunchConfig`; only `KAIRO_LOCAL_SECRET` env is read. Runtime/design gap, not a compile error.

**Final verdict: COMPILES-LIKELY — 0 remaining blockers.**
