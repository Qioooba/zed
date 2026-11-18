# 10 — Endpoint omission audit: all 83 Kairo `/api/v1` endpoints vs. the Zed integration

- **Review date:** 2026-11-19
- **Reviewer:** omission auditor (read-only; this report is the only file written)
- **Branch:** `kairo/integration`, HEAD `203fdb9`
- **Snapshot caveat:** during this audit `kairo/crates/kairo_core/src/service.rs` and
  `kairo/crates/kairo_ui/src/kairo_ui.rs` carried **in-flight uncommitted edits** (line numbers
  shifted between reads). The verdicts below reflect the working-tree content as read:
  `bootstrap()` calls `/workspaces`, `/scan`, `/toolchains/import`, `/projects/import`;
  build polling, `PUT /projects/{id}`, `/java/prepare`, `/encoding/recode`,
  `/jvm/compile`, `/search`, the debug gate and bridge spawn are wired;
  `SaveWithEncoding` now stages the **edited buffer** through a temp file before recode.
  If those edits are later reverted, re-check the YES rows in §1.
- **Sources:** `docs/analysis/02` §3 (83-endpoint table) + §4 (15 feature areas);
  `docs/FEATURE-MATRIX.md`; `crates/kairo_core/src/{client,service,actions,domain,agent,settings}.rs`;
  `crates/kairo_ui/src/{kairo_ui,kairo_panel}.rs`; `extensions/kairo-java/**`;
  `docs/review/04` and `docs/review/07` (historical context only — their verdicts predate
  the current code and were **not** copied forward).
- **Strictness rule:** an endpoint is **YES** only when a reachable Zed action/UI path drives a
  non-placeholder client method. A typed client method with zero callers is **PARTIAL**, not YES.
  A stub that returns `stream::empty()` is **PARTIAL**. A capability with no client method at all
  is **NO**.

---

## 0. Headline result

| Metric | Count |
|---|---:|
| Endpoints audited | **83** |
| YES — reachable, real Zed caller | **23** |
| PARTIAL — method exists but dead/stub-only/unreachable | **12** |
| NO — no Zed client method at all | **48** |
| Endpoint rows whose capability is **absent from FEATURE-MATRIX** (`NO — MISSING` in col. 3) | **12 endpoints / 10 distinct capabilities** |
| Rows classified **MUST-IMPLEMENT and missing** | **10** |

