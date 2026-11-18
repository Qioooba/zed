# 04 — Completeness audit: FEATURE-MATRIX vs. actual implementation

- **Review date:** 2026-11-19
- **Reviewer:** completeness subagent (read-only; this report is the only file written)
- **Branch under review:** `kairo/integration` @ `5da1d9f`
- **Sources compared:** `docs/FEATURE-MATRIX.md`, `docs/analysis/01`–`03`, `docs/VERIFICATION.md`, all code under `kairo/crates/**`, `kairo/extensions/**`, `fixtures/**`, plus the tagged upstream registration lines in `Cargo.toml`, `crates/settings_content/src/settings_content.rs`, `crates/zed/Cargo.toml`, `crates/zed/src/main.rs`, `crates/zed/src/zed.rs`.
- **Strictness rule:** a capability counts as *implemented* only when a user can trigger it from Zed and it performs the real operation end-to-end. A working Go endpoint with no Zed caller is **backend-only**, not implemented. Dead client methods and declared-but-unregistered actions are findings, not coverage.

---

## 0. Headline verdict

The Go runtime agent is genuinely capable and genuinely verified (`docs/VERIFICATION.md` shows real compile / deploy / Tomcat 6 start / JSP serving / static JSP hot reload at the HTTP layer). **The Zed side is mostly a skeleton.** It has 14 declared actions, 12 registered handlers, a panel, a typed HTTP client with ~25 endpoint methods, and one WASM extension that truly registers Java/JSP languages. But the central workflow is **inoperable**: `active_project_id` is never assigned, so `compile`, `deploy`, and `start_server` always return `no active Kairo project`. Build state is never polled, errors are never stored, events are an empty stream, and the debug bridge is never started.

**Strict P0 result (41 P0 rows audited):**

| Classification | Count | Share |
|---|---:|---:|
| **Implemented** (real end-to-end from Zed) | 1 | 2.4 % |
| **Partially implemented** (code present but broken/incomplete) | 9 | 22.0 % |
| **Backend-only** (agent endpoint real, no Zed entry point) | 21 | 51.2 % |
| **Only planned / declared-but-dead** | 10 | 24.4 % |

Of the seven explicit user asks, **only syntax highlighting is truly wired into Zed**. Compile / launch / deploy / debug / hot-deploy / JDK-1.6-and-GBK-deep-compatibility exist as backend capabilities and/or nominal actions, but none complete a real user flow today.

---

## 1. Task 1 — P0 capability-by-capability classification

Legend: **I** = implemented end-to-end; **P** = partial; **B** = backend-only (agent real, Zed has no working entry point); **N** = only planned / declared but dead or miswired.

### 1. Workspace / project / toolchains (7 P0)

| # | Matrix row (claim) | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Open workspace, canonicalize, sandbox (`FEATURE-MATRIX.md:24`) | Planned | **B** | `client.rs:76 open_workspace` + `service.rs:215 open_workspace` exist but are called by **nothing**; no action, no worktree hook. |
| 2 | Detect legacy layout / `ScanProject` (`:25`) | Planned | **B / N** | `client.rs:91 scan_project`, `:99 detect_project` exist; no service method. `actions.rs:31 ScanProject` has **no registered handler** (grep: only definition). |
| 3 | Import wizard (`:26`) | Planned | **N** | `client.rs:107 import_project` is the only trace; no modal, no action, no UI. |
| 4 | `.kairo/project.yaml` source of truth (`:27`) | Reuse agent | **B** | Backend real; `service.rs:195` loads `Project` DTOs but `KairoPanel` never displays projects — panel shows only toolchains/servers/logs. |
| 5 | Three toolchain slots + fail-closed diagnostic (`:29`) | Reuse agent + UI | **P** | `settings.rs:11-13` has all three homes; `kairo_panel.rs:115-131` lists toolchains flat (no slot distinction). Settings values are **never sent to the agent** (no endpoint call, no env propagation). |
| 6 | Toolchain detect + SHA-256 fingerprint + import (`:30`) | Reuse agent | **B** | Agent real; panel shows "fingerprinted/unverified" string only; no import action. |
| 7 | No vendored JDK 6 / never fake compatibility (`:31`) | Planned | **N** | `client.rs:361 is_toolchain_incompatible` is **never called**; `service.rs:38 last_error` is only ever set to `None` (lines 58, 180, 208) — never `Some`. Panel error line is dead; compile errors go to `detach_and_log_err` only. Fail-closed invariant is **not enforced in the UI**. |

