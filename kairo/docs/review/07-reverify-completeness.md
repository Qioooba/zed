# 07 — Completeness re-audit of the prior top-10 gaps

- **Review date:** 2026-11-19
- **Reviewer:** completeness re-auditor (read-only; this report is the only file written)
- **Branch under review:** `kairo/integration` @ `fdbdf2f` (prior audit was `5da1d9f`)
- **Method:** strict re-check of `kairo/crates/kairo_core/src/**`, `kairo/crates/kairo_ui/src/**`, `kairo/extensions/kairo-java/**`, `GAPS-BACKLOG.md`, `UX-DESIGN.md`, `FEATURE-MATRIX.md`, `VERIFICATION.md`, `01-rust-static-review.md`, and the tagged upstream registration lines. Same strictness rule as report 04: a capability counts only when a user can trigger a real operation end-to-end from Zed. Backend capability plus a nominal action is not coverage.
- **Compile status:** still **not compile-verified** — `cargo`/`rustc` are absent on this machine. The six guaranteed compile blockers from report 01 appear to have been fixed in commit `3009412` (verified statically: `Option<&mut Window>` signature, `TaskExt`/`prelude::*` imports, `.id(label)` on the clickable div, `read_with` instead of `WeakEntity::read`), but this remains a static assessment.

---

## 0. Headline verdict

The Zed side has grown substantially: from 14 actions / 12 handlers to **18 actions / 18 registered handlers**, build-result polling is real, fail-closed errors reach the panel, the debug bridge is actually spawned, settings are pushed to the agent, and the panel's action wiring is mostly corrected. **Two of the ten prior top gaps are genuinely resolved (G2, G8). Five are partial. Three remain materially open (G1, G6, G7).**

However, the single most important gap — **G1, no active project on a fresh setup** — is still not resolved. The auto-select in `refresh()` only works if the agent already has a project, and nothing in the Zed flow ever imports one (`POST /projects/import` and `/toolchains/import` are never called). Worse, the only user path that starts the agent, `kairo::ScanProject`, has an async bootstrap race that makes its `open_workspace` call fail on the first press. Consequently a first-time user still cannot reach Compile / Deploy / Launch without having pre-provisioned the agent from the external Kairo IDE.

Strict end-to-end coverage of the seven explicit user asks is therefore **still 1 / 7 ≈ 14 %** (syntax highlighting only). If operations that genuinely drive the agent *when prerequisites exist* are credited, coverage rises to **5 / 7 ≈ 71 %**, but that headline is misleading because four of those five share the G1 bootstrap blocker.

---

## 1. Strict re-verdict on G1–G10

