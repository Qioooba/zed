# 11 — Exhaustive frontend-omission audit (Kairo IDE → Zed port)

- **Audit date:** 2026-11-19
- **Role:** omission auditor (read-only; this report is the **only** file written by this agent)
- **Branch:** `kairo/integration`, HEAD `203fdb9`, plus in-flight working-tree edits to
  `kairo_core/src/service.rs`, `kairo_ui/src/kairo_ui.rs`, `kairo_ui/src/kairo_panel.rs`
  (the parent session was editing the GBK-save wiring while this audit ran; all findings below
  reflect the **current working tree**, not the earlier `fdbdf2f` snapshot used by reports 04/07).
- **Method:** full read of `03-kairo-frontend-features.md` (all 22 packages, all §3 feature areas,
  §4 E2E, §5 REAL/PARTIAL), `01-kairo-philosophy.md` §6.1 (18 invariants) and §6.2/§6.3,
  `FEATURE-MATRIX.md`, `UX-DESIGN.md`, and every file under `kairo/crates/**` and
  `kairo/extensions/**` (21 files total). Cross-checked reports 04/07/08 for known gaps but
  re-verified every claim against current source. No compilation or E2E was possible here
  (no Rust toolchain in this environment); this is a static omission audit.
- **Strictness rule:** a capability counts as *covered in Zed code* only if a user can trigger it
  from Zed and it performs the real operation end-to-end. A typed client method or declared action
  with no caller is **not** coverage.

---

## Headline verdict

- The 22 Kairo packages collapse into **6 user-facing legacy MUSTs** (build, encoding, Java, JSP,
  project, Tomcat) plus the two foundational libraries (protocol/runtime) and search; the rest are
  native substitutes, platform shims or explicitly deferred.
- Of the **18 MUST-preserve invariants**: **3 PRESERVED, 15 PARTIAL, 0 wholly MISSING**. However
  three critical *sub-capabilities* are wholly absent: the WS event stream (invariant 1's reactive
  half / matrix "event sequence/gap"), byte-faithful *normal* save (invariant 4), and zh-CN i18n
  (invariant 18).
- **15 concrete capabilities from `03 §3` are true omissions**: no Zed equivalent (native or Kairo)
  **and** no `FEATURE-MATRIX` row. They are listed in §3 with source anchors.
- The single highest data-loss gap is now narrower than in report 07: `SaveWithEncoding` **does**
  write the edited buffer (`save_buffer_with_encoding`, wired in the current working tree), but
  ordinary `Ctrl+S` still writes Zed's UTF-8 buffer — there is no will-save interceptor.
- The highest *reachability* gap is that GBK search, build cancel, JDT-launch-descriptor,
  hot-swap redefine, debug gate, encoding detection and agent event subscription all exist as
  real code but have **no caller from the Zed UI**.

---

## 1. All 22 Kairo packages → matrix / Zed code / legacy priority

Legend — **Matrix:** Y = explicit row, P = only implied/cross-cutting, N = absent.
**Zed:** Y = real end-to-end, P = partial/backend-only/dead method, N = none, NATIVE = replaced by
a Zed built-in. **Priority** is for the *legacy JSP-maintainer persona*, not dev-script priority.

