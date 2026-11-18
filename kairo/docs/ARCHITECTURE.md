# Kairo-for-Zed — Target Architecture

> Status: design draft (v0.1). Refined as analysis reports land.
> Ground truth for Kairo's *intent*: `analysis/01-kairo-philosophy.md`.
> Ground truth for Zed's *insertion points*: `analysis/05-zed-core-layout.md`.

## 1. Goals and non-goals

### Goals

1. Give Zed the **same real workflows** Kairo IDE has for legacy Java Web projects:
   import/scan, encoding-safe editing, Java/JSP intelligence, real compile (Ant /
   `javac`), one-click Tomcat 6 launch/stop, deploy + hot-deploy, DAP/JDWP
   debugging, search, and operator status surfaces.
2. Preserve Kairo's **legacy semantics**: fail-closed toolchain handling (no
   "emulated Java 6"), byte-faithful GBK encoding, safe/atomic deploy, verified
   process identity, loopback-only server binding.
3. Keep the **diff against upstream Zed tiny** (~5–9 registration lines) so the
   work can be rebased whenever Zed upstream is synced (see `SYNC-STRATEGY.md`).

### Non-goals

- Reimplementing the Go runtime agent in Rust in v1. The agent already implements
  the risky legacy logic and is battle-tested by Kairo's own tests; porting it
  would duplicate semantics and risk divergence. (The original ADR-0002 rejected
  Rust only for *iteration speed*, not for technical reasons.)
- Porting Theia/Electron, its DI container, npm packaging, or the browser form.
- Bundling a JDK 6 or Tomcat 6 binary into the Zed repo. Toolchains remain
  imported/pinned and verified; the already-materialised `bundled/` toolchain of
  `kairo-ide` is referenced by path.

## 2. System shape

```
┌──────────────────────────────── Zed process ────────────────────────────────┐
│                                                                             │
│  kairo_ui crate            kairo_core crate                (upstream Zed)   │
│  ┌────────────────┐       ┌──────────────────────┐       ┌───────────────┐  │
│  │ KairoPanel     │◄─────►│ KairoService (entity)│◄─────►│ Workspace /   │  │
│  │ (status, logs, │  RPC  │ - agent lifecycle    │ gpui  │ tasks / LSP / │  │
│  │  actions)      │       │ - /api/v1 HTTP client│       │ DAP / settings│  │
│  └────────────────┘       │ - domain model       │       └───────┬───────┘  │
│                           │ - task templates     │               │          │
│                           │ - actions/keybindings│               │          │
│                           └──────────┬───────────┘               │          │
│                                      │ HTTP 127.0.0.1             │ spawn    │
└──────────────────────────────────────┼────────────────────────────┼──────────┘
                                       │                            │
                        ┌──────────────▼──────────────┐             │
                        │ kairo-runtime.exe (Go)      │◄────────────┘
                        │ - workspace/project/tools   │  (started with
                        │ - build / deploy / search   │   -port 0 -bind 127.0.0.1)
                        │ - encoding detect/recode    │
                        │ - Tomcat6 lifecycle + logs  │
                        │ - JDT LS supervision        │
                        │ - WS /api/v1/events         │
                        └──────────┬──────────────────┘
                                   │ forks / supervises
                    ┌──────────────┼──────────────────┐
                    ▼              ▼                  ▼
              Tomcat 6.0.53   JDT LS (LSP)    kairo-jdi-bridge.jar
              (loopback)      (stdio)         (DAP stdio ↔ JDWP)
```

**Why this shape:** the Go agent is already the single authority for OS effects
(Kairo ADR-0002, architecture goal #2). Keeping it in that role means Zed inherits
the security boundary, process supervision, and all legacy semantics without
reimplementing them. Zed's job becomes: spawn one agent per workspace, talk
`/api/v1`, and surface the results through native Zed UI (tasks, LSP, DAP, panel,
status).

## 3. Components

### 3.1 `kairo/crates/kairo_core` (new)

Pure logic layer, no UI. Responsibilities:

- **Settings** — `KairoSettings` implementing Zed's `Settings` trait. Registered
  from code (`KairoSettings::register(cx)`), not via upstream `default.json`.
  Fields: agent binary path, bundled-dir path, data-dir path, desired bind address
  (always loopback), toolchain slots (languageServer / compiler / tomcat), source
  & target level, project encoding fallback, Tomcat port/context path.
- **Agent lifecycle** — spawn `kairo-runtime.exe -port 0 -bind 127.0.0.1 ...`
  via `util::process::Child::spawn` (process-tree cleanup on Windows), capture the
  chosen port, health-check `GET /api/v1/health`, supervise restart, stop on drop.
- **HTTP client** — typed `/api/v1` client using `http_client` + `AsyncBody::Json`,
  with the envelope (`requestId`, payload), closed error enum, and DTO boundary.
- **Domain model** — Rust mirrors of the agent's safe DTOs (toolchain, project
  layout, build result, deployment, server state, log entry, encoding info).
- **Actions & tasks** — `actions!(kairo, [Compile, Deploy, HotDeploy, StartServer,
  StopServer, RestartServer, ScanProject, SwitchEncoding, OpenPanel, ...])` and
  `TaskTemplate`s so operations also appear in Zed's task/command surfaces.