**Top 3 omitted legacy-critical capabilities:** (1) JDT project model + launch descriptor →
Java/JSP intelligence is dead (#66, #10); (2) the event stream is an empty stub, so
`hotreload.status`/gap detection never reaches Zed (#60); (3) GBK search is implemented in
`KairoService` but has **no action and no UI entry**, so it is unreachable (#51).

---

## 1. Complete 83-endpoint coverage table

Column legend:
- **Called?** YES / NO / PARTIAL (see strictness rule above).
- **In matrix?** only filled for NO/PARTIAL rows. `YES` = the capability appears in
  `FEATURE-MATRIX.md`; `NO — MISSING` = it is silently omitted from the promised scope.
- **Class:** MUST / SHOULD / OPTIONAL / N/A for a **legacy-project user** (JDK 1.6, GBK,
  Ant/Tomcat 6, 2008-era webapp).
- **Minimal Zed-side fix:** `method + action + UI entry` for missing MUST/SHOULD rows.

### 1.1 Bootstrap, discovery, auth & workspace (1–9)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 1 `GET /health` | YES (`start_agent`, `bootstrap`) | — | MUST — agent liveness gate; implemented | — |
| 2 `GET /endpoints` | **PARTIAL** — `client.endpoints()` has zero callers | NO — MISSING | N/A — Zed spawns its own agent on port 0 and reads `agent-state.json`; dynamic discovery is redundant | — (do not implement; note the matrix omission) |
| 3 `POST /runtime/restart` | **NO** | NO — MISSING | OPTIONAL — Zed owns the child process and uses `ReconnectAgent`; an agent self-respawn would orphan the tracked `AgentHandle`/port handoff | — |
| 4 `GET /workspaces` | YES (`refresh`) | — | MUST — workspace catalog; implemented | — |
| 5 `POST /workspaces` | YES (`bootstrap`, `open_workspace`) | — | MUST — open + sandbox root; implemented | — |
| 6 `GET /workspaces/{id}` | **NO** | YES (workspace row :24) | OPTIONAL — Zed caches the full `WorkspaceRecord` from list/create | — |
| 7 `DELETE /workspaces/{id}` | **NO** | YES (:24) | OPTIONAL — workspace lifetime is bound to Zed's agent process; closing = dropping the agent | — |
| 8 `POST /workspaces/{id}/scan` | YES (`bootstrap`, `scan_workspace`) | — | MUST — legacy layout detection; implemented | — |
| 9 `POST /workspaces/{id}/projects/import` | **NO** | NO — MISSING | N/A — legacy alias of #25; Zed calls `/projects/import` directly | — |

### 1.2 JDT LS, run configurations & launch jobs (10–19)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 10 `GET /workspaces/{ws}/java/launch-descriptor` | **PARTIAL** — `client.launch_descriptor()` exists, zero callers | YES (:38) | **MUST** — the only way to obtain JDT LS command/args/envAllowlist; without it Java/JSP completion, navigation and refactor are impossible | `KairoService::attach_jdt_ls()` → call after `generate_jdt_project`, persist `{command,args,workingDir,envAllowlist}` to `<dataDir>/jdtls-launch.json`; action `kairo::AttachLanguageServer`; UI entry: KairoPanel "Attach JDT LS" button + descriptor status; `kairo_java.rs::language_server_command` reads the persisted descriptor (not `KAIRO_JDTLS_LAUNCHER`) |
| 11 `POST /workspaces/{ws}/java/prepare` | YES (`activate_project`) | — | MUST — offline JDT LS install; implemented | — |
| 12 `GET /workspaces/{ws}/run-configurations` | **NO** | YES (:32) | SHOULD — P1 matrix promise; needed to reuse a project's saved ports/context/vmOptions instead of hardcoding `/` | `client.list_run_configurations(ws)`; action `kairo::LoadRunConfigurations`; UI: panel "Run Configurations" list |
| 13 `PUT /workspaces/{ws}/run-configurations` | **NO** | YES (:32) | SHOULD — save edited run configs (replace-all) | `client.save_run_configurations(ws, doc)`; action `kairo::SaveRunConfigurations`; UI: config editor/Save button |
| 14 `POST /workspaces/{ws}/run-configurations` | **NO** | YES (:32) | SHOULD — create a new config | `client.create_run_configuration(...)`; action `kairo::NewRunConfiguration`; UI: New button in config list |
| 15 `GET /workspaces/{ws}/run-configurations/{cfg}` | **NO** | YES (:32) | OPTIONAL — single read covered by list (#12) | — |
| 16 `PUT /workspaces/{ws}/run-configurations/{cfg}` | **NO** | YES (:32) | SHOULD — edit one config | `client.update_run_configuration(ws, cfg, dto)`; reuse #13's UI |
| 17 `DELETE /workspaces/{ws}/run-configurations/{cfg}` | **NO** | YES (:32) | OPTIONAL — housekeeping, not part of the run workflow | — |
| 18 `POST /workspaces/{ws}/run-configurations/{cfg}/launch` | **NO** | YES (:32 — "before-launch tasks, ports, context path") | **MUST** — the one-click run/debug path with before-launch tasks, orchestration and the unobserved-launch guard that prevents orphan Tomcats; Zed currently hardcodes contextPath `/` and settings ports | `client.launch_run_configuration(ws, cfg, mode, async)`; action `kairo::LaunchRunConfiguration`; UI: panel config dropdown + Run / Debug buttons; on `async=1` poll #19 |
| 19 `GET /launch-jobs/{id}` | **NO** | NO — MISSING | SHOULD — progress state for #18's async launch; orphan protection is invisible without it | `client.launch_job(id)`; no new action; UI: panel launch-progress row with step/tasks/errorCode |

### 1.3 Projects & toolchains (20–28)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 20 `GET /projects` | YES (`refresh`) | — | MUST — project catalog; implemented | — |
| 21 `GET /projects/{id}` | **NO** | YES (:28 project selector) | OPTIONAL — Zed caches full `Project` DTOs from #20 | — |
| 22 `PUT /projects/{id}` | YES (`activate_project`) | — | MUST — pushes source/target/encoding settings; implemented | — |
| 23 `DELETE /projects/{id}` | **NO** | NO — MISSING | OPTIONAL — project unregistration is administrative, not part of the legacy workflow | — |
| 24 `POST /projects/detect` | **PARTIAL** — `client.detect_project()` exists, zero callers | YES (:25) | OPTIONAL — redundant with #8 `/scan`, which returns richer `DetectedProjectLayout` (source dirs, encoding map, warnings) | — |
| 25 `POST /projects/import` | YES (`bootstrap`) | — | MUST — create-only project import; implemented | — |
| 26 `GET /projects/recent` | **NO** | YES (:28) | OPTIONAL — Zed's own worktree MRU covers this; matrix says "Zed worktree + status item" | — |
| 27 `GET /toolchains` | YES (`refresh`) | — | MUST — three-slot catalog; implemented | — |
| 28 `POST /toolchains/import` | YES (`bootstrap`, **conditional** on `compiler_java_home`) | — | MUST — SHA-256 fingerprint import is how a real JDK 6 enters the catalog. Caveat: only the **compiler** slot can be imported from Zed; LS JRE and Tomcat JDK are display-only | — (extend #28 with slot selection for the other two slots) |

### 1.4 Build & Ant (29–36)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 29 `GET /builds` | **NO** | YES (:74 "Build view: history") | SHOULD — build history tab promised; Zed keeps only in-memory `builds` | `client.list_builds(ws)`; action `kairo::ShowBuildHistory`; UI: KairoPanel Build tab |
| 30 `POST /builds` | YES (`start_build`) | — | MUST — real javac build; implemented | — |
| 31 `GET /builds/{id}` | YES (`start_build_polling`, 500 ms × 240) | — | MUST — terminal state + diagnostics; implemented | — |
| 32 `DELETE /builds/{id}` | **PARTIAL** — `client.cancel_build()` exists, zero callers | YES (:74 "cancel via DELETE /builds/{id}") | SHOULD — cancel a long/rung javac or Ant process tree | `client.cancel_build` (method ready); action `kairo::CancelBuild`; UI: Cancel button on the build row |
| 33 `POST /build/custom` | **NO** | YES (:75 "Custom build commands (before-launch tasks)") | SHOULD — before-launch tasks in run configs need custom command execution | `client.custom_build(config)`; action `kairo::RunCustomBuild`; UI: Zed task provider entry |
| 34 `GET /build/custom/{id}` | **NO** | YES (:75) | SHOULD — status for #33 | `client.custom_build_status(id)` polled; no new action; UI: task output line |
| 35 `POST /build/custom/{id}/cancel` | **NO** | YES (:75) | OPTIONAL — cancel a custom command | — |
| 36 `GET/POST /ant/classpath/analyze` | **NO** | YES (:69 "Ant provider (`build.xml`)") | **MUST** — legacy Ant layouts need real `build.xml` classpath/sourceRoots/outputDir; without it both compile classpath and the generated JDT `.classpath` are naive dir guesses | `client.analyze_ant_classpath(projectRoot, buildFile)`; action `kairo::AnalyzeAntClasspath`; UI: panel "Analyze Ant Classpath" button that merges `classpath/sourceRoots/outputDir` into project config and feeds `libraries` into #66 |

### 1.5 Deploy & hot deploy (37–39, 81–83)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 37 `GET /deployments` | **NO** | NO — MISSING | OPTIONAL — deployment history is not a promised matrix capability | — |
| 38 `POST /deployments` | YES (`deploy`) | — | MUST — safe deploy; implemented (caveat: `scope` hardcoded `"incremental"`, mirror mode unreachable) | — |
| 39 `GET /deployments/{id}` | **NO** | NO — MISSING | OPTIONAL — single deployment result; Zed keeps `DeployResult` from #38 | — |
| 81 `POST /jvm/compile-incremental` | **NO** | YES (:73 names both compile endpoints) | SHOULD — batched multi-file incremental compile for large legacy trees; #82 covers the single-file path | `client.jvm_compile_incremental(projectId, files)`; reuse `kairo::HotDeploy`; UI: save hook collects changed files |
| 82 `POST /jvm/compile` | YES (`hot_deploy` .java branch) | — | MUST — incremental compile; implemented (manual action only; no save hook) | — |
| 83 `POST /jvm/redefine` | **PARTIAL** — `client.jvm_redefine()` exists, zero callers | YES (:87 "invoked when debugger attached") | **MUST** — the only path that applies a Java class change to a live Tomcat without losing session state; otherwise every Java save is `restart_required` | `client.jvm_redefine` wired in `hot_deploy` when the debug bridge is attached (pass `sourcePath`, `classPath`, `expectedHash`, `projectId`/`serverId` target binding); action `kairo::HotSwapClass`; UI: panel "Hot Swap" button enabled only with an attached debug session |

### 1.6 Tomcat lifecycle, logs & port diagnostics (40–50, 67)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 40 `GET /servers` | YES (`refresh`) | — | MUST — server catalog; implemented | — |
| 41 `POST /servers` | YES (`start_server`) | — | MUST — real Tomcat 6 start; implemented | — |
| 42 `GET /servers/{id}` | **PARTIAL** — `client.server()` exists, zero callers | YES (:96 `/servers/{id}/*`) | OPTIONAL — Zed caches `ServerResponse`; single fetch would only refresh generation | — |
| 43 `DELETE /servers/{id}` | YES (`stop_server`) | — | MUST — graceful/force stop; implemented | — |
| 44 `POST /servers/{id}/debug` | **PARTIAL** — `client.debug_server()` exists, zero callers | YES (:96, :102) | SHOULD — restart an **already-running** server with JDWP on a fresh port; Zed instead uses `POST /servers{debug:true}` only | `client.debug_server` (method ready); action `kairo::DebugRunningServer`; UI: panel Debug button on each server row; re-spawn bridge with returned `ports.debug` |
| 45 `POST /servers/{id}/restart` | YES (`restart_server`) | — | MUST — new-generation restart; implemented | — |
| 46 `GET /servers/{id}/logs` | YES (`server_logs`, 200-line poll every 2 s) | — | MUST — bounded logs; implemented (no paging/load-more) | — |
| 47 `POST /servers/{id}/recover` | **NO** | YES (:96 "recover") | SHOULD — relaunch a crashed Tomcat; legacy servers die on JDK 1.6/classloader issues | `client.recover_server(id)`; action `kairo::RecoverServer`; UI: panel Recover button on crashed server row |
| 48 `POST /servers/{id}/reload` | YES (`reload_server`) | — | MUST (P1) — verified context reload; implemented (Zed also mirrors `WebRoot/WEB-INF/web.xml` before calling) | — |
| 49 `GET /servers/recoverable` | **NO** | YES (:96) | OPTIONAL — filtered crashed-server view; `GET /servers` already reports state | — |
| 50 `GET /operations/{requestId}` | **NO** | NO — MISSING | OPTIONAL — idempotency/operation-status is an internal protocol concern; Zed sends `requestId` envelopes but never polls status | — |
| 67 `GET /diagnostics/port` | **NO** | YES (:98 "surface suggestion in panel") | SHOULD — legacy machines have port conflicts; surfacing the agent's suggestion avoids opaque start failures | `client.port_diagnostics(port)`; action `kairo::CheckPort`; UI: panel port field + suggestion line |

### 1.7 Search & encoding (51–56)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 51 `POST /search` | **PARTIAL** — `client.search()` is called **only** by `KairoService::search()`, which has **no action and no UI entry** (dead) | YES (:62, :122) | **MUST** — Zed native search is UTF-8 only, so GBK projects are unsearchable; P0 matrix promise | method ready (`service.search`); add action `kairo::SearchEncoded`; UI: KairoPanel query input + results list (bind a key). Also expose regex/case/wholeWord/contextLines/previewReplace options |
| 52 `POST /search/files` | **NO** | YES (:124–125 search-everywhere/find-file rows) | OPTIONAL — Zed native file finder covers UTF-8 paths; GBK filename listing is rarely needed | — |
| 53 WS `/search/stream` | **NO** | YES (:123 "Streaming search — Reuse agent") | OPTIONAL — batch `/search` with `maxResults` covers typical legacy trees; needed only for very large workspaces | — |
| 54 `POST /encoding/detect` | **PARTIAL** — `client.detect_encoding()` is called by `save_active_file_with_encoding()` and `service.detect_encoding()` (the latter itself has no action) | YES (:54 "at file-open boundary") | MUST — detection works only inside explicit Save-with-Encoding; the promised file-open hook is absent | method ready; action `kairo::DetectFileEncoding`; UI: detect on buffer open + status-bar encoding segment; call `service.detect_encoding` from the editor事件 |
| 55 `POST /encoding/recode` | YES (`save_buffer_with_encoding` stages edited text via temp file, then strict recode) | — | MUST — byte-faithful GBK save; implemented (but only via explicit command, no `will-save` interceptor on `Ctrl+S`) | — (add a `will-save` hook so ordinary saves cannot write UTF-8 bytes into GBK files) |
| 56 `POST /encoding/validate` | **PARTIAL** — `client.validate_encoding()` exists, zero callers | YES (:57 "Reuse agent **before every recode**") | **MUST** — pre-flight representability check prevents silent loss of characters not representable in GBK; explicitly promised P0 | wire `client.validate_encoding(text, target)` into `save_buffer_with_encoding` **before** recode and surface `valid:false/error` via `set_error`; no new action; UI entry = existing Save with Encoding button + panel ERROR line |

### 1.8 Security, events & audit (57–60, 59)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 57 `POST /auth/login` | **NO** | YES (:148 auth invariant — no login UI) | N/A — Zed spawns a loopback agent with a shared secret; no multi-user login | — |
| 58 `POST /auth/logout` | **NO** | YES (:148) | N/A — same as #57 | — |
| 59 `GET /audit` | **NO** | YES (:136 "Reuse agent audit only") | OPTIONAL — compliance panel is P3/out of scope; the agent still writes the NDJSON audit log | — |
| 60 WS `/events` | **PARTIAL** — `client.subscribe_events()` returns `futures::stream::empty()` (stub) and has zero callers; no `kairo_core::events` module | YES (:89, invariant :151) | **MUST** — `hotreload.status`, build/server events, `snapshot.required`, `event.gap` are P0 matrix promises; currently none reach Zed | implement a real WS client in `subscribe_events` (`kairo-secret-v1` subprotocol + `agent.events_url()`); add `kairo_core/src/events.rs` with sequence/gap detection; action `kairo::ToggleEvents`; UI: KairoPanel Events section + gap/restart-required banner |

### 1.9 JDT LS distribution & project model (61–66)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 61 `GET /jdtls` | **PARTIAL** — `client.jdtls_status()` exists, zero callers | YES (:40) | SHOULD — LS state/version/lastError is needed to diagnose why Java intelligence is down | `client.jdtls_status`; action `kairo::CheckLanguageServer`; UI: panel LS status line |
| 62 `POST /jdtls` | YES (`prepare_jdtls` in `activate_project`) | — | MUST — offline install; implemented | — |
| 63 `DELETE /jdtls` | **NO** | N/A — endpoint returns 400 by design | N/A — explicitly unsupported; Theia/Zed owns process lifecycle | — |
| 64 `GET /jdtls/distribution` | **NO** | YES (:40) | OPTIONAL — distribution diagnostics overlap #61 | — |
| 65 `GET /jdtls/project` | **NO** | YES (:39) | OPTIONAL — model status can be folded into #66's result | — |
| 66 `POST /jdtls/project` | **PARTIAL** — `client.generate_jdt_project()` exists, zero callers | YES (:39 "**call before LS attach**") | **MUST** — without `.project`/`.classpath`/`.settings`/`KairoJavaConfig.ini`, JDT LS cannot resolve legacy Ant source roots, classpath, source level or GBK encoding → no Java completion/navigation | wire `client.generate_jdt_project` in `activate_project`/`attach_jdt_ls` **before** #10 (pass `workspaceId`, `projectId`, `rootPath`, `buildFile`, `libraries` from #36); action `kairo::GenerateJdtProject`; UI: panel "Generate JDT Project" button + result status |

### 1.10 Maven, SQL, debug tunnel & host JDK (68–76)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 68 `POST /maven/detect` | **NO** | YES (:76 "Maven detect/deps/lifecycle") | OPTIONAL — experimental; target users are Ant/JDK 6; matrix P2 | — |
| 69 `GET /maven/dependencies` | **NO** | YES (:76) | OPTIONAL — same as #68 | — |
| 70 `POST /maven/run` | **NO** | YES (:76) | OPTIONAL — same as #68 | — |
| 71 `POST /sql/execute` | **NO** | YES (:133 "Oracle 11g SQL console", P2) | OPTIONAL — experimental; needs real Oracle to verify | — |
| 72 `POST /sql/test-connection` | **NO** | YES (:133) | OPTIONAL — same as #71 | — |
| 73 `GET /debug/adapter/status` | YES (`check_debug_gate`, `start_server(true)`) | — | MUST — Java 6 gate probe; implemented (caveat: no proactive grey-out of Debug action) | — |
| 74 `POST /debug/tunnel/connect` | **NO** | NO — MISSING | OPTIONAL — experimental `-tags remote` build + SSH access; remote agent out of scope for v1 | — |
| 75 `POST /debug/tunnel/disconnect` | **NO** | NO — MISSING | OPTIONAL — same as #74 | — |
| 76 `POST /debug/jdk/download` | **NO** | NO — MISSING | SHOULD — the JDI bridge needs a host JDK 17+/21; without provisioning, Debug is not available out of the box, and Zed currently **fabricates** a fake `debug_gate.host_jdk` from `language_server_java_home` | `client.download_debug_jdk()`; action `kairo::ProvisionDebugJdk`; UI: panel "Install debug JDK" button with progress from #60 events; use returned `jdkPath` for `spawn_debug_bridge` instead of the setting |

### 1.11 Java run / JUnit / managed sessions (77–80)

| # / path / method | Called? | In matrix? | Class (reason) | Minimal Zed-side fix |
|---|---|---|---|---|
| 77 `POST /java/run` | **NO** | YES (:114 "Run Main / Run Test / Debug Main / Debug Test") | **MUST** — one-click execution of a legacy main/test class is a core user workflow, and the matrix promises it | `client.run_java(request)` (`RunJavaRequest` DTO in `domain.rs`); actions `kairo::RunJava` / `kairo::DebugJava`; UI: panel Run/Debug buttons + JDT CodeLens on main/@Test methods |
| 78 `GET /java/runs/{runId}` | **NO** | YES (:115 "Managed JVM sessions") | SHOULD — state/stdout/stderr for #77 runs | `client.java_run(id)` polled; no new action; UI: panel session row |
| 79 `DELETE /java/runs/{runId}` | **NO** | YES (:115) | SHOULD — kill a runaway managed JVM | `client.kill_java_run(id)`; action `kairo::StopJavaRun`; UI: Stop button on session row |
| 80 `POST /java/detect` | **NO** | NO — MISSING | SHOULD — main/test method discovery powers Run/Test actions and the JUnit results tree (matrix :114 implies discovery; report 04 §2 flagged it as absent) | `client.detect_java(file)`; action `kairo::DetectJavaMethods`; UI: panel method list / LSP CodeLens provider |

---

## 2. Special-attention legacy-critical items — consolidated verdict

| Area | Endpoints | Verdict |
|---|---|---|
| **GBK recode / detect / validate** | 54, 55, 56 | detect + recode reachable via explicit `SaveWithEncoding` (staged edited buffer — snapshot fix); **validate is a dead client method** and matrix explicitly promises it "before every recode". No `will-save` hook on `Ctrl+S` → ordinary saves still write UTF-8 bytes. **MUST: #56.**
| **Toolchain import / fingerprint** | 27, 28 | Listed and imported, but **only the compiler slot** is importable from Zed; LS JRE / Tomcat JDK slots are display-only. Fingerprint is shown as "fingerprinted/UNVERIFIED" in the panel. No matrix omission, but a three-slot UX gap. |
| **Ant classpath** | 36 | **No client method at all** while matrix promises the Ant provider. **MUST: #36.** |
| **JDT project model / launch descriptor** | 10, 61–66 | `prepare` is wired; `generate_jdt_project`, `launch_descriptor`, `jdtls_status` are all **dead client methods**. Java/JSP intelligence is unavailable. **MUST: #66 + #10.** |
| **`/jvm/compile` + `/jvm/redefine`** | 81, 82, 83 | compile is reachable via `HotDeploy` on a `.java` buffer; **redefine is a dead method** and `-incremental` has no caller. **MUST: #83; SHOULD: #81.** |
| **`/servers/reload`** | 48 | Wired to `ReloadContext` action, includes legacy `WebRoot/WEB-INF/web.xml` mirroring. OK. |
| **Search** | 51–53 | `client.search` is called only by an **unwired** `service.search`; no action/UI. GBK projects unsearchable. **MUST: #51.** Streaming/files optional. |
| **Events** | 60 | `subscribe_events()` is `stream::empty()`; no `events` module. Hot-reload status/gap detection absent. **MUST: #60.** |
| **Run configurations / launch jobs** | 12–19 | Entire CRUD + launch + job-polling surface has **no client method**; matrix promises the document but Zed hardcodes ports/context. **MUST: #18; SHOULD: #12–16, #19.** |
| **Debug tunnel** | 74, 75 | No method, **absent from matrix**, `-tags remote` experimental → OPTIONAL / out of scope. |
| **SQL** | 71, 72 | No method, but matrix has a P2 row → OPTIONAL. |
| **Maven** | 68–70 | No method, but matrix has a P2 row → OPTIONAL. |
| **Audit** | 59 | No method, matrix P3 "audit only" → OPTIONAL. |

---

## 3. The 10 capabilities silently missing from FEATURE-MATRIX (12 endpoints)

These are real agent capabilities with **no row at all** in `FEATURE-MATRIX.md`, so a reader of
the promise document cannot tell the Zed integration drops them. Report 04 §2 independently
flagged most of them; this audit confirms all 12 endpoint rows (2, 3, 9, 19, 23, 37, 39, 50,
74, 75, 76, 80).

| # | Capability | Endpoints | Why it matters / disposition |
|---|---|---|---|
| M1 | Dynamic endpoint/port discovery | 2 | Not needed: Zed spawns the agent and reads `agent-state.json`. Document as N/A rather than silently dropping. |
| M2 | Agent self-restart + port handoff | 3 | `ReconnectAgent` restarts the child directly; an agent self-respawn would orphan the tracked handle. OPTIONAL. |
| M3 | Legacy workspace `projects/import` alias | 9 | Redundant with #25. N/A. |
| M4 | Async `LaunchJob` orchestration + unobserved-launch guard | 19 | Orphan-Tomcat protection invisible; needed with #18. SHOULD. |
| M5 | Project unregistration | 23 | Administrative. OPTIONAL. |
| M6 | Deployment history | 37, 39 | Nice-to-have. OPTIONAL. |
| M7 | Operation registry / idempotency status | 50 | Internal protocol concern; Zed sends `requestId` but never polls status. OPTIONAL. |
| M8 | SSH debug tunnel (`-tags remote`) | 74, 75 | Experimental/remote; out of scope v1. OPTIONAL. |
| M9 | JDI bridge host-JDK provisioning | 76 | **Debug depends on it**; matrix omits the step, explaining the non-out-of-box debug flow. SHOULD. |
| M10 | Java method / JUnit discovery | 80 | Needed for Run/Test actions and results tree. SHOULD. |

---

## 4. Ranked MUST-IMPLEMENT work items (all missing, with exact minimal fix)

1. **#66 JDT project model + #10 launch descriptor** — `KairoService::attach_jdt_ls()`: call
   `generate_jdt_project({workspaceId,projectId,rootPath,buildFile,libraries})` (libraries from
   #36), then `launch_descriptor(ws, proj)`, persist descriptor, and have `kairo_java.rs` read it.
   Action `kairo::AttachLanguageServer`; UI: KairoPanel "Attach JDT LS" button + status.
2. **#60 Events stream** — real WS `/events` client with `kairo-secret-v1` subprotocol +
   `kairo_core/src/events.rs` (sequence/gap). Action `kairo::ToggleEvents`; UI: panel Events
   section + `hotreload.status`/`event.gap` banners.
3. **#51 GBK search** — `service.search` exists; add action `kairo::SearchEncoded` and a panel
   query/results UI, expose regex/case/wholeWord/context/previewReplace, bind a key.
4. **#83 JVM redefine (hot swap)** — call `jvm_redefine` from `hot_deploy` when the debug bridge
   is attached (target binding `projectId`/`serverId`); action `kairo::HotSwapClass`; UI: panel
   "Hot Swap" button gated on an attached debug session.
5. **#18 Run-configuration launch** — `client.launch_run_configuration(ws,cfg,mode,async)`;
   action `kairo::LaunchRunConfiguration`; UI: config dropdown + Run/Debug buttons; poll #19.
6. **#36 Ant classpath analysis** — `client.analyze_ant_classpath`; action
   `kairo::AnalyzeAntClasspath`; UI: panel button that merges classpath/sourceRoots into the
   project config and feeds JDT `.classpath` generation.
7. **#77 Java run** — `client.run_java`; actions `kairo::RunJava`/`kairo::DebugJava`; UI: panel
   Run/Debug buttons + JDT CodeLens on main/@Test methods.
8. **#54 + #56 Encoding detect-at-open + validate** — call `validate_encoding` before every
   recode in `save_buffer_with_encoding` and surface `valid:false` as a panel ERROR; add a
   file-open hook (`service.detect_encoding` → status-bar segment). Actions
   `kairo::DetectFileEncoding`; UI: existing Save-with-Encoding button + encoding status line.
9. **SHOULD-tier enablers** — #76 debug JDK provisioning, #80 Java method detection, #18's
   companion #19 launch-job polling, #36's consumer #66 (already #1), #44 debug-running-server,
   #47 recover, #67 port diagnostics, #32 cancel-build, #33–35 custom builds, #12–16/29 build
   & run-config views, #61 JDT status, #78–79 managed sessions. OPTIONAL-tier omissions
   (deploy history, Maven, SQL, audit, SSH tunnel) are documented in §3.

---

## 5. Caveats

- Static audit; no Rust toolchain is available, so no build/compile verification.
- The two implementation files had in-flight edits; YES rows that depend on them are marked
  "snapshot". Re-run this audit after those edits land.
- Historical reports 04/07 predate the current code (fewer actions, no bootstrap, no build
  polling); their gap verdicts were used only as a cross-check, never copied as current fact.
- Nothing here contradicts `docs/VERIFICATION.md`: the Go agent genuinely performs every one
  of the 83 endpoints. The finding is strictly about **which capabilities Zed exposes**.