| # | Package (`packages/…`) | Matrix | Zed code | Priority | Reason / evidence |
|---|---|---|---|---|---|
| 1 | `build-extension` | Y §4 (:68–76) | **P** | **MUST** | `Compile`/`CleanBuild`/`BuildAndDeploy` + `service.rs:563-674` polling give a real `javac` build, but the build view (history rows, severity diagnostics, stdout/stderr lines, `kairo.build.maxBuildHistory`), Maven view and Cancel are absent; `client.cancel_build` (:136) has no caller. A legacy project cannot be built otherwise. |
| 2 | `config-schema` | P §1 (:27) | **P** | SHOULD | No direct UI. Agent owns `.kairo/project.yaml`; Zed builds an import payload with `serde_json` (`service.rs:354-367`) but performs no Ajv/schema validation client-side. Correctness is delegated to the agent; omission has no standalone UI impact. |
| 3 | `decompress-compat` | N | N/A | **NOT-APPLICABLE** | Theia 1.73 / npm `.vsix` shim. Pure packaging; §6.2 says do not port. |
| 4 | `drivelist-stub` | N | N/A | **NOT-APPLICABLE** | npm drive-enumeration shim for Theia path pickers; Zed uses its native file dialogs. |
| 5 | `encoding-extension` | Y §3 (:54–62) | **P** | **MUST** | Highest data-loss risk. `/encoding/detect` + `/encoding/recode` are reached by `SaveWithEncoding` (`kairo_ui.rs:187-210`, `service.rs:1066-1118`), and the current tree writes the **edited buffer**. Missing: any will-save hook (normal `Ctrl+S` still writes UTF-8), 8-encoding dropdown, per-file/folder override precedence, EOL preservation (`eol: None` everywhere), tab suffix, lossy preview. |
| 6 | `git-extension` | Y §9 (:131) | **NATIVE** | SHOULD | Zed native Git covers status/diff/commit/history; Kairo extras (stash/cherry-pick templates) are secondary to the legacy JSP workflow. Not a legacy-critical omission. |
| 7 | `highlighting-extension` | P §2 (:41) | **P** | **MUST** | Basic JSP/Java highlighting exists (`languages/{jsp,java}/*`), but the deterministic `jsp-region-scanner`, Worker incremental tokenization, diagnostics and specific EL/JSTL captures do not. Without *any* highlighting the persona is broken, so the capability is MUST; the advanced scanner is SHOULD. |
| 8 | `i18n` | Y §9 (:139, P2) | **N** | OPTIONAL | Panel strings are English-only. Bilingual team is a nice-to-have, not a legacy workflow; Zed's own i18n can map strings later. |
| 9 | `java-extension` | Y §2 (:38–48) | **P** | **MUST** | `extension.toml:17-23` registers `jdtls` for Java/JSP and `activate_project` calls `/java/prepare`, but `launch_descriptor`, `generate_jdt_project`, `jdtls_status` (`client.rs:243-252`) have zero callers; live templates (150+), hierarchy, refactoring, CodeLens, index-progress and HotSwap-history widgets are absent. JDT LS is an explicit non-negotiable (non-goal: "we will not write our own Java parser"). |
| 10 | `jsp-extension` | Y §2 (:41–47) | **P** | **MUST** | Tree-sitter JSP grammar + Java injection + JDT LS registration give syntax highlighting only. No TLD/EL completion/hover/nav, no virtual-Java scriptlet docs, no JSP↔Servlet/web.xml navigation, no DTD validation, no outline. Treating JSP as plain text breaks the persona (invariant 11). |
| 11 | `plugin-extension` | Y §9 (:134, P3/N/A) | N/A | **NOT-APPLICABLE** | §9 says prefer Zed's extension system; do not port. |
| 12 | `project-extension` | Y §1 (:24–28) | **P** | **MUST** | `bootstrap()` (`service.rs:270-390`) now really calls open→scan→import-toolchain→import-project→activate, and `ScanProject` calls it — the report-07 G1 blocker is fixed. Missing: the 4-step *reviewable* import wizard (detected name/sourceDirs/encoding/JDK/server + confidence + warnings), structure dialog, recent-project MRU. |
| 13 | `protocol` | P §10 + "Reuse agent" | **Y** | **MUST** | No direct UI, but it is the sole integration seam: `domain.rs` has envelope + `requestId` + `ErrorCode` + safe DTOs and `client.rs` has ~30 typed endpoints. Silent omission would break every legacy operation; treat as foundational MUST. |
| 14 | `remote-extension` | Y §9 (:135, P3/N/A) | **N** | OPTIONAL | Deferred in Kairo's own v1 (ADR-0014); Zed remote/collaboration out of scope. |
| 15 | `runtime-extension` | P (all "Reuse agent" rows) | **Y/P** | **MUST** | `KairoClient`, `spawn_agent` (loopback-only, secret, ephemeral port), settings and fail-closed error mapping are real. Defects: `subscribe_events()` returns `stream::empty()` (`:273-279`), `auto_start_agent` is parsed but unused, and one agent is a process-wide singleton rather than per-workspace. |
| 16 | `search-extension` | Y §8 (:122–125) | **N/P** | **MUST** | `KairoService::search` (`service.rs:1201-1229`) sends `projectEncoding` to `/search` but has **no action, button or query input** — dead code. Zed native search is UTF-8 only, so GBK projects are unsearchable. Search is in the v1 mission statement. Search-Everywhere/find-* are covered by Zed native palettes (not omissions). |
| 17 | `sql-extension` | Y §9 (:133, P2) | **N** | OPTIONAL | No Zed SQL panel (agent endpoints are real per analysis 02). Operational convenience only; needs a real Oracle 11g to verify. |
| 18 | `svn-extension` | Y §9 (:132, P3) | **N** | OPTIONAL | No SVN in Zed; not all legacy users use SVN; explicitly P3/Planned. |
| 19 | `test-extension` | **N** (only "Run Test" implied) | **N** | SHOULD | `kairo-test-results` discover→run→results tree is absent from both matrix and code (see §3 #4). Useful but secondary. |
| 20 | `theia-product` | P §9 (:137–140) + §10 | **P** | **MUST** | One `KairoPanel` replaces ~35 widget factories. Status/JDK/encoding/Tomcat facts, 4-state views, bounded logs and the debug surface are required (invariant 18, §6.3 #4); the Theia shell, menus, toolbar and themes are platform-specific and redesigned. |
| 21 | `tomcat-extension` | Y §6 (:95–102) + §5 | **P** | **MUST** | Start/Debug/Stop/Restart/Reload/OpenApp actions, honest `restart_required`, paged 200-line log polling are real. Missing: server tree with state badges, deployments view with pills/triggers, log level filters/live-paused/load-more. Tomcat 6 lifecycle is a v1 ship criterion. |
| 22 | `ui-kit` | Y §9 (:140, P3) | NATIVE | OPTIONAL | Zed themes substitute; §6.2 explicitly says re-skin with Zed primitives. No data-integrity impact. |

**Package coverage summary:** MUST = 1, 5, 7, 9, 10, 12, 13, 15, 16, 20, 21 (11); SHOULD = 2, 6, 19 (3);
OPTIONAL = 8, 14, 17, 18, 22 (5); NOT-APPLICABLE = 3, 4, 11 (3).

---

## 2. The 18 "MUST preserve" invariants (01 §6.1) → status with evidence

| # | Invariant | Status | Evidence in current Zed code |
|---|---|---|---|
| 1 | One authority for OS effects (UI never spawns shells) | **PARTIAL** | `service.rs` routes builds/deploys/servers/encoding/search through `KairoClient` (ADR-0002 honored). **Exceptions:** `service.rs:959-971` directly reads/writes `WebRoot/WEB-INF/web.xml` via `smol::fs` before reload; `service.rs:1089-1108` stages/records temp files directly; `agent.rs:98-127` spawns the JDI bridge directly. All are controlled but bypass the single chokepoint. |
| 2 | Typed wire contract: envelope, correlation IDs, closed error enum, mirrored DTOs | **PARTIAL** | `domain.rs:29-65` (envelope, `ApiError`, `ResponseEnvelope`), `client.rs:329-426` (decode typed errors) — real. But `ErrorCode` is a `serde(transparent)` `String` (not a closed enum), DTOs are hand-written (no schema codegen), `correlation_id` is always `None`, and no DTO exists for the event stream. |
| 3 | Three-toolchain separation + import by fingerprint, fail-closed on JDK 6 | **PARTIAL** | `settings.rs:11-16` has all three homes; panel shows slots; `bootstrap` imports the compiler JDK; build polling emits "a real JDK 6 is required … importing a modern JDK is not a substitute" (`service.rs:654-661`). But `language_server_java_home` is display-only, there is no switch-JDK action/picker, no per-slot `toolchain_incompatible` badge, and fingerprint is only displayed, not verified client-side. |
| 4 | Byte-faithful encoding model (provenance, per-file EOL, properties, save preserves bytes, atomic recode + preview + lossy warning) | **PARTIAL (highest risk)** | `SaveWithEncoding` now stages the **edited buffer** through `/encoding/recode` (`kairo_ui.rs:202-207`, `service.rs:1066-1118`); agent does strict/atomic recode. **Missing:** no will-save interceptor, so ordinary `Ctrl+S` still writes UTF-8; no provenance stored in Zed's buffer; `eol: None` everywhere; no lossy preview UI; no 8-encoding dropdown or override precedence. |
| 5 | Legacy project detection & import (`/scan` → read-only review → `.kairo/project.yaml`) | **PARTIAL** | `bootstrap()` performs open→scan→import→activate end-to-end (`service.rs:270-390`), fixing report 07's G1. But it auto-imports the **first** detected layout with no user review of `confidence`/`warnings`/`encoding_by_extension`, and no schema validation in the client. |
| 6 | Real build: Ant-aware + `javac`, incremental, source/target, CJK diagnostics, argfiles | **PARTIAL** | Real `/builds` + poll-to-terminal (`service.rs:563-674`); source/target/encoding pushed via `PUT /projects/{id}`; `Diagnostic` DTO carries code/severity. Ant provider, per-file incremental (`jvm_compile` dead) and a diagnostics store/view are client-side gaps. Backend verified in `VERIFICATION.md`. |
| 7 | Safe deploy (preflight, containment, symlink checks, ownership token, atomic replace, mirror mode) | **PARTIAL** | `POST /deployments` via `service.rs:676-717`; backend enforces safety. But `scope` is hardcoded `"incremental"` (`:696`) — mirror mode and Publish are not surfaced — and `service.rs:959-971` writes a file directly before reload. |
| 8 | Server lifecycle (opaque IDs, generation, `ProcessIdentity` verification, state machine, agent-exit stop) | **PRESERVED (by reuse)** | `ServerResponse` carries `id`, `state`, `generation`, `ports`, `runtime_instance_id` (`domain.rs:295-313`); panel renders generation/state/http port; agent owns identity checks and state machine. |
| 9 | Hot-reload decision logic (static sync vs compile-only vs context reload; honest status) | **PARTIAL** | `HotDeploy` passes the active `.java` path (`kairo_ui.rs:62-84`) → `/jvm/compile` → `compileOnly → restart_required` (`service.rs:722-749`); context reload via `ReloadContext`; mode shown in panel. Missing: save-triggered sync, `jvm_redefine` when a debugger is attached (dead method), and positive `staticSync` confirmation. |
| 10 | Java intelligence via pinned + SHA-verified JDT LS with crash recovery | **PARTIAL** | `extension.toml:17-23` + `kairo_java.rs:20-61` register JDT LS; `/java/prepare` is called in `activate_project`. `launch_descriptor`, `generate_jdt_project`, `jdtls_status` have zero callers; `KAIRO_JDTLS_LAUNCHER` is an agent env var a WASM extension cannot read; no pin/verify/crash status in Zed. |
| 11 | JSP/Servlet first-class (scriptlet/TLD/EL completion, JSP↔Servlet/web.xml nav) | **PARTIAL** | Tree-sitter JSP + `injections.scm` + JDT LS registered for `jsp` give highlighting only. No TLD/EL completion, virtual Java docs, sourcemap, or web.xml navigation. Injection query over-broad: `((jsp) @injection.content …)` injects Java into the whole document, not just `<% %>` bodies. |
| 12 | Security model (loopback, secret, containment, per-process roots, auth/CSRF/throttle) | **PARTIAL** | `agent.rs:134-139` refuses non-loopback bind; `client.rs` sends `X-Kairo-Secret`; `spawn_agent` passes `-secret` (post-`c5d5a13`). But `GlobalKairoService` is one app-wide agent, not one per workspace/session (breaks the documented per-process-root threat model), and there is no auth/CSRF/throttle layer in Zed (loopback+secret is the substitute). |
| 13 | Secrets never leak through API DTOs | **PRESERVED** | `domain.rs:1-5, 295-313` deliberately omits `catalinaBase`, marker token, env, JVM options and local paths; no secret fields in safe DTOs. |
| 14 | Offline-by-design supply chain (SHA-256, pinning, no silent downloads) | **PARTIAL** | Zed performs no downloads; grammars pinned by commit in `extension.toml:9-15`; agent path configurable. But the JDI bridge jar is **not bundled** under `kairo/**` and falls back to a bare relative filename (`service.rs:830-833`), and JDT LS relies on PATH without an offline override in Zed. |
| 15 | Fail-closed honesty (gated tests, blockers documented, no false green, grey-out) | **PRESERVED** | `unavailable()` writes real errors into `last_error` (`service.rs:199-219`); build polling surfaces closed codes with JDK-6 guidance; debug gate fails closed (`service.rs:795-811`); panel renders `ERROR:` line. Caveat: not every gate is proactively disabled (see #16), and several `log::warn` paths still swallow non-user-facing failures. |
| 16 | Gate philosophy for debug | **PARTIAL** | `check_debug_gate` and `start_server(true)` call `/debug/adapter/status` and fail with a message. No action is proactively greyed out before the gate is checked, and `check_debug_gate` itself has no UI caller. |
| 17 | Performance & resource budget (paged logs, no infinite scroll, no AI, virtualization) | **PARTIAL** | 200-line cap and bounded 2 s polling (`service.rs:989-1036`, `kairo_panel.rs:17,178-185`); no AI surface. Missing: capped JDT heap in Zed, cold-start/RSS measurement, virtualization of the four text-blob columns, and cancellation/throttling polish (`set_position` is a no-op; panel rebuilds wholesale). |
| 18 | i18n (en/zh-CN) and accessible status surfaces | **PARTIAL** | No zh-CN catalog — all panel strings English. Status facts (JDK/source/encoding/EOL/Tomcat) are present as **text** in the panel (`kairo_panel.rs:187-207`) and are not color-only, but there is no Zed status-bar item and no at-a-glance strip (report 08 §1). |

**Totals (row-by-row): PRESERVED = #8, #13, #15 (3); PARTIAL = #1, #2, #3, #4, #5, #6, #7, #9, #10,
#11, #12, #14, #16, #17, #18 (15); wholly MISSING = 0.** The three critical *sub-capability* absences
are the event stream (reactive half of #1/#2), normal-save byte fidelity (#4) and zh-CN i18n (#18).

---

## 3. True omissions — concrete `03 §3` items with no Zed equivalent AND no FEATURE-MATRIX row

Excluded on purpose (not true omissions): local history, bookmarks, TODO, paste history and perf
dashboard are all named in matrix row `:137`; the compliance panel is row `:136`; SQL/Remote/SVN/
plugins/keymaps/themes are rows `:133–140`; Search Everywhere / find file-class-symbol are covered by
Zed native palettes (`:124–125`); granular DAP view commands are covered by Zed's native debug panel
(`:109–111`).

| # | Capability (source anchor in `03`) | Why it is a true omission | Impact |
|---|---|---|---|
| 1 | `kairo-test-results` discover→run→filter results tree, marker adapter, XML parsing (`§3.12`, `§2` row 19) | Matrix has only "Run Test" (P1, execution); no discovery/results-tree row; no Zed code | Medium (SHOULD) |
| 2 | HotSwap history + original-bytecode rollback (`kairo.java.hotswap.showHistory`, `§3.3`, `§2` row 9) | No matrix row; no Zed action/widget | Low–Medium |
| 3 | Experimental JSP breakpoints (`kairo.jsp.toggleBreakpoint`, Jasper CodeLens mapping, `kairo.jsp.debugBreakpoints` pref, `§3.4`, `§5`) | No matrix row (`GAPS-BACKLOG` C3 notes it); Zed has none | Low (experimental in Kairo too) |
| 4 | JDI bridge **host-JDK provisioning** / JDK 17+ bridge runtime (`§3.13`, bridge jar; backend `/debug/jdk/download` in analysis 02) | Matrix has the Java-6 gate but not the host JDK the bridge needs — explains why debug cannot work out of the box | **High for debug** |
| 5 | Remote-debug configuration widget / SSH debug tunnel (`§2` row 9 "remote-debug config", analysis 02 `§4.15`) | No matrix row; Zed DAP attach has no tunnel configuration | Low (P3) |
| 6 | Welcome view + dismissible agent-offline banner (`§3.2`) | No matrix row; no Zed welcome/offline surface | Low (first-run confusion) |
| 7 | Shortcuts/Keymap widget + shortcut cheat-sheet (`Ctrl+Shift+K`, `§3.18`) | Matrix row `:138` is "IDEA keymaps" (bindings), not the cheat-sheet UI | Low |
| 8 | Focus-management commands (`kairo.focus.editor/sidebar/bottomPanel/statusBar/terminal`, `nextPanel/prevPanel`, `hideActivePanel`; `§3.18`) | No matrix row; none in `actions.rs` | Low |
| 9 | Large-file commands (`kairo.largeFile.toggleFullFeatures`, `.togglePreview`; `§3.18`) | No matrix row; no Zed equivalent action | Low (4 GB machine relevance) |
| 10 | Editor toggle commands (`kairo.organizeImports`, `kairo.toggleFormatOnSave`, `kairo.toggleOrganizeImportsOnSave`; `§3.18`) | No matrix row; Zed has generic format-on-save but no Kairo toggle actions | Low |
| 11 | Secondary-window support (`kairo.editor.moveIntoNewWindow`, `§3.18`) | No matrix row; no Zed action | Low |
| 12 | Notification Center (`§3.18`) | No matrix row; panel has an inline error line only | Low–Medium (aggregated errors) |
| 13 | Localized Problems view replacing stock Theia Problems (`§3.18`) | No matrix row; Zed native diagnostics panel is a partial substitute | Low |
| 14 | Native status-bar grouping / tool bar with real chord hints / IDEA menu structure (`§3.1`, `§3.15`) | Matrix has keymaps/themes but no status-bar composition, toolbar or menu-structure row (report 04 §2 agrees) | **Medium–High** (invariant 18 verification surface) |
| 15 | Tomcat Server tree view (runtimes/deployments/ports, state badges, last-reload) + Deployments view (`§3.7`) | Matrix covers lifecycle actions and logs (`:96`, `:99`) but not the two structured views; the single 280 px panel only renders a flat string | Medium (partially absorbed by panel) |

**True-omission count: 15.** (Items 13–15 have partial native/panel substitutes, noted in the table;
they are still absent as concrete named features.) Adjacent backend-only omissions from analysis 02 —
agent self-restart/port handoff, operation-registry idempotency, unobserved-launch guard/`LaunchJob`
orchestration — are not from `03 §3` and are not counted here.

---

## 4. Legacy-critical omissions: minimal Zed implementation or explicit deferral

These are the items named in the task. Some are in the matrix (so not "true omissions" by §3's strict
definition) but are still absent or partial in Zed code.

### 4.1 GBK switching — **PARTIAL, MUST fix**
- Current: `SaveWithEncoding` action and panel button call `save_buffer_with_encoding` (edited
  buffer) → `/encoding/detect` is only called by the *other*, now-unwired `save_active_file_with_encoding`;
  recode uses `eol: None`; no will-save hook. Normal `Ctrl+S` writes UTF-8 bytes.
- **Minimal implementation:** add a save-boundary guard. If upstream exposes a will-save hook, route
  non-UTF-8 buffers through `save_buffer_with_encoding`; otherwise (a) call `client.detect_encoding`
  when an editor for a non-UTF-8 file becomes active, cache `{path → EncodingDetection}` in
  `KairoService`, (b) make the panel's encoding row a click target that dispatches `SaveWithEncoding`,
  and (c) pass the detected EOL into `RecodeRequest.eol` instead of `None`. Fix the now-unused
  `save_active_file_with_encoding`/service `detect_encoding` dead paths (either wire or remove).
- Do **not** defer: this is the highest data-loss risk (R-004, N-025/N-027).

### 4.2 JDK switch — **MISSING action, MUST**
- Current: three slots are display-only except `compiler_java_home` (imported during `bootstrap`);
  no action to switch; no per-slot fail-closed badge.
- **Minimal implementation:** add `kairo::SwitchJdk` action + three panel slot buttons. Each opens a
  Zed file picker → `client.import_toolchain(path, label)` → update `KairoSettings` → `activate_project`
  → render a `toolchain_incompatible` badge on the offending slot. Wire `language_server_java_home`
  into the `KAIRO_JDTLS_LAUNCHER` handoff (#4.10).
- Deferral not recommended; without it a fresh user cannot select the real JDK 6 from Zed.

### 4.3 Project import — **PARTIAL (auto-only), MUST**
- Current: `bootstrap()` imports the first detected layout silently.
- **Minimal implementation:** add a review modal (Zed `Modal`/`Picker`) over
  `DetectedProjectLayout` showing `root_path`, `build_system`, `confidence`, `warnings`,
  `encoding_by_extension`, `layout.src/web_root/build_xml`, with editable source/target/encoding and
  a Confirm button calling `client.import_project` + `activate_project`. This is matrix row `:26`.
- Deferral reason if skipped: backend import already works; only the review UI is missing (data-safety
  of a wrong auto-detected layout is the risk).

### 4.4 Build view — **MISSING view, SHOULD (P1)**
- Current: status string + error line; `Diagnostic` parsed but only the first `code` used; cancel dead.
- **Minimal implementation:** extend `KairoPanel` with a build section: rows from `service.builds()`
  with state badges, diagnostic rows with severity + file:line, a stdout/stderr `<pre>` area, and a
  Cancel button calling `client.cancel_build(id)`. Publish diagnostics into Zed's diagnostics store
  so build errors navigate to the offending line (E2E-04).
- Deferred with reason if skipped: needs a `diagnostics` crate dependency and a larger panel; the
  compile itself is real.

### 4.5 Tomcat server view — **MISSING structured view, SHOULD**
- Current: flat `Servers\n{id}: {state} (gen, http)` string.
- **Minimal implementation:** structured list with explicit text-state tokens
  (`RUNNING`/`STARTING`/`STOPPED`/`CRASHED`), per-server Start/Debug/Stop/Restart/Reload/Open buttons,
  last-reload time and port; add a Deployments section via a new `client.list_deployments()` DTO
  wrapper (agent endpoint per analysis 02) with state pills and a Build & Deploy trigger.
- Deferred with reason if skipped: no data-integrity impact; the single panel can absorb the facts.

### 4.6 Log viewer — **PARTIAL, SHOULD**
- Current: 200-line capped tail poll, scroll box and monospace exist; no level prefix/filter,
  live/paused toggle, load-more, or "newest first" label.
- **Minimal implementation:** add `live: bool` field and a Pause button; prefix each line with its
  textual level (`LogLine.stream` / level from payload); add a bounded "Load earlier" calling
  `service.load_logs(server_id, tail)` (cap 200); label the view "newest first".
- Deferred with reason if skipped: readability only, no loss of data.

### 4.7 Search — **MISSING UI, MUST for GBK**
- Current: `KairoService::search` is dead code; no action/button/query input.
- **Minimal implementation:** add `kairo::SearchEncoded` action (remove the earlier fake stub pattern),
  a one-line query input in `KairoPanel`, and render `/search` results with a preview; pass
  `projectEncoding` from settings. UTF-8 workspaces keep using Zed native search.
- Do not defer: a GBK project cannot be searched from Zed at all today.

### 4.8 DAP debug tool window — **PARTIAL, MUST for debug**
- Current: gate check + real JDI bridge spawn + TCP attach to `server.ports.debug`; Zed native debug
  panel supplies variables/callstack/breakpoints/watch/console. No debug launch config is contributed
  (user must author one with the right port), and the bridge jar is not bundled.
- **Minimal implementation:** contribute a Java attach debug-task template in `extension.toml` /
  `debug_tasks.json`; persist the agent's `ports.debug` to a host-readable state file that
  `kairo_java.rs::get_dap_binary` reads (WASM cannot read the agent's env); document/bundle the JDI
  bridge jar path explicitly and grey out the Debug action until `/debug/adapter/status` + jar exist.
- Deferred with reason (partial): the bridge jar is an EOL, user-supplied binary (B-004), so true
  out-of-box debug cannot be promised; gate + attach are already real when prerequisites exist.

### 4.9 Status-bar facts — **PARTIAL, MUST (invariant 18 / §6.3 #4)**
- Current: facts live only inside `KairoPanel` (compiler JDK, source/target, encoding, Tomcat port);
  no Zed status-bar item and no at-a-glance strip; detected vs configured encoding shown twice.
- **Minimal implementation:** register a Zed status-bar item showing
  `JDK 1.6.0_45 | Source 1.6 | GBK/CRLF | Tomcat 127.0.0.1:8080/<ctx> | agent connected`, each segment
  a click target (encoding → SaveWithEncoding, Tomcat → panel). If the upstream status-bar API is not
  available without a large patch, add the explicit "three facts" strip to the top of `KairoPanel`
  with text tokens (`NOT SET`, `STOPPED`, `FAIL-CLOSED`) and collapse the duplicate encoding rows.
- Deferred with reason (if no status-bar API): keep the panel strip; do not drop the facts.

### 4.10 JSP highlighting / EL / TLD — **PARTIAL, MUST for highlighting; P2 for completion**
- Current: tree-sitter JSP grammar + highlights + Java injection + JDT LS registered for JSP.
  No EL/TLD completion/hover/navigation, no virtual-Java docs, no web.xml navigation.
- **Minimal implementation (cheap, do it):** narrow `injections.scm` to scriptlet/expression nodes
  instead of `((jsp) @injection.content …)`, and add EL/TLD-specific capture queries in
  `highlights.scm`. **Deferred with reason (completion/nav):** a Kairo LSP proxy implementing JSP
  virtual documents + sourcemap (analysis 03 §3.4) is a P2 effort and is already marked Planned in
  matrix rows `:42–46`; do not silently claim it.

### 4.11 Hot deploy — **PARTIAL, MUST**
- Current: `HotDeploy` passes the active `.java` path → `/jvm/compile` honest `restart_required`;
  non-Java → static deploy; context reload via separate action. No automatic save trigger;
  `jvm_redefine` dead; no positive `staticSync` confirmation.
- **Minimal implementation:** add an explicit "Sync this file" action that takes the active editor
  path (mirroring `HotDeploy`), calls `/jvm/compile` for `.java` and `/deployments` for static files,
  and calls `/jvm/redefine` when `service.bridge.is_some()`. Report `staticSync` for static files.
  A true will-save trigger is deferred with reason (upstream save-hook surface), same as §4.1.

### 4.12 Local history — **DEFERRED (P3), reason recorded**
- In matrix `:137`. Zed has native file history/timeline affordances; complete snapshot
  show/restore/compare from `.kairo/` is not ported. No data-integrity impact (deploy is atomic,
  and the browser app explicitly excludes `.kairo/` snapshots from search).
- Minimal if ever needed: map Zed's timeline to the agent's history endpoints; do not block v1.

### 4.13 Bookmarks — **DEFERRED (P3), reason recorded**
- In matrix `:137`. No Zed equivalent action; pure editor convenience with no legacy correctness
  impact. Use a Zed community bookmarks extension if desired; do not build into `kairo_core`.

### 4.14 TODO / FIXME — **DEFERRED (P3), reason recorded**
- In matrix `:137`. Could be delivered cheaply by a tree-sitter TODO query over comments in the
  Kairo Java/JSP languages, but it is not legacy-critical. Defer until the language layer is stable.

### 4.15 Keymaps — **PARTIAL, SHOULD**
- In matrix `:138`. Current: six `ctrl-alt-shift-*` bindings (`actions.rs:50-58`), verified on Windows;
  two collide on Linux/macOS, `SaveWithEncoding` is unbound, no chord hints in the panel, and IDEA
  function-key chords (Shift+F10/Ctrl+F10/Shift+F9) are not offered.
- **Minimal implementation:** ship a Kairo-owned keymap file with the IDEA chords where they are free,
  keep the verified `ctrl-alt-shift-*` set as fallbacks, bind `SaveWithEncoding`, fix the false
  "no collisions" comment, and display chord hints in button tooltips.

---

## 5. Coverage arithmetic and caveats

- Strict user-triggerable coverage of the **11 legacy MUST packages**: fully working = protocol/runtime
  foundations + Java/JSP highlighting (3–4); partial = build, encoding, project, Tomcat, search,
  theia-product information architecture (7). No MUST package is entirely absent.
- **Dead-but-real code** that should be either wired or removed (avoid "capability exists" illusions):
  `client.cancel_build`, `client.jvm_redefine`, `client.launch_descriptor`,
  `client.generate_jdt_project`, `client.jdtls_status`, `client.subscribe_events`,
  `KairoService.search`, `KairoService.check_debug_gate`, `KairoService.detect_encoding`,
  `KairoService.save_active_file_with_encoding`, `AgentHandle::events_url`, `auto_start_agent`.
- This audit is static: no `cargo`/`rustc`, no compile proof, no E2E. Report 06 covers compile-shape;
  `VERIFICATION.md` covers the real-agent backend. Nothing here contradicts the backend evidence:
  the Go agent really compiles/deploys/serves JSP; the finding is about **what Zed can trigger** and
  **what was never ported at all**.
- The working tree had in-flight edits to the GBK-save wiring during this audit; if the parent
  session reverts them, §4.1 regresses to "explicit command recodes stale on-disk bytes" and invariant
  4 should be downgraded to MISSING.