- **Event stream** — consume `WS /api/v1/events` (`build.*`, `server.*`,
  `log.entry`, `event.gap`) and publish as GPUI events to the UI.

### 3.2 `kairo/crates/kairo_ui` (new)

- `KairoPanel` implementing `workspace::Panel` (persistent name `KairoPanel`,
  bottom dock): toolchain status, server state, build/deploy result, paged logs,
  and the primary action buttons.
- One-line-per-action `cx.observe_new` registration for workspace actions and
  status bar contributions.
- Keybindings registered at runtime with `cx.bind_keys` (no upstream keymap edit).

### 3.3 Language & debug contributions

- **JSP language**: a Kairo-owned language config + tree-sitter grammar for JSP
  (HTML + scriptlets + EL + taglibs), registered via Zed's language registry or a
  Kairo-owned extension. Java reuses Zed's built-in Java support; JDT LS is
  launched/supervised by the agent and attached as an LSP server.
- **Debug adapter**: register `kairo-jdi-bridge.jar` as a DAP adapter
  (`java -jar kairo-jdi-bridge.jar <host> <port>`, default `127.0.0.1:8000`). The
  bridge already speaks DAP-over-stdio and attaches to JDWP with bounded retry
  (verified in its source). Tomcat is started with JDWP on loopback.

### 3.4 Upstream Zed edits (the entire patch surface)

| File | Change |
|------|--------|
| `Cargo.toml` | 2 `members` lines (`kairo/crates/kairo_core`, `kairo/crates/kairo_ui`) + 2 `[workspace.dependencies]` aliases |
| `crates/zed/Cargo.toml` | 2 dependency lines |
| `crates/zed/src/main.rs` | 2 init lines (`kairo_core::init(cx);`, `kairo_ui::init(cx);`) next to `terminal_view::init(cx)` |
| `crates/zed/src/zed.rs` | 1 panel-load line in `initialize_panels`' `futures::join!` |
| `Cargo.lock` | regenerated by Cargo (mechanical rebase) |

All edits are tagged with a trailing `// kairo: integration` marker (Cargo files
use `# kairo: integration`) so they are greppable after a rebase.

## 4. Key flows

### 4.1 Compile (real `javac` / Ant, JDK 1.6 fail-closed)

1. User invokes `kairo::Compile` (palette / keybinding / panel button).
2. `kairo_core` sends `POST /api/v1/builds` with project id (or a `build` task
   routed through the agent).
3. Agent selects `compilerJavaHome` by fingerprint, refuses if no real JDK 6 is
   configured (`toolchain_incompatible` — never fakes success), runs Ant target
   `compile` or `javac` with `source/target 1.6`, GBK-aware diagnostics.
4. `build.completed` event → panel shows diagnostics; output goes to Zed terminal.

### 4.2 Deploy + hot deploy

1. `POST /api/v1/deployments` — preflight (no writes), canonicalized paths,
   `DeploymentOwnerToken` HMAC, atomic file replacement, mirror mode.
2. Hot-reload decision: JSP/CSS → static sync (immediate, no Context Reload);
   compiled classes → sync to `WEB-INF/classes`; structural changes → Context
   Reload (surfaced explicitly).

### 4.3 Launch server + debug

1. Agent starts Tomcat 6.0.53 in a per-instance `CATALINA_BASE` bound to
   `127.0.0.1`, JDWP on loopback port 8000.
2. Zed starts a debug session using the registered `kairo-jdi-bridge` DAP adapter;
   breakpoints, variables, stepping flow through Zed's DAP UI.
3. `server.*` events drive the panel's state; logs stream via NDJSON.

### 4.4 Encoding (GBK)

Agent owns byte-faithful read/recode (`/api/v1/encoding/detect`, `/recode`) with
atomic writes; Zed buffers are ordinary UTF-8 text, so the integration point is
the **file open/save boundary**: detect provenance on open, and on save route
non-UTF-8 files through the agent's recode endpoint. (Exact hook TBD after the
Zed extension-architecture report.)

## 5. Correctness & safety rules carried over from Kairo

- Never assume UTF-8; preserve per-file encoding/EOL provenance; no `String::from_utf8_lossy` round-trips.
- Never fake Java 6 compatibility; fail closed with `toolchain_incompatible`.
- Never expose secrets (marker token, env, JVM options, paths) through DTOs.
- Loopback-only binding for agent, Tomcat, JDWP, JDT LS.
- Verify `ProcessIdentity` before signaling; atomic writes; no destructive
  `RemoveAll` on caller-derived paths.
- Paged logs (no unbounded DOM); status surfaces show JDK + source level +
  encoding + EOL + Tomcat port.

## 6. Open questions (to be resolved from remaining reports)

- Exact Zed mechanism/code path for registering a DAP adapter and an LSP server
  from a Kairo-owned extension vs. core (`analysis/04`).
- Exact list of Kairo commands and which are real vs partial/fake (`analysis/03`).
- Exact `/api/v1` request/response shapes (`analysis/02`).