### 2. Editing / language intelligence (4 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 8 | Java completion/navigation via JDT LS (`:38`) | Planned | **P** | `extension.toml:17-23` registers `jdtls` for Java/JSP; `kairo_java.rs:45-51` falls back to `jdtls` on PATH because `KAIRO_JDTLS_LAUNCHER` is never set by any Zed code (`std::env::var` in a WASM host reads Zed's env, not the agent's). `client.rs:201 launch_descriptor` and `:208 generate_jdt_project` are **never called**. |
| 9 | JDT project model generator (`:39`) | Reuse agent + call before LS attach | **B** | `client.rs:208 generate_jdt_project` exists, zero callers. The "call before LS attach" step is absent. |
| 10 | JDT LS pinned/offline install, JDK 21 host, crash recovery (`:40`) | Reuse agent | **B** | Backend real; Zed consumes no descriptor and has no status surface. |
| 11 | **JSP syntax highlighting** (`:41`) | Planned | **I** | `extensions/kairo-java/languages/jsp/*` + `languages/java/*` + `extension.toml:9-15` genuinely register grammars, highlights, injections and a JSP language in Zed. Caveats: `injections.scm:3-4` injects Java into the whole `(jsp)` root rather than only `<% %>` scriptlets, and the claimed "verified" node names could not be re-verified here (network fetch of the pinned grammar's `node-types.json` failed). No equivalent of Kairo's deterministic `jsp-region-scanner` / EL / JSTL-specific captures. |

### 3. Encoding — byte fidelity (6 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 12 | Encoding detection at file-open (`:54`) | Reuse agent at file-open boundary | **B** | `client.rs:189 detect_encoding` has zero callers; **no file-open hook exists**. |
| 13 | 8 encodings dropdown (`:55`) | Planned | **N** | Only `settings.rs:16 encoding: String`; never sent to agent, no UI. |
| 14 | Save preserves encoding + EOL (`:56`) | Planned | **N** | **No save hook anywhere.** Highest data-loss risk: Zed will save GBK buffers as UTF-8. |
| 15 | Strict decode on write, no `?` (`:57`) | Reuse agent | **B** | Backend real but irrelevant to Zed writes because `/encoding/recode` is never called. |
| 16 | Atomic recode (`:58`) | Reuse agent | **B** | `client.rs:197 recode` — zero callers. |
| 17 | GBK full-text search (`:62`) | Reuse agent search | **B** | `client.rs:225 search` — zero callers. Zed's native search is UTF-8-only, so GBK projects are unsearchable from Zed. |

### 4. Build / compile (6 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 18 | Async real `javac` build, source/target 1.6 (`:68`) | Reuse agent + action | **P** | Action + `service.rs:235 compile` + `client.rs:119 start_build` exist, **but** (a) `active_project_id` is always `None` → every call errors, (b) `client.rs:123 build(build_id)` (result polling) has **zero callers**, so state stays `queued`, no diagnostics, and `deploy()` at `service.rs:266-271` can never find a `success` build → sends an empty `buildId`. |
| 19 | Ant provider (`:69`) | Reuse agent | **B** | Backend real. The only Zed-side `ant` invocation is a static fixture task (`fixtures/legacy-sample/.zed/tasks.json`), not a product path. |
| 20 | compat gate `toolchain_incompatible` (`:70`) | Reuse agent; surface in panel | **B / N** | Backend gate verified; Zed **does not surface** it (see #7). |
| 21 | Class-file major version gate (`:71`) | Reuse agent | **B** | Backend only. |
| 22 | GBK post-compile CJK verification (`:72`) | Reuse agent | **B** | Backend only. |
| 23 | Incremental single-file compile (`:73`) | Reuse agent; hot-deploy path | **B** | `client.rs:217 jvm_compile` — zero callers. |

### 5. Deploy / hot deploy (6 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 24 | Safe deploy: owner token, containment (`:82`) | Reuse agent | **B** | Backend real. |
| 25 | Merge / mirror modes (`:83`) | Reuse agent | **B** | Backend real. `DeployRequest.scope` is hardcoded `"incremental"` at `service.rs:276`. |
| 26 | Build & Deploy / Publish one-click (`:84`) | Planned | **N (miswired)** | `kairo_ui.rs:39-49` registers `BuildAndDeploy` but the handler calls **only `service.compile(false, …)`** — it never deploys. No `Publish` action exists at all. |
| 27 | Static hot sync on save (`:85`) | Reuse agent watcher; Zed save triggers sync | **B** | Backend watcher real; **no Zed save hook**, so the trigger never fires. |
| 28 | Compile-only sync → honest `restart_required` (`:86`) | Planned | **N** | No `hot_reload_mode` in panel state; `DeployResult.hot_reload_mode` is parsed at `domain.rs:272` but never read. `jvm_redefine`/`reload_server` have zero callers. |
| 29 | `hotreload.status` events (`:89`) | Planned | **N** | `client.rs:231 subscribe_events` returns `futures::stream::empty()` with a TODO comment; `AgentHandle::events_url()` (`agent.rs:76`) has zero callers. |

### 6. Tomcat 6 server lifecycle (7 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 30 | Start real Tomcat 6 (`:95`) | Reuse agent | **P** | Agent verified; `service.rs:287 start_server` wired to action, but blocked by the missing active project. |
| 31 | Stop / restart / debug / recover / reload (`:96`) | Reuse agent + actions | **P** | Start/stop work in principle; `RestartServer` action calls **`service.refresh`** (`kairo_ui.rs:111-121`) and never `client.rs:158 restart_server`; `debug_server`, `reload_server` have zero callers; no `recover` client method/action. |
| 32 | Process identity verification (`:97`) | Reuse agent | **B** | Backend only. |
| 33 | Paged server logs + stream (`:99`) | Planned | **P** | Better than claimed: `service.rs:319-341` polls `/servers/{id}/logs?tail=200` every 2 s with a GPUI timer and `kairo_panel.rs:143-150` renders the last 200 lines. No paging/load-more UI, and polling stops on first error. |
| 34 | Open Application in browser (`:100`) | Planned | **N / missing handler** | `actions.rs:29 OpenApplication` has **no handler**. `ServerResponse.url` is parsed (`domain.rs:300`) but unused. |
| 35 | Readiness probe (`:101`) | Reuse agent | **B** | Backend only. |
| 36 | JDWP loopback + JDI bridge attached (`:102`) | Planned | **N** | No code spawns `kairo-jdi-bridge.jar`. The extension attaches over TCP to a port nothing listens on (see §3/§5). |

### 7. Debug — DAP/JDWP (5 P0)

| # | Matrix row | Matrix status | Verdict | Evidence |
|---|---|---|---|---|
| 37 | DAP over stdio ↔ JDWP bridge (`:108`) | Planned | **P** | `extension.toml:25-26` + `kairo_java.rs:63-103` register a `java` DAP adapter — but as **TCP attach** (`resolve_tcp_template`) instead of the planned `java -jar … --stdio`, and no bridge process is launched. Cannot connect. |
| 38 | Breakpoints (`:109`) | Zed native | **P** | Native DAP UI exists in Zed (`crates/debug_panel`), but no working adapter for Kairo → unusable. |
| 39 | Variables / watches / call stack / stepping (`:110`) | Zed native | **P** | Same as above. |
| 40 | JDWP attach retry, bounded backoff (`:112`) | Reuse | **B** | Bridge implements it, but bridge is never started. |
| 41 | Java 6 gate probe before advertising debug (`:113`) | Planned | **N** | `client.rs:212 debug_adapter_status` has zero callers. No gating, no greying out. |

**Additional finding on cross-cutting invariants (`FEATURE-MATRIX.md:142-153`):** the invariant table claims "Event sequence/gap detection → `kairo_core::events`". **There is no `events` module** (`kairo_core.rs:4-9` lists actions, agent, client, domain, service, settings) and `subscribe_events` is an empty placeholder. "Gate debug before advertising" is likewise unimplemented. The invariants "UI never executes shell directly" is only partly honored (the static fixture task runs `ant` directly). "Fail-closed toolchain & encoding" is not enforced in Zed.

---

## 2. Task 2 — Kairo capabilities missing from FEATURE-MATRIX entirely

These are real Kairo capabilities (per `analysis/02` and `analysis/03`) with no matrix row, so they are silently dropped from the promised scope:

| Missing capability | Source | Impact if omitted |
|---|---|---|
| **SSH debug tunnel** (`/debug/tunnel/connect|disconnect`, build tag `remote`) | `analysis/02` §4.15, endpoints 74-75 | Remote debugging of legacy deployments cannot be planned; P3 but zero visibility. |
| **JDI bridge host-JDK provisioning** (`/debug/jdk/download`, `jdkmanager`, JDK 17+ requirement) | `analysis/02` §4.15, endpoint 76 | Matrix has "Java 6 gate probe" but not the *host* JDK the bridge needs. Explains why debug cannot work out of the box. |
| **JUnit test discovery + results tree** (`test-extension`, `kairo-test-results`, `/java/detect`) | `analysis/03` §2 row 19, §3.12 | Matrix's "Run Test" (P1) implies execution only; the discover→run→results workflow is absent from scope. |
| **HotSwap history + original-bytecode rollback** (`HotSwapManager`) | `analysis/02` §4.12 | Matrix lists only `RedefineClasses`; rollback/compatibility validation is invisible. |
| **Experimental JSP breakpoints** (`kairo.jsp.toggleBreakpoint`, Jasper CodeLens mapping, pref `kairo.jsp.debugBreakpoints`) | `analysis/03` §3.4, §5 | A distinct P2/P3 capability with no matrix row (also noted in `GAPS-BACKLOG.md` C3). |
| **Agent authentication / session / CSRF wiring** (shared secret, `X-Kairo-Secret`, login throttle) | `analysis/02` §4.7 | `client.rs:37-48 KairoClient::new(…, None)` always passes no secret and `agent.rs` never passes `-secret`; matrix's security invariants assume a boundary that is not configured. |
| **Agent self-restart + port handoff + operation registry / idempotency** (`/runtime/restart`, `/operations/{id}`) | `analysis/02` §4.1-4.2 | No Zed reconnection semantics beyond a manual `ReconnectAgent` action that calls `start_agent` (which is a no-op if a client already exists — `service.rs:146`). |
| **Unobserved-launch guard / async LaunchJob orchestration** (`/launch-jobs/{id}`, before-launch tasks) | `analysis/02` §4.2, endpoint 19 | Matrix's "Run configurations" P1 row mentions the document, not the orchestration; orphan-Tomcat protection is out of scope invisibly. |
| **Welcome view / agent-offline banner / shortcut cheat-sheet / focus & large-file commands** | `analysis/03` §3.18 | Pure UI, low impact, but these are explicitly promised Kairo features absent from the map. |
| **Native status-bar grouping / toolbar / IDEA menu structure** | `analysis/03` §3.1, §3.15 | Matrix has keymaps/themes P2/P3 but not the toolbar/status-bar composition that Kairo users actually navigate by. |

---

## 3. Task 3 — The user's explicit asks: truly wired vs. backend-only

| User ask | Truly wired into Zed? | What actually exists | What is missing |
|---|---|---|---|
| **Real click compile** | ❌ No (nominal only) | Action + service + typed `/builds` client; backend compile verified in `VERIFICATION.md` | `active_project_id` never set → always errors; build result never polled → no success/diagnostics; no error surfaced |
| **Launch (Tomcat 6)** | ❌ No | `StartServer` action → `start_server` → `/servers`; backend verified | Same missing active project; no gate check; no debug bridge |
| **Deploy** | ❌ No | `Deploy` action → `/deployments`; backend verified (49 files / 960 KB) | Requires a `success` build id that polling never produces; scope hardcoded |
| **Debug** | ❌ No (registration only) | `java` DAP adapter declared in `extension.toml`; JDWP bridge real in backend | Nothing starts `kairo-jdi-bridge.jar`; adapter TCP-attacks a dead port; `/debug/adapter/status` unused; no Java launch config |
| **Syntax highlighting** | ✅ **Yes** | Java + JSP tree-sitter grammars, `highlights.scm`, `injections.scm`, language configs, extension registration | Quality caveats only (over-broad Java injection; no Kairo region scanner / EL / JSTL-specific captures) |
| **Hot deploy** | ❌ No (nominal only) | Static-sync watcher and `/jvm/compile`, `/jvm/redefine`, `/servers/{id}/reload` are real in backend | `HotDeploy` action just calls `deploy()`; no save hook; no client callers for any hot-deploy endpoint; events are an empty stream |
| **JDK 1.6 / JSP deep compatibility** | ❌ No | Settings `source_level=1.6`, `target_level=1.6`, `encoding=gbk`, three toolchain homes; backend enforces compat and fails closed (verified) | Settings never sent to the agent; no encoding detect/recode hook; JDT LS never attached; JDT project model never generated |

**Conclusion:** 1 of 7 asks is genuinely wired into Zed; the other 6 are backend capabilities or nominal UI shells.

---

## 4. Task 4 — Action handler audit (`actions.rs` ↔ `kairo_ui`)

Declared in `kairo_core/src/actions.rs`: **14** actions.
Registered in `kairo_ui/src/kairo_ui.rs`: **12** handlers. Panel `toggle_action` returns `TogglePanel` (`kairo_panel.rs:92-94`).

| # | Action | Handler? | Registered operation | Sensible? |
|---|---|---|---|---|
| 1 | `Compile` | ✅ | `compile(false)` | ✅ Correct |
| 2 | `CleanBuild` | ✅ | `compile(true)` | ✅ Correct |
| 3 | `BuildAndDeploy` | ✅ | `compile(false)` only | ❌ **Wrong** — never deploys |
| 4 | `Deploy` | ✅ | `deploy()` | ⚠️ Correct call, but unusable (no build id) |
| 5 | `HotDeploy` | ✅ | `deploy()` | ❌ **Wrong** — alias of Deploy, not hot deploy |
| 6 | `StartServer` | ✅ | `start_server(false)` | ⚠️ Correct call, blocked by missing project |
| 7 | `StartDebugServer` | ✅ | `start_server(true)` | ⚠️ Partial — passes `debug=true` + `"debugPort=…"` config string, but starts no bridge and checks no gate |
| 8 | `StopServer` | ✅ | `stop_server(last server id)` | ⚠️ Correct call; picks last by insertion order, not active |
| 9 | `RestartServer` | ✅ | `refresh()` | ❌ **Wrong** — does not restart |
| 10 | `OpenApplication` | ❌ **none** | — | ❌ Missing handler; `ServerResponse.url` unused |
| 11 | `ScanProject` | ❌ **none** | — | ❌ Missing handler |
| 12 | `TogglePanel` | ✅ | `toggle_panel_focus::<KairoPanel>` (+ panel toggle_action) | ✅ Correct |
| 13 | `RefreshStatus` | ✅ | `refresh()` | ✅ Correct |
| 14 | `ReconnectAgent` | ✅ | `start_agent()` | ⚠️ No-op when a client already exists (`service.rs:146-148`) |

**Panel buttons (`kairo_panel.rs:166-180`):**

| Button | Calls | Sensible? |
|---|---|---|
| Compile | `compile(false)` | ✅ |
| Clean Build | `compile(true)` | ✅ |
| Deploy | `deploy()` | ⚠️ unusable |
| **Start Server** | **`start_agent()`** | ❌ **Wrong** — starts the Go agent, not Tomcat |
| Refresh | `refresh()` | ✅ |

**Keybinding gaps (`actions.rs:41-52`):** only 8 of 14 actions have keys. Missing bindings: `CleanBuild` is bound, but `BuildAndDeploy`, `OpenApplication`, `ScanProject`, `RefreshStatus`, `ReconnectAgent` have none — consistent with them being unfinished. `ctrl-alt-s` (StartServer) on Windows is a known risky chord, but that is a keymap-review concern.

**Dead client methods (no caller anywhere in `kairo/**`):** `open_workspace`, `scan_project`, `detect_project`, `import_project`, `build`, `cancel_build`, `restart_server`, `debug_server`, `reload_server`, `detect_encoding`, `recode`, `launch_descriptor`, `generate_jdt_project`, `debug_adapter_status`, `jvm_compile`, `jvm_redefine`, `search`, `subscribe_events`, `events_url`. That is **19 of ~25** endpoint methods with no Zed caller.

---

## 5. Task 5 — Top 10 gaps by user impact, with minimal implementations

Ordered by "how badly a real user is blocked / how much data can be lost".

### G1 — No active project: every core action fails immediately
- **Why #1:** `service.rs:36,56` — `active_project_id` is initialized to `None` and **never assigned**. `compile`, `deploy`, `start_server` all bail with `no active Kairo project`.
- **Minimal fix:** in `refresh()` and `open_workspace()`, match the active Zed worktree root against `Project.root` (canonicalized) and set `active_project_id`; add a `kairo::SelectProject` action / panel picker for ambiguity; call `refresh()` once after `start_agent()`.

### G2 — Build never completes from the client's perspective
- **Why:** `client.rs:123 build()` has no caller. State stays `queued`; diagnostics never arrive; `deploy()` builds an empty `buildId`.
- **Minimal fix:** after `start_build`, spawn a GPUI task polling `GET /builds/{id}` every 500 ms until a terminal state; update `service.builds`, publish diagnostics into Zed's diagnostics store, and only then allow `Deploy`/`BuildAndDeploy`.

### G3 — GBK save hook absent (silent data corruption)
- **Why:** no file-open or save boundary code; Zed buffers are UTF-8. A GBK JSP opened and saved becomes UTF-8 bytes while the agent still believes it is GBK.
- **Minimal fix:** add `kairo::SaveWithEncoding` action plus a save interceptor: call `/encoding/detect` on open, keep raw bytes for non-UTF-8 files, and route writes through `/encoding/recode` (atomic, strict). Never use `String::from_utf8_lossy`. Add a status-bar encoding segment sourced from `EncodingDetection`.

### G4 — Two declared actions have no handler; three are miswired
- **Why:** `OpenApplication`, `ScanProject` have no handlers; `BuildAndDeploy` only compiles; `HotDeploy` only deploys; `RestartServer` only refreshes; the panel's "Start Server" button starts the agent.
- **Minimal fix:** register all 14 actions; `OpenApplication` → `cx.open_url(server.url)`; `ScanProject` → new `scan_workspace` service method calling `/workspaces/{id}/scan` and populating the panel; `RestartServer` → `client.restart_server`; panel "Start Server" → `start_server(false)`; separate `build_and_deploy` / `hot_deploy` service methods.

### G5 — Debug is completely non-functional from Zed
- **Why:** nothing spawns `kairo-jdi-bridge.jar`; the extension attaches over TCP to a port with no listener; `/debug/adapter/status` is unused.
- **Minimal fix:** (a) call `/debug/adapter/status` in `StartDebugServer` and fail closed with a clear message; (b) after starting the debug server, spawn `java -jar <bundled>/kairo-jdi-bridge.jar 127.0.0.1 <debugPort>` via `util::process::Child` held by `KairoService` (process-tree cleanup on drop); (c) change `get_dap_binary` to launch the bridge with DAP-over-stdio as originally specified in `FEATURE-MATRIX.md:108`, not TCP attach.

### G6 — JDT LS never starts, so Java/JSP intelligence is unavailable
- **Why:** `KAIRO_JDTLS_LAUNCHER` is an agent-process env var; a WASM extension cannot see it. No Zed code calls `/jdtls`, `/jdtls/project`, or `launch_descriptor`.
- **Minimal fix:** on workspace open, have `KairoService` call `/jdtls` prepare + `/jdtls/project` + `GET /workspaces/{ws}/java/launch-descriptor`; persist the returned `command`/`args`/`envAllowlist` to a Zed settings file (or a known agent-state file) that `kairo_java.rs` reads in `language_server_command`, and feed `initialization_options` from the descriptor.

### G7 — Event stream is an empty placeholder
- **Why:** `client.rs:231-237 subscribe_events` returns `stream::empty()`; no `kairo_core::events` module exists despite the matrix's invariant table. No `hotreload.status`, no `event.gap`.
- **Minimal fix:** implement a WebSocket client for `/api/v1/events` with the `kairo-secret-v1` subprotocol (or, as a minimum viable step, poll a status endpoint and synthesize gap detection with the `sequence` field already in `KairoEvent`). Add an `events` module and correct the invariant table's file reference.

### G8 — Fail-closed errors never reach the UI
- **Why:** `last_error` is write-only-to-`None`; all task failures go through `detach_and_log_err` to the console; `is_toolchain_incompatible` is unused. Users see neither `toolchain_incompatible` nor "real JDK 6 required".
- **Minimal fix:** replace `detach_and_log_err` with explicit `update` calls that store `format!("{err}")` in `last_error`; in `KairoPanel`, branch on `is_toolchain_incompatible` and render targeted guidance. Never degrade an agent error to a generic status.

### G9 — Hot deploy has no Zed-side trigger or honesty mapping
- **Why:** no save hook; `jvm_compile`, `jvm_redefine`, `reload_server` have zero callers; `DeployResult.hot_reload_mode` is parsed but unused.
- **Minimal fix:** hook buffer save → if `.java`, call `/jvm/compile`, else `/deployments` static sync; map the result to `synced` / `compiling` / `restart_required` in panel state; call `/jvm/redefine` only when a debug session is attached; make `HotDeploy` use this path, never plain `deploy()`.

### G10 — Settings (JDK 1.6 / encoding / three toolchains) are decorative
- **Why:** `KairoSettings` fields are parsed but never sent to the agent — no toolchain selection, no `-source/-target`, no encoding, no port preference. The three-slot model from ADR-0006 is not represented in the UI.
- **Minimal fix:** pass these into the agent spawn env and/or the build/start request payloads (`compiler_java_home` → JDK selection, `source_level`/`target_level` → build request, `encoding` → detect/recode default, `tomcat_http_port`/`debug_port` → start request); show the three slots distinctly in `KairoPanel` with a `toolchain_incompatible` badge per slot.

*Just outside the top 10 (lower user impact but still matrix P0/P1):* import wizard, GBK search routing, JDT project-model generation, run-configuration launch orchestration, JDI host-JDK provisioning, JUnit results tree, debug gate, i18n/themes, and the missing matrix rows from §2.

---

## 6. Coverage estimate (final answer style)

- **Strict end-to-end P0 coverage:** **1 / 41 ≈ 2 %** (only JSP syntax highlighting).
- **Counting partial implementations at half credit:** ≈ **14 %**.
- **Backend reuse coverage (agent endpoint exists behind the matrix row):** **31 / 41 ≈ 76 %** — consistent with `VERIFICATION.md`, but these are *not* accessible from Zed.
- **Explicit user asks wired:** **1 / 7 ≈ 14 %** (syntax highlighting only).
- **Action hygiene:** 12/14 declared actions have handlers, but **6 of those 12 are wrong or ineffective**; plus 1 wrong panel button. Effective correct action coverage ≈ **7 / 14 = 50 %**, and even those 7 are blocked by G1/G2.
- **Client endpoint utilization:** ~6 / 25 used (≈ 24 %) — 19 typed methods are dead code.

---

## 7. Caveats / limits of this audit

- No Rust toolchain is installed on this machine (per `PROGRESS.md` / `VERIFICATION.md`), so **compilation was not verified**. This audit is static: it assesses wiring and completeness, not whether the crates build. Reviewer 01 covers compile errors.
- The pinned `tree-sitter-jsp` grammar's `node-types.json` could not be fetched (network error), so the highlight/injection query node names from `highlights.scm`/`injections.scm` are reported as *claimed* by the code comment, not independently verified. Reviewer 02 covers query correctness.
- Concurrent reports `03-sync-review.md` and `GAPS-BACKLOG.md` were read for cross-checking only; this report is based on independent inspection of the source and is the only file I wrote.
- Nothing in this report modifies or contradicts the backend evidence in `docs/VERIFICATION.md`: the Go agent really does the work. The finding is that **Zed does not call it**.