| Gap | Prior state | Now | Strict verdict | Evidence / reasoning |
|---|---|---|---|---|
| **G1** No active project | `active_project_id` never assigned | Auto-select added, but incomplete | **NOT RESOLVED (critical)** | `service.rs:282-285` sets the active project only when `projects.len() == 1`; a fresh agent data dir has **zero** projects. `ScanProject` (`kairo_ui.rs:148-165`) calls `open_workspace` → `/scan`, which only fills `detected` (`service.rs:362-391`) — `import_project` (`client.rs:107`) and the toolchain import endpoint have **zero callers**, and `VERIFICATION.md:31` shows project creation is a separate `POST /projects/import`. `SelectProject` (`kairo_ui.rs:166-176`) is a no-op alias of `refresh()` — it cannot select a project. Additionally `start_agent` is async (`service.rs:224-247`) while `open_workspace` is invoked in the same closure (`kairo_ui.rs:159-161`), so on the first press `self.client()` is still `None` and `open_workspace` returns `agent is not running` (`service.rs:336-338`). |
| **G2** Build never completes | `client.build()` had no caller | Polling implemented | **RESOLVED** (gated by G1) | `service.rs:439-506` polls `GET /builds/{id}` every 500 ms for up to 120 s, updates `builds`, state text, pulls the first diagnostic code into `last_error`, and fires the deferred deploy on success. `client.rs:123 build()` is now called. Caveat: diagnostics are surfaced only as a code + raw output string in the panel error line — no Zed diagnostics store, no build-history tab, no cancel (`client.rs:128 cancel_build` still dead). |
| **G3** GBK save hook / data corruption | No hook at all | Explicit command added, automatic save still absent | **PARTIAL (high)** | `SaveWithEncoding` → `save_active_file_with_encoding` (`service.rs:859-896`) does `/encoding/detect` → `/encoding/recode` (strict, atomic in the agent). But there is **no will-save interceptor**, so ordinary `Ctrl+S` still writes Zed's UTF-8 buffer. More seriously, the command only passes the **file path** from the editor buffer (`kairo_ui.rs:182-197`); it never passes the in-memory (possibly edited) buffer contents, so it recodes the *old on-disk bytes* rather than saving the user's edits. `GAPS-BACKLOG` B1 is honest about the Ctrl+S gap but overstates "save". |
| **G4** Missing / miswired actions | 2 handlers missing, 3 miswired | 18/18 handlers registered; several still inert | **PARTIAL** | Real now: `BuildAndDeploy` → `build_and_deploy` with `pending_deploy` (`service.rs:400-403,495-498`), `RestartServer` → `restart_server` (`kairo_ui.rs:109-122`), `ReloadContext` → `reload_server`, `OpenApplication` → `cx.open_url`, `ScanProject` → agent + workspace. Still wrong: `SelectProject` = refresh, `SearchEncoded` = refresh (`kairo_ui.rs:200-212`), `ReconnectAgent` = refresh — none performs its named operation. `HotDeploy` passes `None` (`kairo_ui.rs:62-72`), so the `.java → /jvm/compile` branch (`service.rs:560-579`) has **no caller** and the action is still an alias of `deploy()`. Panel "Start Server" is now correctly `start_server(false)` (`kairo_panel.rs:275-277`); keybindings exist for only 6 of 18 actions. |
| **G5** Debug non-functional | No bridge, TCP attach to dead port | Gate + bridge spawn + attach wired, conditional | **PARTIAL** | `start_server(true)` calls `/debug/adapter/status` and fails closed (`service.rs:629-645`), then `spawn_debug_bridge` launches `java -jar kairo-jdi-bridge.jar 127.0.0.1 <port>` and waits for TCP (`agent.rs:95-124`, `service.rs:663-695`); the DAP adapter TCP-attaches (`kairo_java.rs:63-103`). Blockers to a real out-of-box flow: the bridge jar is **not bundled anywhere in `kairo/**`** and with an empty `bundled_dir` the code falls back to a bare relative filename; the agent's returned `ports.debug` is used for the bridge but **not fed into the debug config** (still defaults to 8000 in `java.json` / `kairo_java.rs:72-76`), and no Java debug launch config is contributed — the user must author one manually. |
| **G6** JDT LS never starts | No descriptor / prepare call | `/java/prepare` now called; descriptor still unused | **NOT RESOLVED** | `activate_project` calls `client.prepare_jdtls` (`service.rs:319-330`), but `launch_descriptor` / `generate_jdt_project` / `jdtls_status` (`client.rs:219-244`) still have **zero callers**. The WASM extension still reads `KAIRO_JDTLS_LAUNCHER` from its own env (`kairo_java.rs:42-51`), which the agent cannot set; only a manually configured `lsp.jdtls.binary` works. No host-file bridge exists. `GAPS-BACKLOG` C1 correctly marks this PARTIAL. |
| **G7** Event stream empty | `stream::empty()`, no events module | Still empty; bounded log polling only | **NOT RESOLVED** | `client.rs:265-271` still returns `futures::stream::empty()`; `AgentHandle::events_url` (`agent.rs:76`) has no caller; `kairo_core.rs:4-9` has no `events` module. `GAPS-BACKLOG` B2 is honest that WS, `event.gap`, `snapshot.required` and `hotreload.status` are absent. Log tail polling (`service.rs:808-832`) is a log-view feature, not an event stream. |
| **G8** Fail-closed errors never reach UI | `last_error` write-only-to-`None` | Errors stored and rendered | **RESOLVED** | `set_error` stores `format!("{error:#}")` (`service.rs:201-205`) and is called on every failing task; build polling synthesizes the diagnostic code and adds JDK-6 guidance for `toolchain_incompatible` (`service.rs:456-494`); the panel renders an `ERROR:` line (`kairo_panel.rs:249-255`) and a dedicated toolchain-slots view. The prior "false green" is gone. |
| **G9** Hot deploy no trigger / honesty | No save hook, mode parsed but unused | Mode shown; trigger still missing | **PARTIAL** | `DeployResult.hot_reload_mode` now drives panel state (`service.rs:533-540`, `kairo_panel.rs:236-247`) and `hot_deploy(Some(java_file))` emits `compileOnly → restart_required` (`service.rs:567-570`) — but that branch is unreachable from any action. No buffer-save → sync hook, `jvm_redefine` / `reload_server` (the latter only via a separate `ReloadContext` action) are not part of the hot-deploy flow, and the `HotDeploy` action is plain `deploy()`. |
| **G10** Settings decorative | All settings ignored | Levels/encoding + Tomcat settings now sent | **PARTIAL** | `activate_project` pushes `sourceLevel`/`targetLevel`/`encoding` via `PUT /projects/{id}` (`service.rs:313-321`, `client.rs:192-198`); `start_server` sends Tomcat JDK home and ports (`service.rs:616-624`). But `compiler_java_home` and `language_server_java_home` are **display-only** (the latter is even used to fabricate a fake `debug_gate.host_jdk` at `service.rs:652-657`), there is no per-slot `toolchain_incompatible` badge, toolchain import is absent, `auto_start_agent` (`settings.rs:19,36,76-78`) is read but never used, and project settings are only pushed in the single-project auto-select case. |

