# Kairo → Zed Feature Matrix

> Every user-facing and system capability of Kairo IDE, mapped to its Zed
> implementation. Sources: `analysis/01` (why), `analysis/02` (Go backend),
> `analysis/03` (frontend features). Zed-side mechanisms come from `analysis/04`
> and `analysis/05`.
>
> Priority: **P0** = required for the core legacy workflow; **P1** = strongly
> wanted; **P2** = secondary; **P3** = deferred/only if cheap.
> Status: **Planned** / **In progress** / **Done** / **Reuse agent** / **N/A**.

## Legend for implementation strategy

- **Reuse agent** — call the existing Go runtime-agent `/api/v1` endpoint from Zed; no reimplementation.
- **Zed native** — implement with Zed APIs (actions, tasks, LSP, DAP, panels, settings).
- **New crate/extension** — new code under `kairo/`.

---

## 1. Workspace / project / toolchains (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Open workspace, canonicalize path, sandbox root | `/workspaces` POST/GET/DELETE; Electron folder picker | Reuse agent; Zed worktree open triggers `POST /workspaces` | P0 | Planned |
| Detect legacy layout (`src/`, `WebRoot/`, `lib/`, `build.xml`, `web.xml`) | `/workspaces/{id}/scan`, `/projects/detect` | Reuse agent; expose as Zed action `kairo::ScanProject` | P0 | Planned |
| Import wizard (pick → detect → choose JDK/runtime → open) | `project-extension` 4-step wizard; `/projects/import` | Reuse agent for detection; a lightweight Zed modal/GUI in `kairo_ui` | P0 | Planned |
| `.kairo/project.yaml` source of truth, migration from `.legacyflow/` | `internal/repository`, config-schema | Reuse agent; Zed reads project config for status display only | P0 | Reuse agent |
| Project selector / active project / recent projects (MRU) | `/projects/recent`, project-selector widget | Zed worktree + `kairo_ui` status item | P1 | Planned |
| **Three toolchain slots** (LS JRE / compiler JDK / Tomcat JDK) | ADR-0006, `/toolchains`, `/toolchains/import` | Reuse agent; show all three in `KairoPanel`; fail-closed `toolchain_incompatible` surfaced as Zed diagnostic | P0 | Reuse agent + UI |
| Toolchain detect + SHA-256 fingerprint + import by path | `/toolchains/import`, `internal/toolchain` | Reuse agent | P0 | Reuse agent |
| No vendored JDK 6; never fake compatibility | ADR-0006 amendment, B-001 | Preserve semantics in client: never report success when agent returns `toolchain_incompatible` | P0 | Planned |
| Run configurations (versioned doc, before-launch tasks, ports, context path) | `/workspaces/{ws}/run-configurations*` | Reuse agent; surface via Zed tasks UI | P1 | Planned |

## 2. Editing / language intelligence (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Java completion/navigation/refactor (JDT LS) | `/jdtls*`, `java-extension` | Zed LSP client attaches JDT LS using launch descriptor from `GET /workspaces/{ws}/java/launch-descriptor`; agent supervises process (offline pin + SHA) | P0 | Planned |
| JDT project model generator (`.project`, `.classpath`, `.settings`) | `/jdtls/project` POST | Reuse agent; call before LS attach | P0 | Reuse agent |
| JDT LS pinned/offline install, host JDK 21, crash recovery | `internal/jdtls`, ADR-0017 | Reuse agent; Zed only consumes descriptor | P0 | Reuse agent |
| **JSP syntax highlighting** (directives, scriptlets, EL, JSTL/custom tags) | `jsp-extension` Monarch grammar, `highlighting-extension` region scanner | New Kairo language config + tree-sitter JSP grammar (or embedding via HTML+Java). Kairo-owned extension, no upstream `crates/languages` edit | P0 | Planned |
| JSP scriptlet completion/diagnostics via virtual Java docs | `jsp-virtual-java.ts`, sourcemap | Advanced: a Kairo LSP proxy that maps JSP regions to Java virtual docs and forwards to JDT LS | P2 | Planned |
| EL completion/hover/navigation, TLD taglib completion | `el-expression-provider.ts`, `tld-parser.ts` | Kairo LSP proxy (same as above) or Kairo extension context server | P2 | Planned |
| JSP ↔ Servlet / web.xml navigation | `jsp-sourcemap.ts`, `webxml-parser.ts` | Kairo LSP proxy | P2 | Planned |
| XML/properties/JSON highlighting + DTD validation | `jsp-extension` grammars | Zed built-in languages where available; add config for XML DTD/properties | P2 | Planned |
| Structure view / outline for JSP | `xml-structure-view.ts` | Zed outline via LSP or tree-sitter | P2 | Planned |
| Java live templates (150+) | `java-extension` | Zed snippets (Kairo-owned snippet files) | P2 | Planned |
| CodeLens Run/Debug Main/Test | `java-extension` | Zed LSP CodeLens + Kairo actions | P1 | Planned |

