# Kairo-for-Zed — Progress & Task Board

> Living document. Updated as work proceeds.
> Branch: `kairo/integration` (based on upstream `main`, 2026-11-19).

## Phase 0 — Setup ✅

- [x] Read both repositories at a high level.
- [x] Create tracking branch `kairo/integration`.
- [x] Set local git identity for commits.
- [x] Create `kairo/` documentation structure.
- [x] Add the AGENTS.md-required review lines to `README.md`.
- [x] Inspect the real toolchain (bundled Tomcat 6.0.53, JDT LS, `kairo-jdi-bridge.jar`, legacy-sample fixture, JDK 21 on PATH).
- [x] Confirm the prebuilt `runtime-agent/kairo-runtime.exe` (15 MB) supports `-port 0`, `-bind`, `-data-dir`, `-bundled-dir`.

## Phase 1 — Understanding ✅

All 5 parallel agents completed:

| Agent | Report | Lines |
|-------|--------|-------|
| Kairo philosophy / ADRs | `analysis/01-kairo-philosophy.md` | 580 |
| Kairo Go runtime agent | `analysis/02-kairo-runtime-agent.md` | 283 |
| Kairo frontend features | `analysis/03-kairo-frontend-features.md` | 436 |
| Zed extension architecture | `analysis/04-zed-extension-arch.md` | 561 |
| Zed core crate layout | `analysis/05-zed-core-layout.md` | 365 |

- [x] Master feature inventory → `docs/FEATURE-MATRIX.md`.
- [x] Target architecture → `docs/ARCHITECTURE.md`.
- [x] Upstream sync strategy → `docs/SYNC-STRATEGY.md`.
- [x] Verified exact upstream insertion points by reading source.

## Phase 2 — Design ✅

- [x] Crate/extension layout: `kairo/crates/kairo_core`, `kairo/crates/kairo_ui`, `kairo/extensions/kairo-java`.
- [x] Decision: **reuse the Go runtime-agent as backend**; Zed is a thin client.
- [x] Decision: GBK handled at file open/save boundary via agent `/encoding/*` endpoints (no `String::from_utf8_lossy`).
- [x] Decision: JSP highlighting via Kairo-owned extension with tree-sitter-jsp + Java injections; JDT LS and JDI DAP bridge registered by the same extension.
- [x] Decision: keep all upstream edits to tagged lines; settings schema gets 2 tagged insertions in `settings_content.rs`.

## Phase 3 — Implementation (in progress)

- [x] `kairo_core` crate scaffold + typed `/api/v1` client + safe DTOs.
- [x] Agent lifecycle: spawn with ephemeral loopback port, discover via `agent-state.json`, process-tree cleanup.
- [x] Zed settings schema (`KairoSettings` + `KairoSettingsContent`).
- [x] Actions + runtime keybindings (compile, deploy, hot-deploy, start/stop server, panel toggle, refresh).
- [x] Global `KairoService` with compile/deploy/start/stop/refresh/load-logs operations.
- [x] `kairo_ui` crate with `KairoPanel` (status, toolchains, servers, paged logs, action buttons) and workspace action handlers.
- [x] `kairo-java` extension: Java/JSP languages, grammars, highlights, injections, JDT LS + DAP adapter registration.
- [x] Upstream registration lines (root Cargo.toml, settings_content.rs, crates/zed Cargo.toml, main.rs, zed.rs).
- [x] Task fixture for `legacy-sample`.
- [ ] Build/verify with `cargo check` — **blocked: no Rust toolchain installed in this environment**.
- [ ] Validate the WASM extension with `zed-extension` — same blocker.
- [x] End-to-end run against `legacy-sample` with real Tomcat 6 (compile → deploy → start → JSP serving → JSP hot reload) at the **agent/API layer**, with evidence in `VERIFICATION.md`. Debug attach and the Rust UI themselves still need a Rust-enabled machine.
- [x] Added `scripts/verify.ps1` for the full Rust + WASM + clippy verification pass.

### Verification constraint

This machine has Java 21 (`JAVA_HOME=E:\Tools\jdk17`) but **no `cargo`/`rustc` on PATH** and no rustup installation. All Rust code was written against API signatures verified directly from the checked-out Zed source, but it has not been compiled. The first action on a machine with Rust 1.98.1 is:

```
cargo check -p kairo_core -p kairo_ui -p zed
cargo build -p kairo_java --target wasm32-wasip2
```

## Decisions log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-19 | Work on dedicated branch `kairo/integration` | Keeps upstream `main` pristine; simplifies rebasing/syncing with Zed authors. |
| 2026-11-19 | All new content under `kairo/`, minimal upstream edits | Small patch surface for upstream sync. |
| 2026-11-19 | Reuse Kairo's Go runtime-agent as backend | Already implements compile/deploy/encoding/search/server lifecycle + JDT LS + DAP bridge; reimplementing in Rust would duplicate a lot and risk divergence. |
| 2026-11-19 | Register settings via `settings_content.rs` (2 lines) rather than editing `default.json` | `default.json` is a hot rebase file; the content struct is stable. |
| 2026-11-19 | JSP/DAP/LSP via Kairo-owned WASM extension | Zero upstream `crates/languages`/`extensions` edits; safe shadowing if upstream later adds Java. |

## How to verify (planned)

1. On a machine with Rust 1.98.1: `cargo check -p kairo_core -p kairo_ui -p zed`.
2. Build extension: `cargo build -p kairo_java --target wasm32-wasip2` and validate with `zed-extension`.
3. Configure Zed settings:
   ```json
   {
     "kairo": {
       "agent_path": "G:\\spaces\\kairo-ide\\runtime-agent\\kairo-runtime.exe",
       "bundled_dir": "G:\\spaces\\kairo-ide\\bundled",
       "compiler_java_home": "<your real JDK 6>",
       "source_level": "1.6",
       "target_level": "1.6",
       "encoding": "gbk"
     }
   }
   ```
4. Open `G:\spaces\kairo-ide\legacy-sample`, run `kairo::ScanProject`, `kairo::Compile`, `kairo::Deploy`, `kairo::StartServer`, attach the `java` debug adapter, open `http://127.0.0.1:<port>/hello.jsp`, edit/save the JSP and confirm immediate effect without Context Reload.