**Score: 2 RESOLVED, 5 PARTIAL, 3 NOT RESOLVED.**

---

## 2. New critical finding not in the prior top-10

**Fresh-user bootstrap is broken end-to-end.** Combining G1 with the startup race:

1. There is no startup hook that starts the agent (`auto_start_agent` is dead), so the only trigger is `kairo::ScanProject`.
2. On the first press, `start_agent()` spawns an async task and returns before `service.client` is set; the immediately following `open_workspace()` bails with `Kairo runtime agent is not running` and is only logged (`kairo_ui.rs:159-161`, `service.rs:336-338`).
3. Even on a second press (when the client exists), `/scan` does not import a project or a toolchain. With an empty data dir, `list_projects()` returns nothing, so `active_project_id` stays `None` and every compile/deploy/start returns `no active Kairo project`.

`VERIFICATION.md:31,78-79` confirms project import and toolchain import are required, separate operations. Until the Zed side exposes them, all compile/launch/deploy/debug paths depend on the user having already configured the agent from the external Kairo IDE (which shares the data dir). This is the highest-impact remaining gap and it invalidates the DONE claims for `GAPS-BACKLOG` A2 and, in practice, A3–A6 and B3–B7 as *first-run* capabilities.

Secondary new findings: `SelectProject` / `SearchEncoded` / `ReconnectAgent` are no-op aliases of `refresh()`; `HotDeploy`'s Java branch is dead code; `SaveWithEncoding` recodes on-disk bytes without writing the edited buffer; the JDI bridge jar is not bundled and the agent's debug port is not propagated into the DAP config; the secret is now sent as `X-Kairo-Secret` (`client.rs:312-314,333-335`) but `spawn_agent` still never passes `-secret` (`agent.rs:151-163`), so a secured external agent is still undrivable.

---

## 3. FEATURE-MATRIX capabilities still unimplemented, by user impact

### Critical — block the core legacy workflow
- **Import wizard / project import** (matrix :26) — no modal, `import_project` has no caller. Combined with the scan gap, no fresh project can be created from Zed.
- **Toolchain detect + import** (:30) — no `/toolchains/import` caller; fresh data dir has no JDK, so a real JDK 6 cannot be selected from Zed.
- **Three-slot model enforced** (:29) — compiler JDK / LS JRE shown but never sent anywhere; no per-slot fail-closed badge or import action.
- **JDT LS attach via launch descriptor** (:38–40) — prepare called, descriptor never consumed; no JDT LS auto-start.
- **Event sequence/gap detection + `hotreload.status`** (:89, invariant :151) — still an empty stream, no `events` module.
- **Save preserves encoding + EOL on normal save** (:56) — no will-save hook; explicit command does not save the edited buffer (data-loss risk).
- **Static hot sync on save / compile-only honesty trigger** (:85–86) — no Zed save-to-sync path; Java incremental branch unreachable.
- **DAP-over-stdio ↔ JDWP bridge with agent-reported port** (:108, :102) — bridge is TCP and requires an external, non-bundled jar and a hand-written debug config with the right port.

### High — degrade or hide important operations
- **GBK full-text search** (:62, :122) — `service.search` / `client.search` are dead; `SearchEncoded` action only refreshes status. GBK projects remain unsearchable from Zed.
- **Build view: history, diagnostics in Zed store, cancel** (:74) — only a status string; `cancel_build` dead; no diagnostics publishing.
- **Merge/mirror deploy mode + Publish** (:83–84) — `scope` hardcoded `"incremental"`; no Publish action; `BuildAndDeploy` is the only chained flow.
- **Debug gate "grey out before advertising"** (:113) — gate is checked at start, but no action is disabled/greyed proactively.
- **Paged logs + NDJSON stream** (:99) — bounded 200-line tail polling, no paging/load-more and no event stream (report 04 already called this partial).
- **Process identity verification / readiness probe surfacing** (:97, :101) — backend-only, no panel state.

### Medium — secondary but promised
- Per-file > folder > extension > project encoding override precedence (:59); 8-encoding dropdown (:55); `.properties` ISO-8859-1 handling (:60); encoding tab suffix/status segment (:61).
- Project selector / MRU (:28) — `SelectProject` is inert.
- Run configurations, before-launch / custom build commands (:32, :75–76) — no task provider; unobserved-launch guard absent.
- Class HotSwap via JDWP `RedefineClasses` when a debugger is attached (:87) — `jvm_redefine` has no caller.
- JSP scriptlet/EL/TLD completion, navigation, outline, snippets, CodeLens (:42–48) — all P2, unimplemented; highlighting itself remains the only language feature (with the over-broad JS injection caveat from report 04).
- Regex/replace-preview and streaming search (:122–123).