## 3. Encoding — byte fidelity (P0, highest data-loss risk)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Detect encoding (BOM/XML/meta/JSP directive/UTF-8/GB18030) | `/encoding/detect`, `internal/encoding` | Reuse agent at file-open boundary | P0 | Reuse agent |
| 8 encodings incl. GBK/GB18030/UTF-8-BOM/ISO-8859-1 | `encoding-extension` dropdown | Kairo settings + agent recode | P0 | Planned |
| Save preserves current encoding + EOL; no silent conversion | ADR-0007 | Kairo save hook routes non-UTF-8 buffers through `/encoding/recode` | P0 | Planned |
| Strict decode on write; representability check (no `?`) | `/encoding/validate`, `StrictDecode` | Reuse agent before every recode | P0 | Reuse agent |
| Atomic recode (temp + fsync + rename) | `internal/atomicfile` | Reuse agent | P0 | Reuse agent |
| Per-file > folder > extension > project override precedence | `encoding-cache-contribution.ts` | Kairo settings resolution + agent cache | P1 | Planned |
| `.properties` = ISO-8859-1 + `\uXXXX` | ADR-0007 | Reuse agent's `PropertiesDecode/Encode` | P1 | Reuse agent |
| Encoding tab suffix + status bar segment | `encoding-tab-decorator.ts` | `kairo_ui` status + buffer title decoration | P1 | Planned |
| GBK full-text search | `/search` with projectEncoding | Reuse agent search | P0 | Reuse agent |

## 4. Build / compile (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Async real `javac` build, source/target 1.6, explicit `-encoding`, argfiles | `POST /builds`, `internal/build` | Reuse agent; Zed action `kairo::Compile` posts request and polls | P0 | Reuse agent + action |
| Ant provider (`build.xml`) | `provider/runtime/ant_provider.go`, `/ant/classpath/analyze` | Reuse agent | P0 | Reuse agent |
| compat gate: reject `-source` below JDK min with `toolchain_incompatible` | `build/compiler.go` | Reuse agent; surface error in panel + diagnostics | P0 | Reuse agent |
| Class-file major version gate (1.6 → 50) | `build/compiler.go` | Reuse agent | P0 | Reuse agent |
| GBK post-compile verification of CJK literals | `services/build.go` | Reuse agent | P0 | Reuse agent |
| Incremental single-file compile | `/jvm/compile`, `/jvm/compile-incremental` | Reuse agent; hot-deploy path | P0 | Reuse agent |
| Build view: history, diagnostics, stdout/stderr, cancel | `build-extension` | `KairoPanel` build tab + Zed diagnostics; cancel via `DELETE /builds/{id}` | P1 | Planned |
| Custom build commands (before-launch tasks) | `/build/custom*` | Zed tasks + agent endpoints | P2 | Planned |
| Maven detect/deps/lifecycle | `/maven/*`, `maven-view-widget.tsx` | Reuse agent; Zed task provider | P2 | Planned |

## 5. Deploy / hot deploy (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Safe deploy: owner token, path containment, symlink checks, atomic writes | `POST /deployments`, `internal/deploy` | Reuse agent | P0 | Reuse agent |
| Merge / mirror modes | `services/diskDeployer` | Reuse agent | P0 | Reuse agent |
| Build & Deploy / Publish one-click | `kairo.buildAndDeploy`, `kairo.publish` | Zed actions chaining build → deploy | P0 | Planned |
| Static hot sync (JSP/CSS immediate, no Context Reload) | `HotReloadWatcher`, `what:"static"` | Reuse agent watcher; Zed save triggers sync | P0 | Reuse agent |
| Compile-only class sync → honest `restart_required` | `hot-deploy-service.ts` | Kairo client maps event to status text; never falsely "synced" | P0 | Planned |
| Class HotSwap via JDWP `RedefineClasses` | `/jvm/redefine`, `internal/debug/hotswap` | Reuse agent; invoked when debugger attached | P1 | Reuse agent |
| Context reload (verified restart, new generation) | `POST /servers/{id}/reload` | Reuse agent | P1 | Reuse agent |
| Hot-reload status events (`hotreload.status`) | EventHub | Kairo client subscribes; panel shows status | P0 | Planned |

## 6. Tomcat 6 server lifecycle (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Start real Tomcat 6.0.53, per-instance `CATALINA_BASE`, loopback only | `POST /servers`, `internal/tomcat6` | Reuse agent | P0 | Reuse agent |
| Stop / restart / debug / recover / reload | `/servers/{id}/*` | Zed actions `kairo::{Start,Stop,Restart,Debug}Server` | P0 | Reuse agent + actions |
| Process identity verification before signaling | `internal/proc`, ADR-0011 | Reuse agent | P0 | Reuse agent |
| Port allocation/leases, port diagnostics | `runtimeplan/ports.go`, `/diagnostics/port` | Reuse agent; surface suggestion in panel | P1 | Reuse agent |
| Paged server logs + NDJSON stream | `/servers/{id}/logs?tail=`, `/events` | Kairo panel paged log view (never infinite scroll) | P0 | Planned |
| Open Application (browser to `http://127.0.0.1:<port><ctx>`) | `kairo.app.open` | Zed action opens system browser | P0 | Planned |
| Readiness probe on a real static resource | `WaitForContextReady` | Reuse agent | P0 | Reuse agent |
| JDWP port bound to loopback, JDI bridge attached | `/servers/{id}/debug`, `kairo-jdi-bridge.jar` | Zed DAP adapter registration + debug launch | P0 | Planned |

