# Gaps backlog — implementation status

> Reconciled with review reports `01`–`05`. Items marked DONE are implemented
> in commits after `5da1d9f`; verification is static until a Rust toolchain is
> available.

## A. Zed-side entry points

| # | Gap | Status | Evidence |
|---|-----|--------|----------|
| A1 | `OpenApplication` handler | **DONE** | `kairo_ui.rs` → `cx.open_url(server.url)` |
| A2 | `ScanProject` handler | **DONE** | worktree root → `open_workspace` → `/scan` |
| A3 | `BuildAndDeploy` only compiled | **DONE** | `KairoService::build_and_deploy` + `pending_deploy` consumed on build success |
| A4 | `HotDeploy` was plain Deploy | **DONE** | `hot_deploy(java_file)` → `/jvm/compile` for `.java`, static deploy otherwise; honesty text `compileOnly → restart_required` |
| A5 | `RestartServer` only refreshed | **DONE** | `restart_server` → `POST /servers/{id}/restart` |
| A6 | `SelectProject`, `SaveWithEncoding`, `SearchEncoded`, `ReloadContext` missing | **DONE** | actions + handlers added |

## B. Legacy semantics

| # | Gap | Status | Evidence |
|---|-----|--------|----------|
| B1 | No GBK save path | **DONE (explicit)** | `SaveWithEncoding` action → `/encoding/detect` → `/encoding/recode` (strict, atomic). Automatic Ctrl+S interception is **not** implemented (Zed has no will-save hook); panel shows current encoding so the user knows to use the command |
| B2 | Event stream stub | **PARTIAL** | Bounded log polling with GPUI timers; `event.gap` / `snapshot.required` over WS not implemented (no WS client wired) |
| B3 | Debug gate not checked | **DONE** | `start_server(debug=true)` checks `/debug/adapter/status` first, fails closed with message |
| B4 | Fail-closed error invisible | **DONE** | `set_error` stores real code; build polling reads diagnostic codes; panel renders `ERROR:` line with JDK6 guidance |
| B5 | Hot-reload honesty absent | **DONE** | `hot_reload_mode` shown in panel; `compileOnly → restart_required` |
| B6 | Debug bridge never started | **DONE** | `spawn_debug_bridge` launches `java -jar kairo-jdi-bridge.jar 127.0.0.1 <port>`; DAP uses the agent-reported debug port |
| B7 | Settings decorative | **DONE** | source/target/encoding pushed via `PUT /projects/{id}`; Java home/ports via `StartServerRequest`; panel shows all three slots |

## C. Language intelligence

| # | Gap | Status | Evidence |
|---|-----|--------|----------|
| C1 | JDT LS automatic launch | **PARTIAL** | Extension supports `lsp.jdtls.binary` override; service calls `/jdtls/prepare`; automatic launch descriptor → extension handoff needs a host-file bridge (WASM cannot read host FS). User can configure binary once |
| C2 | JSP scriptlet/EL/TLD completion | **PLANNED** (P2) | Requires Kairo LSP proxy mapping JSP regions to virtual Java docs |
| C3 | JSP breakpoints | **PLANNED** (P3) | experimental in Kairo too |

## D. Secondary features

| # | Gap | Status |
|---|-----|--------|
| D1 | SVN commands | PLANNED (P3) |
| D2 | Oracle SQL console | PLANNED (P2; agent endpoints real) |
| D3 | Maven view | PLANNED (P2; agent endpoints real) |
| D4 | GBK search routing | DONE — `KairoService::search` sends `projectEncoding` to `/search` |
| D5 | i18n en/zh-CN | PLANNED (P2) |

## E. Real-machine verification performed

- Real Zulu JDK 6 (`javac 1.6.0-119`) compiled `source/target=1.6` → 7 files, class major=50.
- Real Tomcat 6 served JSP 200; JSP hot edit visible in 3 s without restart.
- GBK detection on a real JSP: confidence 0.95, `eol=crlf`.
- See `docs/VERIFICATION.md`.

## F. Remaining hard blockers

1. **Rust toolchain absent** — no `cargo`/`rustc`, so no compile proof. Last reviewer (01) will list static API issues; fix them before declaring done.
2. **WASM ↔ host file bridge** — needed for automatic JDT LS launcher handoff.
3. **Automatic Ctrl+S GBK interception** — needs an upstream save hook (would enlarge the patch surface); explicit command is the current safe path.