### Low — explicitly deferred / P3
- SVN, Oracle SQL console, Maven view, i18n, themes, IDEA keymaps, local history/bookmarks (:132–140) — still PLANNED.
- Capabilities missing from the matrix entirely (report 04 §2): SSH debug tunnel, JDI host-JDK provisioning, JUnit discover→run→results, HotSwap history/rollback, experimental JSP breakpoints, agent self-restart/port handoff, launch-job orchestration, welcome/offline banner.
- `auto_start_agent` setting is parsed but unused.

---

## 4. New coverage estimate for the 7 explicit user asks

| User ask | Strict end-to-end from Zed (no hidden prereqs) | If agent prerequisites already exist |
|---|---|---|
| **Compile** | ❌ No — fresh setup has no active project (G1) + first-press race | ✅ Real `javac` build, polled to terminal state; errors surfaced |
| **Launch (Tomcat 6)** | ❌ No — same G1 blocker; panel button now correct | ✅ Real Tomcat 6 start via `/servers` |
| **Deploy** | ❌ No — requires a successful build id that only a working compile produces | ✅ Real deployment via `/deployments` |
| **Debug** | ❌ No — bridge jar not bundled, port not handed to DAP config, manual config needed | ⚠️ Conditional — gate + real bridge spawn + TCP attach work if jar/port/config align |
| **Syntax highlighting** | ✅ **Yes** — Java + JSP grammars, highlights, injections, language registration | ✅ Yes |
| **Hot deploy** | ❌ No — action is plain deploy; no save hook; Java incremental branch unreachable | ❌ No distinct hot-deploy operation from Zed (static sync only via agent's own watcher) |
| **JDK 1.6 / GBK deep compatibility** | ⚠️ Partial — compat errors surfaced, explicit recode exists, but normal Ctrl+S still corrupts and JDT LS never starts | ⚠️ Partial — levels/encoding reach the agent; real JDK 6 must be imported externally |

**Numbers:**
- **Strict, no hidden prerequisites: 1 / 7 ≈ 14 %** (unchanged from report 04 — highlighting only).
- **Counting operations that genuinely call the agent when prerequisites are met: 5 / 7 ≈ 71 %** (highlight, compile, launch, deploy, debug-conditional). Treat this as an upper bound: four of those five are unusable by a first-time user until G1 is fixed.
- **Fully working without caveats: 1 / 7; partial: 2 / 7 (debug, JDK/GBK); absent: 1 / 7 (hot deploy); conditional on G1: 3 / 7 (compile, launch, deploy).**

---

## 5. GAPS-BACKLOG claims that do not hold under strict re-audit

| Claim | Re-audit |
|---|---|
| A2 `ScanProject` DONE | Overstated: open_workspace fails on first press (race) and scan never imports a project. |
| A4 `HotDeploy` DONE | Overstated: the `.java → /jvm/compile` branch has no caller; action = deploy. |
| A6 `SelectProject` / `SearchEncoded` DONE | Overstated: both handlers only call `refresh()`; neither performs its named operation. |
| B1 GBK save DONE (explicit) | Partly true: recode is real but operates on on-disk bytes, not the edited buffer; Ctrl+S still corrupts. |
| B3 debug gate DONE | True for the check itself; the full debug flow still needs an unbundled jar, port handoff and manual config. |
| B6 debug bridge DONE | Partly true: spawn is real, but no jar ships with `kairo/**` and the agent port is not propagated to the DAP config. |
| B7 settings DONE | Overstated: compiler JDK and LS JRE slots are display-only; `auto_start_agent` unused. |
| C1 JDT LS PARTIAL | Accurate. |
| B2 event stream PARTIAL | Accurate (log polling only). |

---

## 6. Caveats

- Static audit only: no Rust toolchain is available, so no compile or UI/E2E verification was performed. The compile blockers of report 01 appear fixed in `3009412`, but that is source inspection, not a passing build.
- Judgment about whether `/workspaces/{id}/scan` auto-creates a project is based on `VERIFICATION.md`, which performs `POST /projects/import` and `/toolchains/import` as explicit separate steps; the Zed code never calls them. If the agent version in use auto-imports on scan, G1 would be less severe, but that is not evidenced in this checkout.
- The `X-Kairo-Secret` header is now sent by the client (improvement over report 04), but the agent is still spawned without a secret flag; secured external agents remain unsupported.
- Nothing here changes the backend evidence in `VERIFICATION.md`: the Go agent genuinely compiles, deploys, serves JSP and hot-reloads. The finding remains that the Zed side does not let a fresh user reach those capabilities end-to-end.