## 7. Debug — DAP/JDWP (P0)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| DAP over stdio ↔ JDWP bridge | `bundled/kairo-jdi-bridge.jar` | Register as Zed debug adapter (`java -jar ... 127.0.0.1 8000`) | P0 | Planned |
| Breakpoints (line/exception/hit-count/logpoint) | `internal/debug/breakpoint` | Zed DAP native UI | P0 | Zed native |
| Variables / watches / call stack / stepping | `internal/debug/variable`, `step_control` | Zed DAP native UI | P0 | Zed native |
| Debug console / expression evaluation | `ExprEval.java`, DAP REPL | Zed DAP console | P1 | Zed native |
| JDWP attach retry with bounded backoff (15 s window) | `JdiConnector.java` | Bridge already implements; no work | P0 | Reuse |
| Java 6 gate probe before advertising debug | ADR-0016, `gate_probe.go` | Reuse agent `/debug/adapter/status`; grey out action if gate fails | P0 | Planned |
| Run Main / Run Test / Debug Main / Debug Test | `/java/run`, `java-extension` | Zed tasks + DAP launch configs | P1 | Planned |
| Managed JVM sessions | `/java/runs/{id}` | Reuse agent; panel shows state | P1 | Reuse agent |
| Multi-VM debug orchestration | `multi_vm_orchestrator.go` | Reuse agent (P2) | P2 | Reuse agent |

## 8. Search (P1)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Encoding-aware full-text search, regex, replace preview + undo | `/search`, `search-extension` | Zed native search for UTF-8; route GBK projects through agent `/search` | P1 | Planned |
| Streaming search | `/search/stream` | Reuse agent for large GBK workspaces | P2 | Reuse agent |
| Search Everywhere (files/classes/symbols/actions) | `search-everywhere-*` | Zed's native fuzzy file/symbol/command palettes cover this | P2 | Zed native |
| Find file/class/symbol | Zed equivalents | Zed native | P2 | Zed native |

## 9. VCS / SQL / plugins / secondary views (P2–P3)

| Kairo capability | Source | Zed implementation | Prio | Status |
|---|---|---|---|---|
| Git status/history/blame/stash/cherry-pick | `git-extension` | **Zed native Git** already covers most; map extra commands where needed | P2 | Zed native |
| SVN (18 commands) | `svn-extension` | Zed has no SVN; a Kairo task wrapper could call `svn` CLI (P3) | P3 | Planned |
| Oracle 11g SQL console (pure-Go driver) | `/sql/*`, `sql-extension` | Reuse agent endpoints; a small Kairo SQL panel (P2); needs real Oracle to verify | P2 | Planned |
| `.vsix` plugin install / allowlist | `plugin-extension` | Prefer Zed's extension system; do not port (P3) | P3 | N/A |
| Remote Linux agent / TLS file gateway | `kairo-remote`, `remote-extension` | Zed remote/collaboration; out of scope for v1 | P3 | N/A |
| Enterprise compliance panel (RBAC/audit/retention/SSO) | `theia-product`, security | Reuse agent audit only; Zed auth model differs | P3 | Reuse agent |
| Local history / bookmarks / TODO / paste history / perf dashboard | `theia-product` views | Zed native equivalents where available; otherwise P3 | P3 | Planned |
| IDEA keymaps | `theia-product` | Zed keymap files (Kairo-owned) | P2 | Planned |
| i18n en/zh-CN | `i18n` (~5,500-line catalogs) | Zed's i18n (if any) or English-only first; keep status strings informational | P2 | Planned |
| Themes (kairo-dark, kairo-idea-dark) | `ui-kit` | Zed theme files (Kairo-owned) | P3 | Planned |

## 10. Cross-cutting invariants (must survive in every layer)

| Invariant | Zed enforcement point |
|---|---|
| UI never executes shell directly for legacy ops | All effects via `kairo_core` → agent HTTP |
| Envelope + `requestId` + closed error enum + typed DTO | `kairo_core::client` |
| No secret leakage through DTOs | `kairo_core::domain` safe structs only |
| Loopback-only binding (agent, Tomcat, JDWP, JDT LS) | Settings validation + agent defaults |
| Fail-closed toolchain & encoding | Client never converts errors into success |
| Event sequence/gap detection | `kairo_core::events` |
| Paged logs, no infinite scroll | `KairoPanel` log view |
| Gate debug before advertising | `/debug/adapter/status` check |
