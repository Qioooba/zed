# Gaps backlog — full implementation queue

> Working list kept while the 5 review agents run. Every item here will be
> implemented (or explicitly proved as not applicable) before this round ends.
> Severity/priority will be reconciled with `docs/review/0*-*.md` when they land.

## A. Zed-side entry points declared but not wired

| # | Gap | Planned implementation |
|---|-----|------------------------|
| A1 | `kairo::OpenApplication` action has no handler | `kairo_ui::init` handler opens `server.url` via Zed's `OpenUrl` / `cx.open_url` |
| A2 | `kairo::ScanProject` action has no handler | Handler calls `KairoService::scan_workspace` → `POST /workspaces/{id}/scan`, populates detected layout in panel |
| A3 | `kairo::BuildAndDeploy` only compiles | Add `KairoService::build_and_deploy` chaining compile → poll build terminal state → deploy |
| A4 | `kairo::HotDeploy` only calls deploy | Add `KairoService::hot_deploy` using `/jvm/compile` + `/deployments` + honestly surface `restart_required` |
| A5 | `kairo::RestartServer` only calls refresh | Add `POST /servers/{id}/restart` client + service method |

## B. Legacy-semantics gaps

| # | Gap | Planned implementation |
|---|-----|------------------------|
| B1 | No GBK save hook — Zed buffers are UTF-8, saving a GBK file can corrupt it | Implement a save-routing service: on buffer save, detect encoding via `/encoding/detect`; for non-UTF-8, send bytes through `/encoding/recode` (atomic). Hook via Zed project save events if available, else provide explicit `kairo::SaveWithEncoding` action + settings guard |
| B2 | Event stream absent (`subscribe_events` returns empty stream) | Implement WS `/api/v1/events` with `kairo-secret-v1` subprotocol using a real Zed websocket crate if available; otherwise keep 2 s poll but add explicit `event.gap` detection and log it |
| B3 | Debug gate not checked | `KairoService::check_debug_adapter` calls `/debug/adapter/status`; `StartDebugServer` action verifies availability first and surfaces a clear message on failure |
| B4 | `toolchain_incompatible` shown as generic error in panel | Add typed error display in `KairoPanel` with explicit "real JDK 6 required" guidance |
| B5 | Hot-reload honesty not surfaced | Add `hot_reload_mode` / `restart_required` field to panel state from deploy result and compile-only events |

## C. Language intelligence gaps (P1/P2 from FEATURE-MATRIX)

| # | Gap | Planned implementation |
|---|-----|------------------------|
| C1 | JDT LS launch descriptor from agent unused by extension | Expose descriptor over a small agent endpoint/file and have `kairo-java` read `KAIRO_JDTLS_LAUNCHER`; add initialization options from descriptor |
| C2 | JSP scriptlet completion / EL / TLD / JSP↔Servlet navigation | Requires a Kairo LSP proxy that maps JSP regions to virtual Java docs; implement as a new `kairo_jsp_ls` crate or extend the extension with a JSP language server that forwards to JDT LS (bounded scope: start with scriptlet completion + EL hover) |
| C3 | JSP breakpoints (experimental) | `debugger.scm` query + CodeLens; lower priority, keep as P2 if not feasible |

## D. Secondary Kairo features (P2/P3)

| # | Gap | Planned implementation |
|---|-----|------------------------|
| D1 | SVN commands | Zed tasks wrapping `svn` CLI + keybindings (P3) |
| D2 | Oracle SQL console | Agent `/sql/*` endpoints + a small SQL panel tab (P2) |
| D3 | Maven view | Agent `/maven/*` endpoints + task provider (P2) |
| D4 | Search routing for GBK projects | Route `/api/v1/search` when project encoding != UTF-8 (P1) |
| D5 | i18n en/zh-CN status strings | Zed-owned locale files (P2) |

## E. Process

- [ ] Reconcile this list with the 5 reviewer reports.
- [ ] Implement A + B first (core asks), then C, then D as time permits.
- [ ] Commit each logical group with a clear message.
- [ ] Re-review after fixes; update `PROGRESS.md`.
