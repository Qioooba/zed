# Kairo IDE — Design Rationale, Purpose & Product Philosophy

> **Analysis scope.** This report documents *why Kairo IDE exists and what it is for*, as a
> handoff artifact for the effort to bring its capabilities into Zed (`kairo/README.md`).
> It is derived exclusively from the source documents listed in §7 — primarily
> `docs/product-requirements.md`, `docs/architecture.md`, `docs/ui-spec.md`,
> `docs/security.md`, `docs/testing.md`, `docs/BUILD.md`, `docs/RUN.md`,
> `docs/BLOCKERS.md`, `docs/MILESTONES.md`, `README.md`, `DELIVERY.md`, and **all ADRs
> 001–0015** (plus ADRs 0016, 0017 and 0026, which bear directly on the legacy constraints
> and the porting decision). Every claim below is traceable to a named source;
> direct quotes are marked with quotation marks and a source reference.
>
> **Important caveat about the sources.** Kairo's docs are explicitly "living documents"
> (`product-requirements.md` header: *"Status: living document. Source of truth for what we
> build and why"*). They were written at different dates and some later waves supersede
> earlier scope decisions (e.g. ADR-0014 defers Debug and Remote Linux Server, while
> `MILESTONES.md` later records both as implemented in Wave 3.1 and Wave 11). Where the
> sources disagree, this report reports **both the original design intent and the later
> reality**, and flags the divergence rather than silently picking one. This matters for a
> porting team: the *philosophy* is stable, the *feature status* moved.

---

## 0. Executive summary

Kairo IDE is a deliberately narrow tool. Its own README says it best:

> "This is **not** IntelliJ IDEA. It is not VS Code. It is a focused tool for a focused job:
> keep a 2008-vintage Servlet/JSP project alive on a 4 GB Windows 10 cloud desktop, with the
> same workflow in Chrome from a Mac laptop when the developer is travelling."
> — `README.md` lines 7–10

The product exists because mainstream IDEs optimize for the *opposite* of this user's
situation: modern frameworks, high-end hardware, network access, and administrative freedom.
Kairo optimizes for: a **single maintainer of a 10+ year-old internal Java Web application**,
on a **locked-down 2 vCPU / 4 GB Windows 10 cloud desktop behind a corporate intranet**,
working with **JDK 1.6, Tomcat 6, Servlet/JSP, mixed GBK/UTF-8 files**, often **offline**, and
needing the **same workflow from a Mac laptop via a browser** while travelling.

Its central design bet is captured in three ideas:

1. **Two deployment forms, one codebase** — desktop (Electron + Theia + Go agent) and
   localhost browser (Theia + Go agent) load the *same compiled frontend* and speak the
   *same `/api/v1` wire protocol*; only platform glue differs. (ADR-0001, ADR-0014)
2. **A hard process boundary** — the UI never executes a shell. All process/filesystem/OS
   effects go through a single **Go Runtime Agent**, which is the security boundary and the
   owner of the domain model. (ADR-0002, `architecture.md` §1)
3. **Fail-closed, byte-faithful legacy handling** — never assume UTF-8, never pretend a
   modern `javac` is Java 6, never silently bundle EOL binaries, never silently upgrade a
   user's project. (ADR-0006, ADR-0007, `BLOCKERS.md`)

For a Rust/Zed port, the transferable essence is **the capability model and the legacy
semantics** (toolchain fingerprinting, three-way JDK separation, encoding provenance, process
identity verification, safe atomic deploy, typed wire DTOs, hot-reload decision logic). The
**Theia/Electron shell, inversify DI, npm packaging and browser-form mechanics are disposable**
and should be redesigned around Zed's native architecture.

---

## 1. The core problem being solved

### 1.1 Mission statement (verbatim)

> "Provide a **lightweight, cross-platform IDE** for developers maintaining legacy Java Web
> projects in low-spec, restricted environments — typically a Windows 10 cloud desktop on a
> corporate intranet. The product must let a developer **import, edit, build, run, search, and
> redeploy** a JDK 1.6 / Tomcat 6 / Servlet / JSP / GBK project without leaving the IDE,
> across two deployment forms (native desktop, localhost browser) from a single codebase."
> — `docs/product-requirements.md` §1, lines 8–14

### 1.2 Who the user is (persona)

From `docs/product-requirements.md` §2.1 (lines 18–29):

| Attribute | Exact source statement | Product consequence |
|---|---|---|
| Role | "**Backend / Ops engineer** maintaining a 10+ year-old internal Java Web product." | Values operational control and reproducibility over modern DX. |
| Machine | "**Windows 10 cloud desktop, 2 vCPU, 4 GB RAM, no admin rights**, behind an intranet." | No system installs, no elevated ports, tiny memory ceiling; toolchains must be imported by path, not installed. |
| Project layout | "non-standard: custom `src/`, `WebRoot/`, `lib/`, sometimes Ant `build.xml`, sometimes raw `javac`." | Project layout must be **detected, not assumed**; workspace scan + import wizard. |
| File encodings | "Files are mixed **GBK and UTF-8**." | Encoding provenance per file is a first-class model; see §5.3. |
| Platform versions | "Tomcat is **6.0.30 or 6.0.53**; the JVM is **JDK 1.6**; the database is **Oracle 11g**." | A dedicated Tomcat 6 provider, Java 6 toolchain handling, and an external SQL tool rather than a built-in DB designer. |
| Mobility | "The same developer also needs to work on a **macOS laptop** when travelling, and an **internal Linux server** when the project needs a real Linux build." | Browser form factor is a requirement, not a convenience; remote Linux was designed then deferred (ADR-0014). |

### 1.3 The anti-persona — explicitly *not* the user

From `docs/product-requirements.md` §2.2 (lines 31–37):

- "New Spring Boot / Cloud / Quarkus projects → IntelliJ IDEA, VS Code + Java Pack, or Eclipse. **We are not a competitor.**"
- "Frontend-heavy React/Vue projects → VS Code / Cursor. We do not optimize for that."
- "High-end developer hardware. We are optimized for 4 GB RAM, not 64 GB."

This is a **positioning** decision as much as a technical one: Kairo is a *maintenance tool*, not a general-purpose IDE. Every "we will not" in the docs (§3.4 below) follows from refusing to chase these other users.

### 1.4 Why the environment constraints dominate the design

| Constraint | Where it is stated | How it shapes architecture / product |
|---|---|---|
| 2 vCPU / 4 GB RAM | `product-requirements.md` §2.1, §6, §7; `architecture.md` §12; `LIMITATIONS.md` §3; `RISK_REGISTER.md` R-003 | Performance budgets are acceptance criteria (≤8 s cold start, ≤350 MB idle RSS); "no AI completion" (conflicts with low-spec); JDT LS heap capped; pnpm chosen over Turborepo/Nx "because caching adds complexity" and to keep `node_modules` small (ADR-0005); container-per-user rejected because "the dev box is 4 GB RAM. We would run out of memory with five users" (ADR-0008); Rust rejected for the agent due to "slower iteration speed for v1" (ADR-0002). |
| No admin rights | `product-requirements.md` §2.1, §7; `PHASE4_COMPLETION_MASTER_PLAN.md` line 33 | "zero-configuration startup," no system-level installation; user-supplied toolchains recorded by fingerprint rather than installed; loopback-only agent binding. |
| Corporate intranet / locked-down network | `product-requirements.md` §1; `security.md` §1; `BUNDLED.md` lines 1–13; `security.md` §9 | "offline upgrade packages only", no auto-update; first-run downloads are opt-in with SHA-256 verification and **fail fast** when `bundled/` is empty — "no silent fallback to a public URL — this is the offline-by-design policy" (`BUNDLED.md`). |
| Mac travel → browser | `README.md` lines 8–10; `product-requirements.md` §2.1, §7; `ui-spec.md` §18; ADR-0014 | The browser form must be **byte-for-byte the same UI** — "No layout, color, icon, or spacing may diverge between the browser and desktop builds" (`ui-spec.md` §18). This is *why* Theia was chosen over a desktop-only shell (ADR-0001). |
| 2008-era Servlet/JSP | `README.md` line 8; `product-requirements.md` §1; `MILESTONES.md` Wave 4 | JSP is a first-class language (scriptlet completion, TLD completion, EL, JSP↔Servlet navigation), not an afterthought; hot-reload must avoid a Tomcat "Context Reload" for JSP saves (success criterion #5). |
| Oracle 11g | `product-requirements.md` §2.1; §5 (out-of-scope: visual DB designer, "Oracle 11g connect is via external SQL tool"); `LIMITATIONS.md` §2 | The SQL console is deliberately read-only, SELECT-only, Oracle-only — an operational convenience, not a DB product. |

---

## 2. Why Theia + Electron + Go agent instead of IntelliJ / VS Code

This is answered most directly by **ADR-0001** (Theia as IDE platform), **ADR-0002** (Go
Runtime Agent split), **ADR-0009** (JDT LS integration), and **ADR-0026** (desktop packaging).

### 2.1 Platform decision — ADR-0001

ADR-0001 §Context enumerates the five options the team considered:

1. "Build our own editor in a webview."
2. "Fork VS Code (Microsoft, Apache-2.0, but very large)."
3. "Fork Zed (GPL/AGPL/commercial — license incompatible)."
4. "Adopt Eclipse Theia (EPL-2.0, smaller surface, designed for this)."
5. "Adopt Eclipse Theia Blueprint or a similar packaged Theia app."

**Decision** (ADR-0001 §Decision): "We adopt **Eclipse Theia as the IDE platform**, layered
with our own product package `@kairo/theia-product`. We do not fork Theia. We consume it as a
set of npm packages and compose our product on top."

| Rejected option | Reason (verbatim, ADR-0001) |
|---|---|
| Own webview editor | "writing an editor from scratch on the budget we have is not credible." |
| Fork VS Code | "the VS Code repo is large, has a heavy telemetry surface, and the Marketplace licensing does not let us re-skin or restrict it cleanly." |
| Fork Zed | "the Zed license is incompatible with our distribution model (closed-source IDE sold to enterprises)." |
| — | **Note for the porting team:** the original rejection of Zed was a *licensing/distribution* objection in 2026, not a technical one. The current effort explicitly revisits this by porting capabilities into Zed rather than forking Kairo's shell. |

**Why Theia was accepted** (ADR-0001 §Decision, bullets):

- "Theia ships Monaco, LSP/DAP, the workbench shell, terminal, and the extension system out of the box."
- "Theia is EPL-2.0, compatible with our Apache-2.0 product."
- "Theia already supports Electron (desktop) and browser (server) forms from a single codebase — exactly our use case."
- "Theia is built on inversify, the same DI used by VS Code, so the extension API is close enough to VS Code that we can claim 'compatible with VS Code extensions' for the common subset."

**Accepted costs** (ADR-0001 §Consequences, Negative): "Theia is a moving target" (pinned to `1.51.x`, quarterly updates); "Theia is large: ~1.2 GB of node_modules with the full IDE" (mitigated by trimming, chunk splitting, lazy-loading JDT LS); "Theia does not own the JDT Language Server"; "Theia does not own Tomcat."

**Why not IntelliJ IDEA specifically:** Kairo never lists IntelliJ as an option to adopt; it names IntelliJ as the *right tool for the anti-persona* ("New Spring Boot / Cloud / Quarkus projects → IntelliJ IDEA" — `product-requirements.md` §2.2) and as the visual benchmark they refuse to chase ("We will **not** try to make Theia look like JetBrains. We will use Theia design tokens, tuned" — §8 Non-goals). A heavyweight JVM IDE with an index-centric memory profile cannot meet the 4 GB / ≤8 s budget.

### 2.2 Why a separate Go Runtime Agent — ADR-0002

ADR-0002 §Context: "The IDE needs to execute things the browser/UI should not execute directly: `javac`, `catalina.sh`, file system operations, port binding, process supervision, log streaming, and on the server form, multi-tenant isolation."

Options considered (ADR-0002 §Context): (1) all in the Theia backend (Node.js); (2) fork-exec via Tauri's Rust sidecar; (3) "a Go service that the Theia backend calls over HTTP"; (4) a Rust service; (5) embed in Electron's main process with a Node API.

**Decision:** "We adopt **option 3: a Go service, the Kairo Runtime Agent**." It is "a single static binary," started in-process by the Theia backend on desktop and as a sidecar in server deployment. It "owns the workspace, the build engine, the process supervisor (Tomcat), the toolchain registry, the search engine, the encoding detectors, and the audit log."

> "The UI never executes shell. The UI talks to the agent over `/api/v1` (HTTP + WebSocket)."
> — ADR-0002 §Decision

**Why Go** (ADR-0002 §Why Go): single static binary per platform (`CGO_ENABLED=0`); "precise control over processes (`syscall.Exec`, `Setpgid`, `Kill` groups)"; "a fast, embedded HTTP/WS server — `net/http` + a small router is enough; we deliberately do not pull in a framework for v1"; "The team has Go experience."

**Why not Node** (ADR-0002 §Why not Node): "We want a hard memory ceiling on the agent. Node's RSS is larger and less predictable"; "race-detector-validated concurrency"; "child-process `kill` is famously platform-fragile."

**Why not Rust** (ADR-0002 §Why not Rust): "Slower iteration speed for v1. We accept the trade." **This is the single most important sentence for the Zed handoff:** the Go agent was a *schedule* choice, not a semantic one. The responsibilities it carries are portable.

**Why not embed in Electron** (ADR-0002): "Couples us to Electron's process model. The server form has no Electron; we would need to re-implement."

**Security consequence** (ADR-0002 §Consequences): "The agent is the security boundary (see ADR-0008). The Theia backend is *not*." `architecture.md` §1 states the same rule as goal #2: "UI never executes shell. All process / filesystem / OS effects go through the Go Runtime Agent."

### 2.3 Why JDT LS instead of a Java parser — ADR-0009

ADR-0009 §Context: "The IDE needs Java language intelligence: completion, navigation, refactor, formatting, diagnostics, documentation. We are not writing a Java parser." Options: Eclipse JDT LS; JavaParser/WALA ("lower fidelity; not real-time; not a refactor engine"); own ECJ-based server ("big investment; not credible for v1").

**Decision:** adopt Eclipse JDT LS, downloaded on first use with SHA-256 verification to `bundled/eclipse-jdt-ls/`. Crucially, the split between the LS runtime JRE and the project compiler is mandated here and in ADR-0006: "JDT LS requires **a JRE to run**, and that JRE is *separate* from the JDK the project compiles against" (ADR-0009 §"JDT LS does not speak Java 6"). The Go agent is "the **process supervisor** for JDT LS: it spawns it, gives it the JVM args, and restarts it on crash with exponential backoff," and "The Theia extension **never** spawns JDT LS itself" — again keeping process control in one place.

Rejected: "Use JDT LS via VS Code Java Extension Pack" — "brings in a UI we don't want, an updater we don't control, and an extension API that targets Marketplace, not workspaces" (ADR-0009).

### 2.4 Desktop packaging — ADR-0026

The Electron desktop is a *packaging shell* for the browser product: `electron-builder` chosen for Windows NSIS / macOS DMG / Linux AppImage, code signing and auto-update. ADR-0026 explicitly states "Desktop 与 Browser 仅在启动层分叉，业务功能不分叉" ("Desktop and Browser fork only at the startup layer; business functionality does not fork"). Tauri was rejected as an alternative: "Tauri 需要 Rust 工具链，且与 Theia 的集成复杂度高，不适合当前阶段." The cost is acknowledged: "安装包体积大（约 200MB+，包含 JDT LS 和 Tomcat 6）."

> **Porting implication.** ADR-0026 is pure platform packaging. None of it should be ported;
> Zed has its own distribution, update and extension mechanisms. Only the *seam* matters:
> "agent lifecycle: browser = manual, desktop = auto-started" (`ADR-0026` §5).

---

## 3. Goals, non-goals, constraints, and deferred items

### 3.1 Functional pillars committed for v1

From `docs/product-requirements.md` §4 (table, lines 68–81):

| Pillar | Meaning | v1 scope |
|---|---|---|
| Workspace | Open / persist / recent / multi-root | yes |
| Editing | Files, folders, tabs, splits, undo, multi-cursor, encoding-aware | yes |
| Search | File / workspace / regex / replace with preview | yes |
| Java | "JDT LS for code intel; separate toolchain for compiler" | yes (ADR-0006) |
| JSP / HTML / CSS / JS | "Syntax, encoding, ES5 default" | yes |
| Build | "Real `javac` (any JDK), Ant-aware, incremental" | yes |
| Tomcat 6 | Run, stop, deploy, logs | yes |
| Debug | DAP, breakpoints, variables, JDWP | **deferred (post-v1)** |
| Remote | Auth, sandbox, audit, reconnect | **deferred (post-v1, ADR-0014)** |
| Plugins | Theia extensions + Go internal adapters | yes (two layers only) |

> **Evolution note:** `MILESTONES.md` later records Debug as implemented (Wave 3.1, with
> 86 Go + 8 frontend tests), and Remote Linux Agent as implemented (Wave 11: File Sync,
> Container Isolation, Session Manager). ADR-0016 introduced a "gate probe" before shipping
> Java 6 debug. The *principle* — prove the Java 6 / Tomcat 6 / JDWP chain before claiming
> debug — survived; the scope decision did not.

### 3.2 Architectural goals (verbatim from `architecture.md` §1)

1. "**One frontend, one backend protocol, two deployment forms.** Desktop (Electron) and localhost Browser. Remote Linux Server is deferred to post-v1 (ADR-0014)."
2. "**UI never executes shell.** All process / filesystem / OS effects go through the Go Runtime Agent."
3. "**Plugin-friendly core.** All `ServerRuntimeProvider`, `BuildProvider`, `EncodingProvider`, `ProjectImporter` are typed Go interfaces with at least one real implementation."
4. "**Replaceable transports.** v1 uses HTTP + WebSocket under `/api/v1`. The wire format is the contract; HTTP is replaceable."
5. "**No JDT-6 vs. Tomcat-6 specifics in core.** They live in providers (ADR-0004)."

`architecture.md` §2 adds the vertical-slice rule (ADR-0015): "JSON is only at the HTTP/WS adapter and persistence codec boundaries"; "inner layers never import outer layers"; "the UI never calls `infrastructure` directly."

### 3.3 Quality bars and success criteria

`product-requirements.md` §6 (quality bars, "goals, with the methodology documented in `docs/testing.md` §5"):

| Metric | Target |
|---|---|
| Cold start to workspace open | ≤ 8 s on 2 vCPU / 4 GB (measured, not estimated) |
| First completion after Java file save | ≤ 1.5 s |
| Incremental compile of a single changed file | ≤ 2 s |
| Full-text search across 10 k-file project | ≤ 3 s |

`architecture.md` §12 expands the budget (cold start ≤4 s, open 1 k-file project ≤2 s, large-file keystroke P95 ≤50 ms, tab switch ≤300 ms, idle RSS ≤350 MB, JDT LS heap 768 MB default with 256 MB–4 GB bounds). `testing.md` §5 gives the measurement methodology, including the honest caveat from `BLOCKERS.md` B-005: "GitHub-hosted runners are Linux/macOS/Windows Server 2022, not Windows 10 cloud desktops... The numbers in `architecture.md` §10 are **goals**, not measured values."

The six **v1 ship criteria** (`product-requirements.md` §7, lines 108–121) are a compact statement of purpose:

1. Import a GBK, JDK 1.6, Tomcat 6 project from a local folder.
2. Open it, show the file tree, detect encoding correctly (**no mojibake**).
3. Edit a `.java` file with full JDT LS completion/navigation, "using a real JDK 6 for compilation."
4. Run on bundled Tomcat 6.0.53 with one click.
5. Save a JSP and see it in a browser **without a Context Reload**.
6. Same project, same workflow in the localhost browser form.

> "If any one of these fails, v1 has not shipped."

Post-v1 criteria add breakpoint→hit→variable inspection (DAP/JDWP) and the same workflow from a Linux server opened in Chrome on Windows.

### 3.4 Non-goals — "do not even try"

From `product-requirements.md` §8 (lines 130–137), verbatim:

- "We will **not** try to make Theia look like JetBrains. We will use Theia design tokens, tuned."
- "We will **not** write our own Java parser. JDT LS is mandatory."
- "We will **not** ship JDK 6 binaries. Toolchain import + fingerprint only."
- "We will **not** auto-upgrade a user's project to Spring Boot. We support what is there."

From `product-requirements.md` §5 (Out of scope for v1, lines 83–94):

- Marketplace / online plugin install.
- Visual database designer ("Oracle 11g connect is via external SQL tool").
- Profiling / flame graphs.
- Cloud sync, account federation, OAuth.
- "AI completion (separate roadmap, conflicts with low-spec target)."
- Git LFS and LFS-based large media workflows.
- DAP/JDWP Debug (endpoint scaffold exists, "no breakpoint→hit proof").
- Remote Linux Server (multi-user, remote auth, audit, container isolation — ADR-0014).
- LegacyFlow runtime plugins / Layer 3 (dynamic loading, JSON-RPC, sandbox — ADR-0014).
- Class HotSwap ("requires JDWP agent integration").

UI-level anti-patterns (`ui-spec.md` §7): "No splash-of-logo welcome," "No modal stacking," "No unsolicited toasts," "No 'AI' surface area in v1," "No infinite-scroll log views. Logs are paged... We do not render 10 000 lines in the DOM."

Further hard limits (`LIMITATIONS.md`): "仅支持 JDK 6 / Tomcat 6 遗留 Java Web 项目" (only JDK 6 / Tomcat 6 legacy Java Web projects); "不支持 Java 8+ 语言特性"; refactoring limited to Organize Imports / Safe Delete / Extract Method; SQL limited to Oracle 11g, SELECT-only, read-only by default; project scale recommended < 10 k files; max 3 concurrent debug sessions; max 10 k search results; max 50 MB diagnostic bundle.

### 3.5 Deferred items and their reasons

`product-requirements.md` §5, `architecture.md` §11, `MILESTONES.md` §"Deferred (post-v1)", `DELIVERY.md` §"Deferred", and ADR-0014 all carry the same deferred list. Consolidated:

| Deferred capability | Reason (as recorded) | Tracking |
|---|---|---|
| Remote Linux Server | "Requires multi-user, auth, audit, container isolation"; "not v1 scope for a product whose primary persona is a single developer on a Windows 10 cloud desktop" | ADR-0014 |
| DAP/JDWP Debug | "`/api/v1/servers/{id}/debug` endpoint exists but no breakpoint→hit proof" | ADR-0014 / MILESTONES; gate in ADR-0016 |
| LegacyFlow runtime plugins (Layer 3) | "Dynamic plugin loading, JSON-RPC bridge, sandboxed subprocess"; designed in ADR-0004 but "never implemented — no plugin loader, no JSON-RPC bridge, no sandboxed subprocess manager" | ADR-0014 supersedes ADR-0004 Layer 3 |
| Class HotSwap | "Requires JDWP agent integration"; later limited to "method body modification only" | future roadmap / `LIMITATIONS.md` |
| Dynamic plugins / marketplace | "Plugin marketplace, online install" | future roadmap / B-006 |
| Remote audit log | "`/api/v1/audit` endpoint exists but remote mode deferred" | ADR-0014 |
| SSO / corporate IdP | "We don't have a target IdP. v1 ships local accounts only." | B-007; auth interface pluggable for v1.1 OIDC |
| TOTP / hardware keys | "TOTP is a v1.1 feature" | `security.md` §3; ADR-0008 follow-ups |
| Per-user cgroup enforcement | v1.1 / v1.2 | ADR-0008 follow-ups |

ADR-0014's *positive* consequences are worth quoting because they explain the philosophy of deferral: "**No placeholder code**: `apps/server/` has been deleted. No more 'TODO: implement' comments masquerading as product features"; "**Simpler security model**: no remote access means no remote auth, no audit, no multi-user isolation in v1"; "**Faster time to v1**: fewer features to build and test." Its acknowledged cost: "Remote Linux Server is a real use case... Deferring it means some users will not have their preferred workflow in v1."

### 3.6 Blockers — what they *cannot* do and what they do instead

`docs/BLOCKERS.md` defines a blocker as "something we **cannot complete in this codebase** without an external dependency we do not have. We list them here so the work that *can* ship is not held hostage by the work that cannot."

| ID | Blocked | Why | "What we do instead" |
|---|---|---|---|
| B-001 | Real `javac` on Java 6 source; true Java 6 `rt.jar` test; debug attach to Java 6 HotSpot | "Oracle JDK 6 is end-of-life... not available on the public Oracle download site for new users. We do not redistribute it." | Import a user-owned JDK 6, fingerprint SHA-256, record provenance; **no silent fallback**; `toolchain_incompatible` error; Java-6 tests gated by `KAIRO_JDK6_HOME` and skipped, not failed. |
| B-002 | Immediately-runnable bundled Tomcat 6 from a clean checkout | "~10 MB; we download it on first use, after SHA-256 verification... gated by user acceptance of the 'Tomcat 6 is EOL' notice." | Deterministic `scripts/fetch-tomcat6.sh` with mirror URL; refuse to bind Manager/AJP/JMX to non-loopback; mandatory first-run notice. |
| B-003 | 100%-accurate JDT LS diagnostics/completion for Java 6 | "JDT LS is a third-party binary; we do not fork it. Upstream Java 6 source-level compatibility is best effort." | Run JDT LS on modern JRE, set `sourceLevel: 1.6`, report warnings explicitly, compile with user's JDK 6 (fail closed without one). |
| B-004 | Reliable JDWP debug attach to Java 6 | "Modern Java debug adapters (e.g. `vscode-java-debug`) target Java 8+ JDWP. Java 6 JDWP protocol differences... are not always honored." | Pluggable `DebugAdapterProvider`; JDT-based DAP front-end + JDI backend; "adapter limitation" surfaced with workaround link. |
| B-005 | True E2E perf numbers on 2 vCPU / 4 GB Windows 10 | CI runners are `windows-2022`, not Windows 10 cloud desktops. | Measure on `windows-2022` 2-core for relative comparison; ship `scripts/perf-collect.ps1`; label numbers as goals until a `perf-reports/` entry exists. |
| B-006 | Online plugin install | "We are not building one in v1." | Local directory or `.lfrpkg` file drop; Extensions view manages enable/disable/remove. |
| B-007 | Corporate SSO | "We don't have a target IdP." | Local accounts with Argon2id; pluggable auth interface for v1.1 OIDC bridge. |

Closure discipline (`BLOCKERS.md` lines 157–164): "We do not close a blocker by quietly ignoring it; we close it by writing the workaround in code, pointing at it from this file, and moving the relevant `MILESTONES.md` line from `[/]` to `[x]`." This *honesty about gaps* is a cultural constraint worth preserving in any port.

### 3.7 Documented internal tensions (read before porting)

The sources contain honest self-corrections that a porting team should not "fix" by ignoring:

- **Encoding of truth:** `security.md` §§2.1, 3.1, 7.1, 11 were corrected on **2026-10-21**: "**代码从来没有实现过按会话的根**" (the code never implemented per-session roots) — roots are **per-process**, so "多租户部署...**不做按用户隔离**" (multi-tenant deployment does not isolate per user). Normative requirement: "远程形态**必须**按「每工作区/每用户一个进程」部署" (remote form must be deployed one process per workspace/user). Several spec items were marked "假绿" ("false green") and then genuinely fixed (Argon2id, CSRF enforcement, login throttle, idle timeout). The diagnostic-package export remains unimplemented (§7.1, §11 #7).
- **Protocol envelope shape differs** between `architecture.md` §5 (`{workspaceId, projectId, requestId, correlationId, payload}` + `ok/payload`/`ok/error`) and `docs/specs/API_CONTRACT_SPEC.md` §1 (`{requestId, payload}` + `{requestId, data, error?}`). The invariant is the envelope + `requestId` + closed error enum + a single generated schema, not the exact field list.
- **JDT LS version pins differ** across docs: ADR-0009 says `1.36.0` (as of writing), ADR-0017 locks `1.21.0` for Java 6 compatibility, `BUNDLED.md` says `1.55.0` (2026-01-13), `architecture.md` §12 budgets 768 MB heap while ADR-0009/`testing.md` say 256 MB. The portable rule: **pin a known-good version, verify SHA-256, allow an offline tarball override, and never silently upgrade.**
- **Scope moved:** v1 deferred Debug/Remote; later waves implemented them. Treat ADR-0014 as the *priority* decision, not a permanent feature ban.

---

## 4. "Two forms, one codebase" and the `/api/v1` wire protocol

### 4.1 The strategy

`product-requirements.md` §3 and `README.md` §"Two forms, one codebase" define it:

| Form | Use case | What runs where |
|---|---|---|
| **Form A — Desktop** (primary v1) | day-to-day work on the same machine | "Electron + Theia + Go agent, all in one process" |
| **Form B — Localhost browser** | "same as desktop, but in Chrome"; Mac travel; quick iteration; CI | "Theia in browser, Go agent on the same box" |
| Form C — Remote Linux Server | real Linux build / central dev box | **deferred to post-v1 (ADR-0014)** |

Shared by both forms (verbatim, `README.md` lines 19–23 and `product-requirements.md` lines 62–66):

- "The same compiled web frontend (Monaco + Theia shell + Kairo extensions)."
- "The same workspace, project, and toolchain model."
- "The same `/api/v1` wire protocol."
- "No business code forks per form. Platform differences live in `runtime-agent/internal/platform/`, nothing else."

`ui-spec.md` §18 makes the parity rule absolute: "The same CSS, React widgets, and i18n keys are used for the browser and desktop products. The only permitted differences are: Desktop uses the Electron main process for window lifecycle and process cleanup; Browser binds the backend to `127.0.0.1`. No layout, color, icon, or spacing may diverge between the two targets."

`RUN.md` documents additional run modes on top of these two forms: headless `Kairo-Server.exe` (a "~2 MB Go launcher" that starts `Kairo.exe --headless`, "~200 MB less than desktop mode"), a development localhost mode, and the (deferred-at-v1) remote Linux server with systemd hardening and TLS.

### 4.2 Process model

`architecture.md` §4: "There is exactly **one Runtime Agent process** per workspace session."

- **Desktop:** Theia backend (Node) ↔ in-proc IPC ↔ Go Runtime Agent; BrowserView (Electron) hosts Monaco + Theia frontend, which talks HTTP/WS `/api/v1` to the agent; the agent forks Tomcat with env, ports and `CATALINA_BASE`.
- **Localhost browser:** "Theia served from localhost, Go agent on the same machine. Same protocol, same domain model, same process layout (minus Electron)."
- **Agent is the security boundary**; desktop binds `127.0.0.1` only with a one-time pairing token (`security.md` §3).

### 4.3 The wire protocol

From `architecture.md` §5 and ADR-0003:

- "Wire format: **JSON over HTTP, NDJSON over WebSocket**. Versioned by URL prefix `/api/v1`."
- Request envelope (verbatim `architecture.md` §5): `workspaceId` "from auth, not user-supplied"; `projectId?`; `requestId` "UUIDv4, client-generated"; `correlationId?`; `payload`.
- Response envelope: `ok: true` + `payload`, or `ok: false` + `error: { code: KairoErrorCode; message; details?; retryable? }`.
- "Error codes are a closed enum... in `packages/protocol/src/errors.ts` and... mirrored in `runtime-agent/internal/api/errors.go`. CI fails if the two diverge."
- **Single source of truth** (ADR-0003 §Decision): a hand-written JSON Schema `packages/protocol/src/protocol.schema.json` from which TypeScript types and Go types are generated; "CI fails if generated files are out of date relative to the schema"; generated files are committed so codegen needs no network.
- **Versioning** (ADR-0003): "The protocol has a major version in the URL (`/api/v1`). Breaking changes bump to `/api/v2`. The agent implements v1 and v2 simultaneously during migration windows."
- **Transport-agnostic by design** (ADR-0002 follow-ups, ADR-0003 §"Why not gRPC from day one"): "Later we may replace the HTTP transport with gRPC; the protocol shapes are transport-agnostic"; JSON chosen because throughput is "human-speed (a developer pressing keys), not machine-speed," and because operators can curl the API.

**Endpoint map (v1, `architecture.md` §5.1)** — this is the capability surface a Zed port must be able to reach one way or another:

| Area | Endpoints |
|---|---|
| Health / auth | `GET /api/v1/health`; `POST /api/v1/auth/login`; `POST /api/v1/auth/logout`; `GET /api/v1/audit` (deferred remote) |
| Workspace | `GET/POST /api/v1/workspaces`; `DELETE /api/v1/workspaces/{id}`; `POST /api/v1/workspaces/{id}/scan` ("Detect legacy layout") |
| Project | `GET /api/v1/projects`; `GET/PUT /api/v1/projects/{id}` |
| Toolchain | `GET /api/v1/toolchains` ("Detect installed JDKs"); `POST /api/v1/toolchains/import` ("Import a JDK and fingerprint it") |
| Build | `POST /api/v1/builds`; `GET /api/v1/builds/{id}` |
| Deploy | `POST /api/v1/deployments`; `GET /api/v1/deployments/{id}` |
| Server | `POST/DELETE /api/v1/servers[/{id}]`; `GET /api/v1/servers/{id}`; `POST /api/v1/servers/{id}/debug` (deferred); `GET .../logs?follow=true` (NDJSON over WS) |
| Search | `POST /api/v1/search` |
| Encoding | `POST /api/v1/encoding/detect` ("Sniff BOM + heuristics"); `POST /api/v1/encoding/recode` ("Read by encoding A, write by encoding B (lossy warning)") |
| Events | `WS /api/v1/events` ("Subscribe to all long-lived streams") |

`docs/specs/API_CONTRACT_SPEC.md` adds the event vocabulary that makes the UI reactive without polling: `build.queued`, `build.started`, `build.completed`, `build.cancelled`, `server.started`, `server.stopped`, `server.crashed`, `log.entry`, `snapshot.required`, `event.gap` — with sequence numbers, workspace IDs, timestamps, and gap detection (also `DELIVERY.md`: "Log cursor: Monotonic cursor + gap detection"). It also states the discipline: "HTTP handler 不传 `json.RawMessage` 给 service" and "所有 endpoint 都有 request/response contract test."

> **Porting note:** the *protocol* is a viable integration seam even if the rest of the
> Theia shell is discarded. A Zed extension can talk to a Go (or Rust) agent over the same
> envelope contract, keeping the domain semantics while replacing the frontend.

---

## 5. Why JDK 1.6 / Tomcat 6 / GBK matter — collected mentions

These three constraints are not incidental; they are the reason the product exists. Below is
every substantive mention found in the designated sources, grouped by theme.

### 5.1 JDK 1.6 / Java 6

| Source | Exact statement / rule |
|---|---|
| `README.md` l.3–5 | "A lightweight, cross-platform IDE for legacy Java Web projects: JDK 1.6 / Tomcat 6 / Servlet / JSP / GBK." |
| `product-requirements.md` §2.1 | "the JVM is **JDK 1.6**"; success criteria #3 requires "using a real JDK 6 for compilation." |
| `product-requirements.md` §8 | "We will **not** ship JDK 6 binaries. Toolchain import + fingerprint only." |
| `product-requirements.md` §5 | "DAP/JDWP Debug — endpoint scaffold exists but no breakpoint→hit proof." |
| `architecture.md` §5.1 | `GET /api/v1/toolchains`; `POST /api/v1/toolchains/import` "Import a JDK and fingerprint it." |
| `architecture.md` §6 | `java:` config has three homes: `languageServer`, `compiler`, `runtime` with `sourceLevel/targetLevel`, `permGen?`. |
| `architecture.md` §12 | "Incremental single-file compile (JDK 21 → JDK 6 source) ≤ 2 s." |
| **ADR-0006** (whole ADR) | "**JDK 6 / Tomcat 6 EOL strategy**." "We are not in the business of distributing Oracle JDK 6 binaries. We are not in the business of pretending a Java 21 `javac --release 6` is 'the same' as Java 6 in every respect (it is not, for annotation processing, for `sun.misc.Unsafe` access, and for some compiler-bug edge cases)." Three toolchain slots, never one; import wizard runs `java -version`/`javac -version`, computes SHA-256, records provenance, does **not** copy the JDK into the repo. Fingerprint format `sha256:<hex>`; "the agent refuses to use an imported toolchain whose fingerprint is empty." **Amendment 2026-09-25 (DBG-17):** the "emulated-v6 via `--release 6`" fallback "does not exist — modern javac rejects release 6 outright, and the compiler fails closed with `toolchain_incompatible`." |
| `BLOCKERS.md` B-001 | "Oracle JDK 6 is end-of-life... We do not redistribute it"; "modern `javac --release 6` does not exist (Java 17+ rejects release 6; verified: `error: release version 6 not supported`)"; "Java 6 compilation requires a real, separately installed JDK 6 toolchain; without one the Java-6-target use case is reported as unconfigured, never as 'emulated' success." |
| `BLOCKERS.md` B-003 | JDT LS Java 6 compatibility is "best effort"; set `sourceLevel: 1.6`; "never silently downgrade to a higher source level"; "The compile step still uses the user's Java 6 JDK, independent of JDT LS; without one the build fails closed." |
| `BLOCKERS.md` B-004 | "Modern Java debug adapters (e.g. `vscode-java-debug`) target Java 8+ JDWP. Java 6 JDWP protocol differences (specifically, some reference type info commands) are not always honored." |
| **ADR-0009** §"JDT LS does not speak Java 6" | "The default JDT LS does not know `--release 6`. It is targeted at Java 8+"; `languageServerJavaHome` modern LTS vs `compilerJavaHome` user's Java 6; crash recovery (3 restarts in 60 s, then banner); default heap 256 MB. |
| **ADR-0016** (whole ADR) | Java 6 / Tomcat 6 / DAP "技术闸门" (technical gate). Probe: JDK 6 JDWP with `-Xdebug -Xrunjdwp`, Tomcat 6 debug startup, `JDWP-Handshake`, DAP connectivity, breakpoint hit, variables, step, on Windows 10 **and** macOS. Risks: pre-u45 JDWP bugs, DAP adapters targeting JDK 8+, source/class line-number mismatch, platform differences; "如果闸门探针在任一维度失败... Phase 1 将不带 Debug 功能发布" (if the gate fails on any dimension, Phase 1 ships without Debug). |
| **ADR-0017** (whole ADR) | JDT LS compatibility matrix for "Java 6 Web 项目（GBK 编码、Ant 构建、依赖不完整）"; locks JDT LS **1.21.0** because it runs on JDK 17 and "已确认可以正确处理 Java 6 source/target 项目"; degrades type-level features when dependencies are missing; "如果未来 JDT LS 上游完全移除 Java 6 支持，需要评估替代方案（如使用旧版 ECJ 或自定义语言服务器）." |
| `testing.md` §5.3 | "`--release 6` does not exist on modern JDKs — Java-6-target builds require a real JDK 6 toolchain and fail closed as `toolchain_incompatible` without one." |
| `testing.md` §8 | "We do not test against Oracle JDK 6 in CI (license + availability). There is currently no opt-in JDK 6 test path." |
| `BUILD.md` prerequisites | "JDK 6 is **not** required to build. It is required to *run* Java 6 compilation in the IDE." Env var `KAIRO_JDK6_HOME`: "If set, integration tests use this Java 6." |
| `RISK_REGISTER.md` | R-001 "JDK 6 JDWP 不兼容" → ADR-0016 gate; R-002 "JDT LS 停止支持 Java 6 source/target" → lock JDT LS 1.21.0 + SHA-256. |
| `LIMITATIONS.md` | "不支持 Java 8+ 语言特性" (no Java 8+ language features); "仅支持 JDK 6 / Tomcat 6 遗留 Java Web 项目." |
| `ui-spec.md` §1.3 / §1 (status bar) | `☕ JDK 6 (1.6.0_45) · ☕ Source 6` — the status bar constantly surfaces the active JDK/source level. |

**Design lesson:** never fake compatibility. The 2026-09-25 amendment is a deliberate, public admission that an earlier design idea was wrong. Preserve the *fail-closed* semantic (`toolchain_incompatible` rather than a bogus success) in any port.

### 5.2 Tomcat 6

| Source | Exact statement / rule |
|---|---|
| `product-requirements.md` §2.1 | "Tomcat is **6.0.30 or 6.0.53**." |
| `product-requirements.md` §7 | "The user can run the project on a bundled Tomcat 6.0.53 with one click"; "save a JSP and see the change in a browser **without a Context Reload**." |
| `product-requirements.md` §4 | Pillar "Tomcat 6 — Run, stop, deploy, logs" in v1 scope. |
| `architecture.md` §6 | `serverRuntime: type: tomcat6 # resolves to a ServerRuntimeProvider`; config: ports, contextPath, env, jmx, debug. |
| `architecture.md` §3 | `bundled/tomcat6/` — "Apache Tomcat 6.0.53 archive + checksum." |
| `architecture.md` §11 | Tomcat specifics live in providers, not core (goal #5 / ADR-0004). |
| **ADR-0006** §"Tomcat 6 bundling" / §"Tomcat 6 plugin" | "We **do not** vendor a Tomcat 6.0.53 binary in git." Agent downloads from official Apache mirror, checks SHA-256 against `bundled/tomcat6/SHA256SUMS`, unpacks to `bundled/tomcat6/apache-tomcat-6.0.53/`. Mandatory notice: "Tomcat 6 is end-of-life. This is a development convenience. Do not expose this server to untrusted networks." "The agent never starts Tomcat 6 with its Manager, AJP, or JMX ports bound to a non-loopback address." "All Tomcat-6 specific code lives in `bundled/plugins/org.kairo.tomcat6/`"; "The core does not import `catalina.sh` or any `catalina.*` Java package." Plugin implements `ServerRuntimeProvider`, `BuildProvider`, `DeploymentProvider`, `DebugAdapterProvider`. |
| `BLOCKERS.md` B-002 | "An immediately-runnable bundled Tomcat 6.0.53 from a clean checkout, on a network that cannot reach the Apache archive" is blocked; workaround: deterministic fetch + SHA-256 + EOL acceptance + loopback-only ports. |
| `BLOCKERS.md` B-004 | Tomcat 6 debug requires a JDWP-capable adapter path. |
| `security.md` §4 | "Tomcat is **never** started on a public interface. The agent starts Tomcat on `127.0.0.1` and (in server mode) reverse-proxies `/<contextPath>/` to it. Manager, AJP, JMX, and JDWP ports are bound to `127.0.0.1` and never proxied." |
| `security.md` §5 | "Every Tomcat instance runs as the same OS user as the agent, but inside a per-instance `CATALINA_BASE` with a fresh `work/`, `temp/`, `conf/`, `logs/`"; "refuses to start a Tomcat whose `CATALINA_BASE` is not inside the workspace." |
| **ADR-0011** (whole ADR) | Tomcat lifecycle: `ServerID` independent of `ProjectID`; `RuntimeID` `tomcat6` builtin or `rtm_`; `Generation` per process start; `ProcessIdentity` (PID + executable + start time + `CatalinaBase` + marker token) verified before signaling; state machine with `preparing/starting/running/stopping/restarting/failed/crashed`; "Agent exit MUST stop all Tomcat processes it manages. Cross-lifecycle process recovery is intentionally NOT supported." |
| **ADR-0012** / **ADR-0013** | Never leak `CatalinaBase`, `CatalinaHome`, `ShutdownPort`, `JVMOptions`, `Env`, `MarkerToken` to HTTP; deployment writes guarded by an HMAC `DeploymentOwnerToken` bound to `(ws, proj, srv, root)`. |
| **ADR-0016** | Tomcat 6.0.53 debug-mode startup verified by the gate probe; degrade to unsupported-platform banner if probe fails. |
| `testing.md` §8 | "We do not run a real Tomcat 6 in CI. The wrapper is unit-tested; the E2E server-start step is gated on B-002... gated items fail the run — they are recorded, not silently passed." |
| `RUN.md` first run | "Apache Tomcat 6.0.53 — ~10 MB" auto-downloaded on first use and stored under `bundled/`. |
| `MILESTONES.md` | "Bundled Tomcat 6 — ✅ verified — SHA-256 checksum, Apache-2.0"; Tomcat6Provider hot-reload watcher, `syncCompiledClasses`, "JSP/CSS即时生效" (JSP/CSS take effect immediately). |

**Design lesson:** EOL software is treated as a *hazard to be contained*, not a dependency to be hidden: no vendoring, explicit user consent, checksums, loopback-only ports, isolated `CATALINA_BASE`, and verified process identity before any kill signal.

### 5.3 GBK / encoding

| Source | Exact statement / rule |
|---|---|
| `product-requirements.md` §2.1 | "Files are mixed **GBK and UTF-8**." |
| `product-requirements.md` §2.2 / architecture | JSP files often GBK; `encoding: { source, jvm, request, response }` in project config. |
| `product-requirements.md` §7 | Success criterion #2: "detects encoding correctly (**no mojibake**)." |
| `architecture.md` §9 ("Encoding discipline (this is important)") | "**All files on disk are bytes.** Kairo never assumes UTF-8."; "Every `TextDocument` has a `DocumentEncoding` with `byteOrderMark | detected | explicit` provenance"; "Saving preserves encoding unless the user picks 'Save with encoding'"; "Workspace encoding is a *fallback*, not a *mandate*." |
| `architecture.md` §5.1 | `POST /api/v1/encoding/detect` "Sniff BOM + heuristics"; `/encoding/recode` "Read by encoding A, write by encoding B (lossy warning)." |
| **ADR-0007** (whole ADR) | "Legacy Chinese projects mix GBK (GB2312, GB18030) and UTF-8 files, sometimes with a UTF-8 BOM, sometimes without. `properties` files are ISO-8859-1 with `\uXXXX` escapes. JSP files are often GBK even when the rest of the project is UTF-8. **Saving a file with the wrong encoding destroys data, silently.** We have seen developers lose days of work to 'the IDE auto-saved as UTF-8' bugs. The product's encoding discipline is one of the highest-stakes parts of the UX." Detection order: BOM → project default → heuristics → "Trust the file. The detector is a *guess*"; save preserves `current` encoding and EOL; "Save with encoding" shows a transformation preview and lossy warning; `*.properties` always ISO-8859-1 with `\uXXXX`; registry aliases (`cp936`, `ms936` → `gbk`); search engine "must read files with their declared encoding, not assume UTF-8." Rejected: "Always assume UTF-8... Detect per-line... Convert everything to UTF-8 on import." |
| `docs/specs/ENCODING_SAFETY_SPEC.md` §0–2 | "确保... GBK/GB18030/UTF-8/ISO-8859-1 等编码环境下，文件打开、编辑、保存、批量转换全部字节级安全." Documents bugs being fixed: "浏览器 `TextEncoder` 只产生 UTF-8，用它验证 GBK/ISO-8859-1 是错误的 (N-025)"; "`Recode` 用 `os.WriteFile` 原地覆盖，进程崩溃或磁盘满可能损坏文件 (N-027)"; "普通保存... 保留当前编码和 EOL，不做隐式转换"; recode via temp file + flush/sync + preserve mode + atomic rename. |
| `security.md` §6 / `RUN.md` | Diagnostic bundle excludes source files and secrets; the encoding indicator in the status bar opens "Save with encoding." |
| `RISK_REGISTER.md` R-004 | "真实遗留项目编码破坏" (real legacy project encoding corruption), impact **high** → "编码安全机制，repo diff 验证." |
| `testing.md` §3 | `legacy-sample/` contains `WebRoot/hello.jsp` (GBK), `WebRoot/utf8.jsp` (UTF-8), and a JSTL taglib usage — mixed encodings are a permanent test fixture. |
| `MILESTONES.md` lines 170, 180 | E2E fixes: "javac GBK/双语归一化"; "Node Buffer 不支持 gbk → 改 agent recode API；特殊字符集剔除 GBK 外的 ♠♣♥♦/CJK 兼容字（Go 逐 rune 验证）." |
| `ui-spec.md` §1.3 | Status bar shows `ABC encoding`; "Each segment is a click target. Encoding opens 'Save with encoding'." |
| `ADR-0017` §compat matrix | "GBK 项目：通过 ADR-0007 的编码处理机制，JDT LS 可以正确读取 GBK 编码的源文件，所有功能正常工作." |

**Design lesson:** encoding is treated as *data integrity*, not display polish. Any Zed port must preserve byte-level round-tripping and provenance, and must not route GBK validation through a browser `TextEncoder` (which only produces UTF-8).

### 5.4 The three-toolchain split (the key legacy mechanism)

ADR-0006 §"JDK toolchain" and ADR-0009 together mandate **three separate slots, never one**:

| Slot | Purpose | Default |
|---|---|---|
| `languageServerJavaHome` | JRE that runs JDT LS | Modern LTS JRE (17 or 21), auto-downloaded |
| `compilerJavaHome` | JDK that compiles project sources | "a Java 6 JDK that the user imports"; no silent emulation |
| `tomcatJavaHome` | JDK/JRE that runs Tomcat | "matches `compilerJavaHome`" |

> "The IDE **never** picks the toolchain for the user. It shows what was chosen, with provenance, and a button to change." — ADR-0006

The config model in `architecture.md` §6 mirrors this: `java.languageServer.{javaHome,vmOptions}`, `java.compiler.{javaHome,sourceLevel,targetLevel,args}`, `java.runtime.{javaHome,vmOptions,permGen?}`. The UI surfaces all three in the status bar (`jdk 6`, `Source 6`, encoding, EOL, Tomcat port).

**This is the single most important capability to port.** A modern editor that simply "uses the project JDK" will either fail to run JDT LS or silently compile with the wrong bytecode target — exactly the failure modes Kairo was built to prevent.

---

## 6. Handoff summary — porting these capabilities to Zed

The receiving project's own `kairo/README.md` states the goal: "give Zed the same real, clickable compile / launch / deploy / debug / syntax-highlighting / hot-deploy workflow, while keeping the diff against upstream Zed small enough to rebase on future Zed releases." The following is what a porting team **must preserve** versus what is **platform-specific and should be redesigned**.

### 6.1 MUST preserve — capabilities and invariants

| # | Capability / invariant | Source anchor | Why it must survive the port |
|---|---|---|---|
| 1 | **One authority for OS effects.** UI/editor never spawns shells directly; a single background service owns process/filesystem/network effects. | ADR-0002, `architecture.md` §1 #2 | Security boundary, process supervision, and audit all depend on one chokepoint. In Zed this becomes a trusted sidecar/agent, not Theia backend IPC. |
| 2 | **Typed wire contract with envelope, correlation IDs, closed error enum, and generated/mirrored DTOs.** | ADR-0003, `architecture.md` §5, `API_CONTRACT_SPEC.md` | Keeps frontend and backend independently evolvable; prevents `json.RawMessage` bugs. Use Zed's RPC/serialization but keep the shape and versioning discipline. |
| 3 | **Three-toolchain separation + toolchain import by fingerprint, fail-closed on missing JDK 6.** | ADR-0006 (as amended), ADR-0009, B-001/B-003 | Core legacy semantic. `toolchain_incompatible` must never become a silent fake success. |
| 4 | **Byte-faithful encoding model** (BOM/declared/detected provenance, per-file EOL, properties = ISO-8859-1 + `\uXXXX`, save preserves bytes, recode atomic + preview + lossy warning). | ADR-0007, `ENCODING_SAFETY_SPEC.md`, `architecture.md` §9 | Data-integrity, the highest-stakes UX. Zed's buffer model must be adapted to store provenance, not just decoded text. |
| 5 | **Legacy project detection & import** (`/scan` → `src/`, `WebRoot/`, `lib/`, `build.xml`, `web.xml`; `.legacyflow/project.yaml`-style config with `schemaVersion`). | `architecture.md` §5.1/§6, `API_CONTRACT_SPEC.md` §2.1–2.2 | Non-standard 2008 layouts cannot be assumed; the read-only scan + user review flow is the product's entry point. |
| 6 | **Real build**: Ant-aware + `javac`, per-file incremental, source/target levels, Chinese diagnostic parsing, argfiles for Windows command lines. | `product-requirements.md` §4, ADR-0010 §6, `MILESTONES.md` "Build & Deploy" | "Real `javac`" is a mission statement, not a simulation. |
| 7 | **Safe deploy** with preflight (zero writes), canonicalized relative paths, symlink checks, ownership token, atomic single-file replacement, mirror mode, no `os.RemoveAll` on caller-derived targets. | ADR-0010 §7, ADR-0013, `security.md` §2 | Destructive operation over a live webapp; correctness here prevents data loss. |
| 8 | **Server lifecycle**: opaque IDs, logical server vs process `Generation`, `ProcessIdentity` verification before signaling, validated state machine, desired-vs-observed state, agent-exit stops managed Tomcat, no cross-lifecycle reattach. | ADR-0011, ADR-0013 | PID reuse and orphaned Tomcat are real hazards on Windows; this design is portable to any OS. |
| 9 | **Hot-reload decision logic** (Static Sync vs Compile-only vs Context Reload; JSP/CSS immediate; compiled classes synced to `WEB-INF/classes`). | `product-requirements.md` §7 #5, `ui-spec.md` §2.3 tokens, `MILESTONES.md` Wave 16 | The single most visible "keep a legacy project alive" workflow; must remain one-click and understandable. |
| 10 | **Java intelligence via JDT LS**, pinned + SHA-verified, downloaded/offline override, crash recovery, dependency-aware degradation. | ADR-0009, ADR-0017, `BUNDLED.md` | Reimplementing a Java parser is an explicit non-goal. Zed can host the same LS through its LSP support. |
| 11 | **JSP/Servlet first-class treatment**: scriptlet/TLD/EL completion, JSP↔Servlet/web.xml navigation. | `MILESTONES.md` Wave 4, `product-requirements.md` §4 | The legacy app is a JSP app; treating JSP as plain text breaks the persona's workflow. |
| 12 | **Security model**: path canonicalization chokepoint (realpath, containment, `bundled/` read-only), loopback-only desktop binding, token not in URL, Argon2id, CSRF on session-authenticated writes, login throttle, idle timeout, append-only audit, fail-closed roots. | `security.md` §§1–4, ADR-0008, ADR-0012 | Non-negotiable; reuse the *rules* even though the transport/host changes. Note §2.1: one process per workspace for multi-user deployments. |
| 13 | **Secrets never leak through API DTOs** (typed boundary; sensitive fields not copied; marker token, env, JVM options, paths excluded). | ADR-0012, ADR-0013 | "Once a secret leaves the process, it is effectively impossible to recall." |
| 14 | **Offline-by-design supply chain**: SHA-256 verification, pinned versions, optional offline tarball/override, no auto-update, no silent downloads. | `BUNDLED.md`, ADR-0009, `security.md` §9, B-002 | Air-gapped intranet reality; a port must not introduce network dependencies Zed users cannot satisfy. |
| 15 | **Fail-closed honesty**: gated tests, blockers documented with workarounds, no false green, gated features greyed out rather than half-shipped. | `BLOCKERS.md`, `testing.md` §8, ADR-0016, `security.md` §3.1/§7.1 | Cultural invariant; prevents a port from claiming capabilities it has not proven. |
| 16 | **Gate philosophy for debug**: prove Java 6 + Tomcat 6 + JDWP end-to-end before advertising breakpoint debug. | ADR-0016, ADR-0006 B-004 | Modern DAP adapters target Java 8+; assume incompatibility until proven otherwise. |
| 17 | **Performance & resource budget as acceptance criteria** (cold start ≤8 s, idle RSS, capped JDT heap, paged logs, no AI, virtualization). | `product-requirements.md` §6, `architecture.md` §12, `LIMITATIONS.md`, `ui-spec.md` §7 | The 4 GB machine is the user; a port that ignores this reproduces the problem it solves. |
| 18 | **i18n (en/zh-CN) and accessible status surfaces** (encoding/JDK/source level/Tomcat visible, glyph+text not color-only). | `ui-spec.md` §§6, 13, 20.5; ADR-0017 context | Bilingual team; status bar is how the user verifies the legacy toolchain. Zed has its own i18n — map the strings, keep the information. |

### 6.2 Platform-specific — redesign, do not port

| Kairo mechanism | Why it is platform-specific | Zed-side redesign direction |
|---|---|---|
| Eclipse Theia as platform, inversify DI, `@kairo/theia-product` + `packages/*-extension` | ADR-0001 chose it for browser+desktop from one codebase and VS Code extension compatibility; none of that is needed once the host is Zed. It was also "large: ~1.2 GB of node_modules" and a moving target. | Implement capabilities as Zed extensions / native workspace contributions; use Zed's LSP, tasks, debug adapter and panel APIs. |
| Electron shell, `BrowserView`, `apps/desktop/main.ts`, `electron-builder`, NSIS/DMG/AppImage, code signing, `electron-updater` | ADR-0026; pure packaging/window lifecycle. | Zed's own distribution, windowing and update mechanisms. Keep only the *seam*: agent auto-start/lifecycle. |
| Theia browser form (`apps/browser`, localhost bind) | Exists to serve the same frontend to a travelling Mac user. | Zed clients are native; remote access becomes Zed's own collaboration/remote story, not a served Theia bundle. |
| Theia `EncodingDocument` wrapper + Monaco model save hooks | Tightly coupled to Theia/Monaco text model. | Reimplement provenance-aware buffer handling against Zed's buffer/routing APIs; keep ADR-0007's semantics. |
| `@kairo/protocol` TypeScript package + `json-schema-to-typescript`/Go codegen pipeline | Language/codegen tooling is Theia/npm-specific. | Keep the *contract and versioning rules* (envelope, `requestId`, error enum, v1→v2 migration), regenerate for Rust/WASM/JSON-RPC as appropriate. |
| `runtime-agent/internal/platform/` OS shims, `MoveFileExW` / Unix fsync, `setpgid`, cgroups v2 | Implemented against Go stdlib and Windows/macOS/Linux specifics. | Re-express with Rust stdlib / Zed's process APIs; preserve *behavior* (atomic rename, identity checks, loopback binding), not the Go calls. |
| Go-specific concurrency choices (`-race`, goroutines, HMAC token in Go) | ADR-0002 chose Go for schedule and static binary, not semantics. | Rust is acceptable here — the original "why not Rust" was "slower iteration speed for v1," not a technical objection. |
| LegacyFlow Layer 3 runtime plugins (JSON-RPC subprocess, manifests, permission enums) | Explicitly deferred in ADR-0014; never implemented. | Do not port. If extensibility is needed, use Zed's extension host. |
| Theia design tokens / `kairo-theme.css` / React `Kairo*` widgets / Lumino menus | `ui-spec.md` §§2, 3, 15–20. | Re-skin with Zed's theme/UI primitives; preserve only the *information architecture* (status bar facts, 4-state views, one primary action, paged logs). |
| Self-signed cert / systemd unit / kairo-admin CLI from `RUN.md` | Server-form deployment detail; remote deferred at v1. | Map to Zed's remote/collaboration story only if/when remote is in scope. |

### 6.3 Recommended porting seams (derived from the design, not invented)

1. **Adopt the `/api/v1` capability surface as the contract** for a background agent reachable from Zed. The protocol is deliberately transport-agnostic (ADR-0002/0003), so JSON-over-a-Zed-RPC channel is a legitimate evolution.
2. **Port the domain rules before the UI**: toolchain model → encoding model → plan/deploy/security → server lifecycle → hot-reload decision. These are where the legacy risk lives.
3. **Keep JDT LS as the Java intelligence provider** and let Zed host the LSP client; reuse the pin-and-verify policy from ADR-0017/`BUNDLED.md`.
4. **Mirror the status bar's information content** (active JDK + source level + encoding + EOL + Tomcat/connection) even if the visual design changes completely — it is the user's verification surface.
5. **Do not port "emulated v6".** It was retracted in ADR-0006's 2026-09-25 amendment; carry forward `toolchain_incompatible` fail-closed behavior.
6. **Do not port deferred/placeholder code.** ADR-0014's lesson was to delete `apps/server/` rather than keep fake features.

### 6.4 Risks the sourcing docs explicitly flag for a port

- **Version skew in the docs** (§3.7): pin decisions by ADR number and date, not by a single "current" value; JDT LS version, heap, envelope shape and feature scope all shifted.
- **No real hardware validation**: B-005 and `ROADMAP.md` leave "真实遗留项目 E2E 验证 (需 Java 6 + Tomcat 6 真实环境)" and "Windows 10 真实环境完整验证" open. A port inherits this gap; do not treat CI green as proof on the target box.
- **Encoding is the highest data-loss risk** (R-004); the browser `TextEncoder` trap (N-025) and in-place `os.WriteFile` recode trap (N-027) must not be reintroduced in a Rust/Zed implementation (e.g. via a naive `String::from_utf8_lossy`).
- **Secrets/DTO creep** (ADR-0012): adding a field to a domain struct must not automatically expose it; keep an explicit safe DTO.
- **Multi-tenant isolation** (`security.md` §2.1): roots are per-process, not per-session; a shared-agent design silently breaks the documented threat model.

---

## 7. Source index

Primary documents read for this report (all under `G:\spaces\kairo-ide\` unless noted):

| Source | Sections used |
|---|---|
| `README.md` | Positioning, two forms, shared assets, status |
| `DELIVERY.md` | Verified / partial / deferred status, "15 ADRs (001-0015)" |
| `docs/product-requirements.md` | §1 Mission, §2 persona/anti-persona, §3 two forms, §4 pillars, §5 out of scope, §6 quality bars, §7 success criteria, §8 non-goals, §9 doc set |
| `docs/architecture.md` | §1 architectural goals, §2 vertical slices, §3 monorepo, §4 process model, §5 protocol + endpoints, §6 model, §7 plugins, §8 composition root, §9 encoding, §11 deferred, §12 perf budget, §13 versioning |
| `docs/ui-spec.md` | §1 layout/status bar, §2 tokens, §5 states, §6 a11y, §7 anti-patterns, §13 i18n, §18 browser/desktop parity, §20 UI guidelines |
| `docs/security.md` | §1 threat model, §2/§2.1 sandbox + per-process roots, §3/§3.1 auth, §4 network, §5 process isolation, §6 audit, §7/§7.1 secrets, §9 updates, §11 acceptance |
| `docs/testing.md` | §1 pyramid, §3 fixtures, §5 perf methodology, §8 what is not tested |
| `docs/BUILD.md` | Prereqs, JDK 6 not needed to build, env vars, writable-path restriction |
| `docs/RUN.md` | Four forms, first-run downloads, diagnostic center, user state paths |
| `docs/BLOCKERS.md` | B-001…B-007, closure discipline |
| `docs/MILESTONES.md` | Wave 0 gates, backend/frontend/build status, Wave 4 JSP, Wave 3.1 debug, deferred table, later waves |
| `docs/BUNDLED.md` | offline-by-design policy, bundled layout |
| `docs/LIMITATIONS.md`, `docs/RISK_REGISTER.md` | Design/feature/perf/security limits; R-001…R-016 |
| `docs/ROADMAP.md` | Remaining real-environment validation gaps |
| `docs/specs/API_CONTRACT_SPEC.md` | Envelope, endpoint shapes, event types, error codes, no-bare-payload rule |
| `docs/specs/ENCODING_SAFETY_SPEC.md` | GBK/GB18030 byte-safety rules, N-025/N-027 |
| `E:\github\zed\kairo\README.md` | Porting goal and receiving context |

**ADRs read in full — 001–0015 as required:**

| ADR | Title | Role in this report |
|---|---|---|
| 0001 | Theia as the IDE platform | §2.1 platform choice; rejects own-editor, VS Code fork, Zed fork |
| 0002 | Go Runtime Agent split | §2.2; why Go, not Node/Rust/Electron; UI never executes shell |
| 0003 | Protocol versioning and shape ownership | §4.3; single schema, `/api/v1`, envelopes, no gRPC/Protobuf at v1 |
| 0004 | Plugin architecture (three layers) | §3.2/§3.5/§5.2; provider interfaces, Layer 3 deferred by ADR-0014 |
| 0005 | Monorepo structure | §1.4/§3.2; pnpm, small `node_modules`, offline bootstrap |
| 0006 | JDK 6 / Tomcat 6 EOL strategy | §5.1/§5.2/§5.4; three toolchains, fingerprint, no vendoring, fail-closed amendment |
| 0007 | Encoding handling (GBK, UTF-8, BOM, properties) | §5.3; encoding model, detection, save, properties, rejected alternatives |
| 0008 | Remote workspace security model | §3.4/§6.2; per-user workspaces, cgroups rejected, TLS, audit |
| 0009 | JDT LS integration | §2.3/§5.1; adopt JDT LS, LS process supervision, Java 6 caveats |
| 0010 | Project identity and execution planning | §6.1 #7; opaque IDs, path policy, plan resolver, atomic deploy |
| 0011 | Runtime server lifecycle (superseded by 0015) | §5.2; identity, Generation, ProcessIdentity, reconciliation |
| 0012 | Safe API DTO boundary (superseded by 0015) | §6.1 #13; excluded fields, no domain serialization to HTTP |
| 0013 | Deployment Owner Token | §6.1 #7; HMAC capability bound to (ws, proj, srv, root) |
| 0014 | Desktop + localhost v1; Remote deferred | §3.5/§4.1; two forms, two plugin layers, scope rationale |
| 0015 | Lightweight vertical slices | §3.2/§4.2; typed services/plans/repos, composition root, JSON at boundaries |

**Additional ADRs consulted because they bear on legacy constraints / the port:** 0016 (Java 6/Tomcat 6/DAP gate), 0017 (JDT LS compatibility matrix, Java 6 + GBK), 0026 (desktop packaging, desktop/browser fork only at startup). `MILESTONES.md` records 30 ADRs (001–0030); the remaining ones (0018–0025, 0027–0030) concern later waves (git stash/cherry-pick, debug sessions, performance gates, test coverage, supply chain, event hub, ripgrep, `any` elimination, remote Linux agent, Maven, multi-module debug, enterprise compliance) and were outside the requested scope except where cross-referenced above.
