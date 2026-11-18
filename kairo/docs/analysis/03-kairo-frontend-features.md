# 03 — Kairo IDE frontend / Theia extension feature inventory

> **Scope of analysis:** `G:\spaces\kairo-ide` — the Electron/Theia frontend of Kairo IDE, a
> workstation for legacy Java 6 / Tomcat 6 / JSP / GBK web projects.
> **Method:** static inventory of `apps/`, all 22 packages under `packages/`, the command/menu/
> widget/preference registries, `tests/e2e/`, and the self-reported state in `docs/ui-spec.md`
> and `docs/MILESTONES.md`. Honesty notes are cross-checked against later audit reports and,
> where feasible, against current source.
> **Paths below are relative to `G:\spaces\kairo-ide` unless stated otherwise.**
> Doc dates: `docs/MILESTONES.md` is stamped "Last verified 2026-08-22"; `docs/ui-spec.md` is
> the living UI source of truth; later audits exist (`docs/gpt6/…2026-09-21.md`,
> `docs/qa/2026-10-20-parallel-campaign/`) and are used below where they contradict MILESTONES.

---

## 1. Architecture summary

Kairo IDE is a **two-process-plus-agent desktop product** with an optional browser target:

1. **Electron main process** (`apps/desktop/src/main.ts`, ~1600 lines). On launch it:
   - starts the **Go Runtime Agent** (`kairo-runtime.exe`) on `127.0.0.1` with an OS-assigned
     port written to `agent-state.json`, plus a CSPRNG session secret passed only via env /
     preload (`buildAgentEnv`, `startAgent`, `tryReuseAgent`);
   - starts the **Theia backend** as a child node process (`ELECTRON_RUN_AS_NODE=1`,
     `startTheiaBackend`) pointing at `KAIRO_AGENT_URL`;
   - opens a `BrowserWindow` with `contextIsolation`, `nodeIntegration:false`, `sandbox:true`,
     a preload bridge (`apps/desktop/src/preload.ts`), full-work-area sizing, and a strict
     allowlist for `window.open` / `will-navigate` / `will-redirect` / `will-frame-navigate`
     (only the Theia frontend port, never the agent port);
   - runs startup validation (Node ≥ 20.10, platform/arch checks), JDK and Tomcat detection
     dialogs (`jdk-check.ts`, `tomcat-check.ts`), agent auto-respawn with exponential backoff,
     child-process-tree teardown (`child-lifecycle.ts`, `process-manager.ts`), and file logging
     to `artifacts/desktop-main.log` or `<userData>/logs/desktop-main.log`.
   - **Native menus** are built in `buildMenuTemplate()` (~lines 1141–1337) and forwarded to the
     renderer as `menu-action` IPC messages. Native IPC handlers cover JDK switch, Tomcat
     configure, OS keychain encryption of SQL passwords (`safeStorage`, `kairo:safe-storage-*`),
     DevTools toggle, renderer-error forwarding.
   - **Headless mode** (`Kairo-Server.exe` rename or `--headless`) starts agent+Theia without a
     window — the "server mode" the browser app consumes (`docs/milestones` marks a separate
     `apps/server/` as *deferred/removed*).
2. **Theia frontend composition** (`packages/theia-product`). `product.ts`, `product-frontend.ts`,
   `product-bindings.ts`, and `src/main/browser/kairo-product-frontend-module.ts` wire every
   Kairo extension plus stock Theia extensions (`@theia/core, editor, monaco, debug, navigator,
   outline-view, markers, preferences, search-in-workspace, terminal, scm, plugin-ext,
   plugin-ext-vscode, workspace, filesystem, userstorage, variable-resolver, messages, test`).
   `kairo-views-contribution.tsx` is the single largest user-facing contribution: it defines the
   `KairoCommands` namespace, ~35 widget factories (see `kairo-factory-ids.ts`), the native-style
   **Kairo menu** with submenus, and localized React views.
3. **Runtime boundary** (`packages/runtime-extension`) is the only module allowed to call the Go
   agent's `/api/v1/*` HTTP/WS endpoints; every other feature goes through `KairoRuntimeClient`,
   `RuntimeConnectionService`, `WorkspaceContextService`. Wire shapes/error codes live in
   `packages/protocol`; project config JSON Schema in `packages/config-schema`
   (`.legacyflow/project.yaml`).
4. **UI infrastructure** (`packages/ui-kit`: design tokens, two dark Theia themes + Monaco themes,
   reusable React primitives, standard `.kairo-empty-state` / `.kairo-error-banner` CSS) and
   **localization** (`packages/i18n`: `KairoI18nService`, preload module, locale coordinator,
   en.ts / zh-CN.ts catalogs of ~5,500 lines each) are consumed by all feature extensions.

**Product composition note:** `packages/theia-product/package.json` declares dependencies on all
Kairo extensions **except** `remote-extension` and `sql-extension` — those UIs are nonetheless
registered (their factory IDs and commands live in `kairo-factory-ids.ts` /
`kairo-views-contribution.tsx` through `kairo-sql-console-widget.tsx` and `kairo-remote-widget.tsx`),
so treat the dependency list as incomplete rather than authoritative. The binary npm shims
`decompress-compat` and `drivelist-stub` are wired via root `pnpm.overrides` (`package.json`).

---

## 2. Package / extension inventory

| # | Package (`packages/…`) | Kind | Purpose | Concrete user-facing contributions (commands / views / settings / languages) | Key source files |
|---|---|---|---|---|---|
| 1 | `build-extension` | Frontend feature ext. | Build store, build view, custom build runner, Maven view. | Build history widget (Build/Clean Build/Cancel/Show Output, severity diagnostics), `maven-view-widget.tsx` (detect/dependencies/lifecycle/output), custom build runner with stdout/stderr line classes; `kairo.build.maxBuildHistory` setting consumed from store. | `src/browser/build-store.ts`, `build-view-widget.tsx`, `kairo-custom-build-runner.tsx`, `maven-view-widget.tsx`, `build-marker-adapter.ts` |
| 2 | `config-schema` | Library | JSON Schema for `.legacyflow/project.yaml`; Ajv validation shared with Go agent. | No direct UI; powers project import detection and run-configuration forms. | `src/index.ts`, `src/run-configuration.ts` |
| 3 | `decompress-compat` | npm shim (private) | CJS-callable alias over `@xhmikosr/decompress` so Theia 1.73 `.vsix` install works. | Enables plugin installation; no UI. | `index.js`, `index.d.ts` |
| 4 | `drivelist-stub` | npm shim | Pure-JS drive enumeration (Windows A–Z, Unix root), replaces native `drivelist`. | Enables path pickers on bundled builds; no UI. | `src/index.ts` |
| 5 | `encoding-extension` | Frontend feature ext. | GBK / GB18030 / UTF-8-BOM / ISO-8859-1 round-trip bridge to agent encoding endpoints. | Commands `kairo.encoding.reopen`, `.save`, `.show`, `.convert`; encoding tab decorator (suffix), status-bar encoding segment, `KairoFileService`, 8-encoding dropdown, auto-detect + directory-level overrides, EOL conversion, byte-integrity guard. | `src/browser/encoding-commands.ts`, `encoding-service.ts`, `kairo-encoding-registry.ts`, `encoding-tab-decorator.ts`, `kairo-file-service.ts`, `reopen-strategy.ts`, `project-encoding-contribution.ts`, `encoding-cache-contribution.ts` |
| 6 | `git-extension` | Frontend + Node backend ext. (`theiaExtensions.backend`) | Git status/history/blame/diff/stash/cherry-pick via `child_process` git CLI behind Node JSON-RPC. | Commands: Changes (`kairo.git.changes`), Commit (`kairo.git.commit`), Pull/Push/Fetch, Create/Switch Branch, Discard Changes, stash toggle (`kairo-git-stash:toggle`), cherry-pick pick/continue/abort (`kairo-git-cherrypick:*`), history toggle (`kairo-git-history:toggle`). Widgets: changes/commit/diff/history/stash, gutter + file-status + explorer decorators, blame decorator, status-bar branch/cherry-pick entry. Commit templates, commit search, pre-commit checks. | `src/browser/git-*.ts(x)`, `src/node/git-backend-service.ts`, `git-workbench-contribution.ts`, `git-sync-contribution.ts` |
| 7 | `highlighting-extension` | Frontend + Worker ext. | Deterministic JSP region scanning, incremental Worker tokenization, Monaco adapter, diagnostics. | Command `kairo.highlighting.inspect` (diagnostics). Language coverage for Java/JSP/XML/properties/JSON grammar rules; no visible panel. | `src/common/jsp-region-scanner.ts`, `src/common/grammars/{jsp,java}-rules.ts`, `src/worker/*`, `src/browser/monaco-tokenization-adapter.ts`, `highlighting-diagnostics.ts` |
| 8 | `i18n` | Frontend preload ext. | Type-safe translations, lazy language packs, Theia text replacement. | Setting `kairo.language` (`zh-CN`/`en`) with human-readable labels; all Kairo command/menu/widget/status strings localized; Theia core text mirrored to Chinese. | `src/browser/kairo-i18n-service.ts`, `kairo-i18n-preferences.ts`, `kairo-locale-coordinator.ts`, `kairo-theia-text-replacement.ts`, `src/locales/{en,zh-CN}.ts` |
| 9 | `java-extension` | Frontend + Node backend ext. | JDT Language Server lifecycle, Java editing intelligence, Maven, JUnit runner, Java debug helpers. | ~40 commands: completion (`kairo.java.smartCompletion`, live templates add/manage), navigation (`goToTypeDefinition`, `goToSuperMethod`, `findUsages`, `showUsages`, `goToDeclaration`, `goToImplementation`, `peekDefinition`, `peekImplementation`, `navigation.fileStructure`), hierarchy (call/type in/out), index progress (`pause`,`rebuild`), run (`java.run.*`: Run Main/Debug Main/Run Test/Debug Test), breakpoint condition/hit-count/logpoint/evaluate, hot-swap history. Widgets: hierarchy, references, hot-swap, Maven, remote-debug config, index progress UI, CodeLens/semantic/diagnostics providers. Preferences: `kairo.java.formatOnSave`, `.organizeImportsOnSave`, `.tabSize`, `.insertSpaces`, `.hotswap.enabled`. | `src/browser/java-monaco-registration.ts`, `java-navigation-contribution.ts`, `java-hierarchy-contribution.ts`, `java-run-commands.ts`, `java-live-templates.ts`, `java-hotswap-service.ts`, `src/node/jdt-ls-manager.ts` |
| 10 | `jsp-extension` | Frontend feature ext. | JSP/Taglib/EL/XML/properties/JSON language support, virtual Java, encoding-aware save. | Monarch grammars + region scanner; scriptlet Java completion/diagnostics (virtual documents), TLD tag/attribute completion, EL completion/hover/navigation (11 implicit objects, 16 operators), JSP↔Servlet/web.xml navigation, taglib URI completion, XML DTD completion/validation, structure view/outline, find usages, JSP breakpoint CodeLens (experimental, preference `kairo.jsp.debugBreakpoints`), JSP debug toggle. Command `kairo.jsp.toggleBreakpoint`. | `src/browser/jsp-monarch.ts`, `jsp-grammar.ts`, `jsp-scriptlet-*`, `jsp-tld-completion.ts`, `el-expression-provider.ts`, `tld-parser.ts`, `webxml-parser.ts`, `xml-structure-view.ts`, `jsp-sourcemap.ts`, `jsp-debug-breakpoint.ts` |
| 11 | `plugin-extension` | Frontend + Node backend ext. | VS Code `.vsix` compatibility, local install, allowlist curation. | Commands `kairo.extensions.open`, `.installFromVsix`, `.enable`, `.disable`, `.uninstall`, `.reload`; Extensions widget (`kairo-extensions`) with compatibility score, View-menu entry, keybinding. Backend scanner/installer with Zip-Slip protections (historical audit flagged allowlist as weak — see §5). | `src/browser/kairo-extensions-contribution.ts`, `kairo-extensions-widget.tsx`, `src/node/kairo-extension-{installer,scanner,allowlist,service}.ts` |
| 12 | `project-extension` | Frontend feature ext. | Workspace/project model, Welcome view, import wizard, project selector/structure. | Commands `kairo.project.structure`; widgets import wizard (`kairo-import-wizard`), project selector (`kairo-project-selector`), project structure dialog; 4-step import (pick folder → detect layout → choose JDK/runtime → open), GBK-aware detection, active-project service. | `src/browser/import-wizard-widget.tsx`, `project-selector-widget.tsx`, `project-structure-dialog.tsx`, `project-service.ts`, `active-project-service.ts`, `project-structure-contribution.ts` |
| 13 | `protocol` | Library | Wire protocol shapes, error codes, versioned endpoint map shared with Go agent. | No UI; 38 documented endpoints (per `docs/API_REFERENCE.md`). | `src/index.ts`, `src/network.ts`, `src/error-codes.fixture.ts` |
| 14 | `remote-extension` | Frontend feature ext. | P3 Remote agent: TLS 1.3 + WebSocket connection, auth, reconnection, sandbox enforcement. | Remote panel/widget (`kairo-remote`): connection form, status chip, recent connections, connected-info card, file sync & container controls; connection service & sandbox service. | `src/browser/remote-panel-widget.tsx`, `remote-connection-widget.tsx`, `remote-connection-service.ts`, `remote-sandbox-service.ts` |
| 15 | `runtime-extension` | Frontend feature ext. (foundation) | The single HTTP/WS client to `/api/v1/*`; connection lifecycle, security, workspace context, save-failure notifications, resource settings. | Connection-status state consumed by status bar and Remote widget; `kairo.resource.*` settings; no standalone panel. | `src/browser/runtime.ts`, `runtime-connection-service.ts`, `runtime-security.ts`, `workspace-context-service.ts`, `save-failure-notification.ts` |
| 16 | `search-extension` | Frontend feature ext. | Full-text search UI over agent `/api/v1/search`, replace preview, file index. | Search Center (search/replace with scope + file masks + replace preview), Search Everywhere (double-Shift fuzzy: file/class/symbol/action), Find File (`kairo.find.file`), Find Class (`kairo.find.class`), Find Symbol (`kairo.find.symbol`), Find Action (`kairo.find.action`), commands `kairo.search.center.toggle`, `kairo.search.replace`, `kairo.search.results.open`. | `src/browser/search-center-{widget,contribution}.tsx/.ts`, `search-everywhere-*`, `find-file-*`, `find-class-*`, `find-symbol-*`, `find-action-*`, `search-replace-service.ts`, `file-index-service.ts` |
| 17 | `sql-extension` | Frontend feature ext. | Lightweight Oracle 11g SQL tool. | SQL Connection widget, SQL editor, results grid (with CSV/JSON export), credential storage via Electron `safeStorage`; command `kairo.view.sqlConsole`; Kairo SQL Console widget also lives in theia-product. Backend is now a pure-Go Oracle driver (`runtime-agent/internal/sql/oracle_driver.go`), replacing the historical 501 stub. | `src/browser/sql-connection-widget.tsx`, `sql-editor-widget.tsx`, `sql-results-widget.tsx`, `sql-connection-service.ts`, `sql-execution-service.ts` |
| 18 | `svn-extension` | Frontend + Node backend ext. | Out-of-box Subversion support (IDEA-like) via Node JSON-RPC + `svn` CLI. | **18 commands**: `svn.showChanges`, `.showHistory`, `.refresh`, `.update`, `.commit`, `.add`, `.revert`, `.cleanup`, `.checkout`, `.annotate`, `.diff.show`, `.lock`, `.unlock`, `.browseRepo`, `.showInfo`, `.switch`, `.resolve`, `.ignore`, plus `svn.statusBar.click`. Widgets: changes, history, diff, commit/conflict/update/repository/info dialogs, ops dialogs, gutter/file-status/explorer decorators, status bar. **17 preferences** (`svn.enabled`, `svn.path`, decorations, gutter, annotate, commit, update, auth, trustServerCert, …). Keybindings registered. | `src/browser/svn-contribution.ts`, `svn-*-widget.tsx`, `svn-preferences.ts`, `svn-status-bar-contribution.ts`, `src/node/svn-backend-service.ts` |
| 19 | `test-extension` | Frontend feature ext. | JUnit discovery, execution, results view. | Test tree widget (`kairo-test-results`), test markers adapter, runner/discovery/store; command `kairo.view.tests`. | `src/browser/test-tree-widget.tsx`, `test-tree-contribution.ts`, `test-runner.ts`, `test-discovery.ts`, `test-store.ts` |
| 20 | `theia-product` | Application composition | Wires all extensions; owns shell, tool bar, status bar, menus, views, themes integration, debug UI, settings, bookmarks, keymaps, welcome. | `KairoCommands` namespace (~45 commands) + ~35 widget factories; native-style Kairo menu; IDEA keymaps; status bar; 2 dark themes; compliance/notification/keymap/cheatsheet widgets; DAP debug tool window. | `src/main/browser/kairo-product-frontend-module.ts`, `kairo-views-contribution.tsx`, `kairo-status-bar-contribution.ts`, `kairo-toolbar-widget.tsx`, `kairo-factory-ids.ts`, `debug-*.tsx`, `src/main/node/*` |
| 21 | `tomcat-extension` | Frontend feature ext. | Tomcat/Server runtime panel, deployments, logs, hot deploy. | Server view (`kairo-server-view`): Start/Start(Debug)/Stop/Restart/Open Application, state badges; Log viewer (`kairo-log-viewer`): paged logs, level filters, auto-scroll, live/paused chip; Deployments list; `HotDeployService` (500 ms debounce, save/blur/build hooks, 4 modes, honest status). Command `kairo.server.update`. | `src/browser/server-view-widget.tsx`, `server-store.ts`, `server-service.ts`, `log-viewer-widget.tsx`, `hot-deploy-service.ts`, `server-model.ts` |
| 22 | `ui-kit` | Frontend library + Theia ext. | Design tokens, themes, reusable React primitives. | Themes `kairo-dark` and `kairo-idea-dark` (both dark; light is a set of color overrides), Monaco `kairo-dark`/`kairo-idea-dark`; `KairoActionButton`, `VirtualList`, `ResizableSplit`, `FormField`, `ToolWindowFrame`, `PathField`, shared CSS. IDEA editor preference overrides. | `src/tokens.ts`, `src/browser/kairo-theme.ts`, `kairo-theme-idea.ts`, `kairo-theme-contribution.ts`, `kairo-theme.css`, `virtual-list.tsx`, `resizable-split.tsx` |

**Apps:** `apps/desktop` = Electron shell (above); `apps/browser` = Theia browser target
(`package.json` binds `127.0.0.1`, default locale `en`, `security.workspace.trust.enabled`,
`files.autoSave:off`, watch/search excludes for `.git/.svn/WEB-INF/lib/*.class/target/build/…`;
`esbuild.mjs`, `postbuild.cjs`, `scripts/start.cjs`). Browser/desktop parity is mandated by
`docs/ui-spec.md` §18.

---

## 3. Feature areas — concrete commands and UI

### 3.1 Shell, menus, tool bar, activity bar, command palette

- **Top-level menus** (`apps/desktop/src/main.ts`: File, Edit, Selection, View, Go, Terminal,
  Kairo, Help; in-browser the same command IDs are injected into Theia's menu bar by
  `kairo-views-contribution.tsx`, KAIRO_MENU path `['main-menu','5_kairo']`). `docs/ui-spec.md`
  §15 lists `File · Edit · View · Selection · Go · Run · Terminal · Window · Help` as the
  canonical order — the shipped native menu uses a slightly different order (note drift).
- **Kairo menu submenus**: Import/Select/Scan project, Run Configurations, `Build & Run`
  (Build, Clean Build, Build & Deploy, Publish, Start Server, Start Server (Debug), Stop Server,
  Restart Server, Open Application, Check Java Debug Adapter), `View` (Servers, Builds,
  Deployments, Tomcat Logs, Maven, TODO/FIXME, Test Results, SQL Console, Remote Development,
  Performance Dashboard), `Debug` (Open Debug View/Console, Variables, Call Stack, Breakpoints,
  Watch, Debug Toolbar, Debug Diagnostics), `Window` (Toggle Terminal, Keyboard Shortcuts,
  Switch JDK, Configure Tomcat, Reconnect Runtime Agent).
- **Tool bar** (`kairo-toolbar-widget.tsx`): project selector, run-configuration selector,
  Run / Debug / Stop / Build buttons with busy state and real chord hints computed by
  `kairo-platform.ts:toolbarShortcut()` (Win: Shift+F10 / Shift+F9 / Ctrl+F2 / Ctrl+F9;
  mac: Ctrl+Shift+R / Ctrl+Shift+D / Cmd+F2 / Cmd+F9).
- **Activity bar** (ui-spec §1.1): Explorer (Ctrl+Shift+E), Search (Ctrl+Shift+F), Java
  (Ctrl+Shift+J), Servers, Run & Debug, Extensions, with hover/active/focus styling in
  `ui-kit/src/browser/kairo-theme.css`.
- **Command palette** (F1 / Ctrl+Shift+P) and **Search Everywhere** (double-Shift) both expose
  every Kairo command; i18n requires no mixed-language labels (ui-spec §20.9).

### 3.2 Project model, import wizard, welcome

- Commands: `kairo.project.import`, `kairo.project.select`, `kairo.project.scan`,
  `kairo.project.structure` (file → structure dialog, keybinding).
- Widgets: `kairo-import-wizard`, `kairo-project-selector`, Welcome (`kairo-welcome`) with three
  GET STARTED actions — **Import Project** (`kairo.project.import`), **Run Configurations**
  (`kairo.runConfigurations.manage`), **Build & Deploy** (`kairo.buildAndDeploy`), plus "Pick a
  folder / Open Folder" (`workspace:openFolder`), recent projects, 3-card quick-start guide,
  dismissible agent-offline banner.
- Import flow (e2e `ch09-import.spec.ts`): Step 1 path/folder picker → scan (`detectProject`) →
  Step 2 review of detected name/sourceDirs/encoding/JDK/server with warnings & confidence badge
  → Step 3 `openWorkspace` + `import` + activate project. Handles Chinese/space paths, duplicate
  roots, forbidden paths, offline agent, GBK fallback, empty-project validation.
- Run configurations: `kairo.runConfigurations.manage`, `kairo.runConfiguration.run`,
  `kairo.runConfiguration.debug`; service auto-derives HTTP/debug ports from project config and
  settings, persists selections.

### 3.3 Java language intelligence (`java-extension`, JDT LS)

- JDT LS distribution managed by `src/node/jdt-ls-manager.ts` / `jdt-ls-service.ts`, with
  SHA verification; Theia backend LS + browser `JavaLanguageClient`; document sync;
  index progress UI (pause/rebuild) and status-bar entry.
- Completion: basic, smart (`Ctrl+Shift+Space`), Hippie (`Alt+/`), context/live templates
  (150+ in 15 categories), recent completions, completion resolve/UX.
- Navigation/refactoring: definition, implementation, type definition, super method, usages,
  file structure, call/type hierarchy, organize imports, surround-with, unwrap, override gutter,
  safe delete, generate/change signature/extract refactorings, diagnostics manager with
  fallback diagnostics.
- CodeLens: run/debug Main, run/debug Test, references/implementations/usages. JUnit discovery,
  runner, results (also `test-extension`). Maven view (dependency tree, lifecycles, goals, mvnw).
- Preferences: `kairo.java.formatOnSave`, `.organizeImportsOnSave`, `.tabSize`, `.insertSpaces`,
  `.hotswap.enabled` (HotSwap history widget command `kairo.java.hotswap.showHistory`).

### 3.4 JSP / XML / EL / properties language support (`jsp-extension` + `highlighting-extension`)

- **JSP syntax highlighting**: Monarch grammar (`jsp-monarch.ts`, `jsp-grammar.ts`) plus
  `highlighting-extension`'s deterministic `jsp-region-scanner.ts` and Java rules; incremental
  Worker tokenization (`src/worker/incremental-tokenizer.ts`, `highlighting-worker.ts`) with
  Monaco tokenization adapter and token cache. Explicitly covers directives, scriptlets
  (`<% %>`, `<%= %>`, `<%! %>`, `<%@ %>`), EL, JSTL, custom tags, attribute-embedded EL,
  scriptlet background decoration; XML/properties/JSON Monarch grammars and DTD validation.
- Completion/navigation: scriptlet Java completion via virtual documents (`jsp-virtual-java.ts`,
  `virtual-document-manager.ts`, sourcemap), diagnostics with coordinate remapping (500 ms
  debounce), EL completion (implicit objects, operators, bean properties) + hover + Ctrl+Click
  navigation, TLD taglib/attribute completion from workspace `.tld` and 5 built-in JSTL,
  JSP↔Servlet and web.xml↔Java/JSP bidirectional navigation, Java→web.xml reverse lookup, XML
  DTD completion, structure/outline view, find-usages (import/useBean/scriptlet/web.xml).
- **Experimental**: JSP breakpoints (`kairo.jsp.toggleBreakpoint`, CodeLens Jasper mapping) gated
  by preference `kairo.jsp.debugBreakpoints`.

### 3.5 Encoding — GBK round-trip (`encoding-extension`)

- Commands: `kairo.encoding.reopen`, `kairo.encoding.save`, `kairo.encoding.show`,
  `kairo.encoding.convert`.
- Eight encodings in the dropdown (preference `kairo.encoding.defaultProjectEncoding`):
  `utf-8`, `utf-8-bom`, `utf-16le`, `utf-16be`, `gbk`, `gb18030`, `iso-8859-1`, `us-ascii`.
- Auto-detection (`kairo.encoding.autoDetect`), three-level override precedence
  (per-file > deepest folder > extension), directory-level overrides, project-level encoding
  from detection, EOL normalization (CRLF/LF), byte-integrity round-trip guard, unrepresentable
  character refusal (no silent `?`), dirty-file guard on Reopen, Convert via confirmation dialog,
  tab suffix indicator and status-bar encoding segment, GBK full-text search through agent.
- Agent-side encoding endpoints used via `runtime-extension` (`recode`, `probe`, etc.).

### 3.6 Build / compile (`build-extension`, `theia-product`, `java-extension`)

- Commands: `kairo.build`, `kairo.cleanBuild`, `kairo.buildAndDeploy`, `kairo.publish`,
  `kairo.server.update`, `kairo.server.reloadContext`, `kairo.view.customBuild`.
- Build view (`kairo-build-view`): Build (primary), Clean Build, Cancel, Show Output; history
  rows with running/succeeded/failed/cancelled/pending badges; diagnostics list with severity
  styling; stdout/stderr line coloring. Backed by `BuildStore` snapshot/event wiring and agent
  build endpoints; `kairo.build.maxBuildHistory` preference.
- Ant/javac providers on the Go side (incremental javac, always explicit `-encoding` after the
  2026-10 fix); build-marker adapter maps compiler errors to editor problems; build-failure
  workflow (E2E-04) navigates to the offending line.
- Maven view (`kairo-maven-view`, shared code in `java-extension` and `theia-product`): detect,
  dependency tree, lifecycle goals, output/error panels; Maven wrapper fallback.
- Custom build runner (`kairo-custom-build-runner.tsx`) replaces an older deferred placeholder.

### 3.7 Tomcat / server lifecycle / deploy / hot reload (`tomcat-extension`)

- Commands: `kairo.server.start`, `kairo.server.debug`, `kairo.server.stop`,
  `kairo.server.restart`, `kairo.server.update`, `kairo.server.reloadContext`, `kairo.app.open`,
  `kairo.debug.checkAdapter`, `kairo.view.{servers,deployments,logs,customBuild}`.
- **Server view** (`kairo-server-view`): tree of runtimes/deployments/ports, Start (primary),
  Start (Debug), Stop, Restart, Open, state badges (running/starting/stopping/stopped/error/
  crashed), hot-reload banner/last-reload time, `Ctrl+F10` update, `Shift+F10` start.
- **Deployments view** (`kairo-deployments`): timestamp/target/state pill/trigger/reload-mode,
  Deploy (primary) + Refresh, overflow details, standard empty state ("No deployments yet" →
  Build & Deploy).
- **HotDeployService** (`hot-deploy-service.ts`, 500 ms debounce; onSave/onBlur/onBuildCompleted):
  four modes — `staticSync` (JSP/CSS served straight from docBase, no copy), `compileOnly`
  (incremental javac + class sync; Tomcat `reloadable=false` means classes are **not** picked up,
  so status is honestly `restart_required`), `classHotSwap` (JDWP redefine; falls back to
  `restart_required` when no debugger attached), `contextReload` (calls
  `POST /servers/{id}/reload`; status `synced`). A test specifically guards against falsely
  reporting "synced" (`hot-deploy-status-honesty.test.cjs`).
- **Log viewer** (`kairo-log-viewer`): paged log lines (never infinite scroll), stream badges,
  level colors, live/paused indicator, auto-scroll checkbox, runtime/server status chips,
  polling status, load-more, header+toolbar+content chrome, empty/error states. Settings:
  `kairo.server.logMaxLines`.
- Server preferences: `kairo.server.autoStart`, `.autoStop`, `.defaultHttpPort`, `.defaultDebugPort`.

### 3.8 Search (`search-extension`)

- Search Center (`kairo-search-center`): workspace full-text via agent `/api/v1/search` with
  scope model, file-mask filters, results widget with preview, two-phase replace with
  fingerprint/mtime validation and undo (`SearchReplaceService`), 10k default result cap.
- Search Everywhere (double-Shift): concurrent fuzzy providers for files / classes / symbols /
  actions (partial-failure degradation), recent queries, path-safety checks.
- Find File (`Ctrl+Shift+N` / `Cmd+Shift+O` — **not** Ctrl+P, which is parameter info in IDEA
  keymap), Find Class (`Ctrl+N`), Find Symbol, Find Action (`Ctrl+Shift+A`), Go to Line (`Ctrl+G`).
- Preferences: `kairo.search.excludePatterns`, `kairo.search.maxResults`; browser app also sets
  `search.exclude`/`files.exclude` defaults and a `registerOverride('files.exclude', …)` so
  `.kairo/` local-history snapshots don't shadow real files.

### 3.9 Git (`git-extension`)

Real `git` CLI via Node JSON-RPC backend (`src/node/git-backend-service.ts`). Statuses, diff,
history, blame, stash (list/push/pop/apply/drop/show/clear with search/filter widget), cherry-pick
(single/batch/continue/abort; per-commit button in history; status-bar indicator), commit widget
with 7 built-in templates and recent-message suggestions, commit search, pre-commit checks,
branch create/switch, pull/push/fetch/discard, gutter + explorer + file-status decorators,
context-menu actions, status-bar branch entry. Commands listed in §2 row 6.

### 3.10 SVN (`svn-extension`)

Real `svn` CLI via Node JSON-RPC backend; 18 commands (§2 row 18), changes/history/diff views,
commit/conflict/update/repository/info/annotate dialogs, gutter decorations, explorer decorators,
status bar with changes/conflicts/root/revision and a 10-item QuickPick of quick actions.
17 preferences including path, decorations, gutter, annotate, commit options, update on open,
ignore whitespace, trust server cert, auth cache. Keybindings registered. Historical audits note
English-locale parsing assumptions and Windows `file://` handling issues (see §5).

### 3.11 SQL (`sql-extension` + `theia-product`)

Connection panel (host/port/SID/service-name/user/password, keychain-backed credential storage
through Electron `safeStorage`), SQL editor with connection-id binding, results grid with
sticky header/CSV+JSON export/history dropdown, status chips. Commands/views:
`kairo.view.sqlConsole`, `kairo-sql-console`, `kairo-sql-connection`, `kairo-sql-editor`,
`kairo-sql-results`. Backend endpoints `POST /api/v1/sql/test-connection` (returns
`connectionId`) and `/api/v1/sql/execute` (columns/rows/rowCount/executionTimeMs/truncated,
paging via `maxRows`/`offset`, named params) implemented with a **pure-Go Oracle driver** —
the old hardcoded 501 stub documented in 2026-09 audits has been replaced in current source
(`runtime-agent/internal/sql/oracle_driver.go`, `oracle_context_connector.go`).

### 3.12 Testing / JUnit (`test-extension` + `java-extension`)

Test tree (`kairo-test-results`) with discovery/run/rerun/filter, JUnit XML parsing (including
edge cases), marker adapter, status badges/counts, empty/error states; `kairo.view.tests` command;
Java CodeLens Run/Debug Test actions; `kairo.junit-test-controller` integrates with Theia Test
view.

### 3.13 Debug (IDEA-style tool window, `theia-product` + Java debug + Go JDWP)

- Commands (selected): `kairo.debug.openView`, `.openConsole`, `.openToolWindow`,
  `kairo.debug.view.{variables,callstack,breakpoints,toolbar,console,watch}`,
  `kairo:open-debug-diagnostics`, `kairo.debug.checkAdapter`, `kairo.debug.restart` (Rerun),
  `.dropFrame`, `.toggleInlineValues`, `.muteBreakpoints`, `.evaluateExpression`,
  `.console.focus`, plus IDEA keybindings F8/F7/Shift+F8/F9/Alt+F9, Ctrl+F8/Shift+F8.
- Widgets/factories: `kairo-debug-tool-window` (Debugger/Console tabs; Frames+Breakpoints left,
  Variables+Watches right; empty states; status bar), `kairo-debug-variables`, `kairo-debug-callstack`,
  `kairo-debug-breakpoints` (conditional/hit-count/log message), `kairo-debug-watch`,
  `kairo-debug-console` (DAP REPL with history/completion), `kairo-debug-toolbar`,
  `kairo-debug-condition-editor`, `kairo-debug-diagnostics`, `kairo-debug-hover-provider`,
  `kairo-debug-inline-values`. `src/main/node/kairo-java-debug-adapter-contribution.ts` locates
  the bundled `kairo-jdi-bridge.jar` (KAIRO_BUNDLED_DIR); status bar shows adapter availability.
- Backend: JDWP variable/stackframe/breakpoint/step orchestration, multi-VM orchestrator,
  deferred cross-module breakpoints, module dependency ordering (per MILESTONES, Wave 13).

### 3.14 Plugin system (`plugin-extension`)

Theia `plugin-ext` + `plugin-ext-vscode` provide VS Code extension host compatibility; Kairo adds
local `.vsix` install (`kairo.extensions.installFromVsix`), enable/disable/uninstall/reload,
allowlist-based curation and compatibility scoring in the Extensions widget. `decompress-compat`
shim keeps Theia 1.73 installation working. Security caveat in §5.

### 3.15 Status bar (`theia-product/src/main/browser/kairo-status-bar-contribution.ts`)

Left-to-right logical groups (ui-spec §1.3, §20.5): project placeholder/name, JDK version/state,
encoding + EOL, build state, server state with port, debug state, agent connection state,
hot-reload status (hidden with no active project), notification bell, large-file indicator,
format-on-save indicator, plus Git/SVN branch entries from their own contributions. Each segment
is a click target (encoding → Save with encoding; server → Servers view; connected → connection
panel; bell → notification center). Placeholder states are dimmed via
`.kairo-statusbar-placeholder`; severe states keep full opacity + glyph; semantic state classes
(`.kairo-statusbar-state-success/active/warning/error/neutral`); all strings localized.

### 3.16 Themes (`ui-kit`)

Two registered Theia themes: **`kairo-dark`** (`KairoDarkTheme`, VS-Code-ish) and
**`kairo-idea-dark`** (`KairoIDEATheme`, IntelliJ-style; set as default at startup), with matching
Monaco themes. Light mode is delivered through Theia's built-in light theme plus
`KAIRO_THEME_COLOR_OVERRIDES` (ui-spec §2.4 defines full light tokens). Theme persists per user;
`Ctrl+K Ctrl+T` switches. Dark accent `#4a9eff`; IDEA surface consistency rules in ui-spec §20.

### 3.17 i18n / localization (`i18n`)

`KairoI18nService.t(key, params)` is the source of truth; en.ts / zh-CN.ts catalogs (~5,500 lines
each) are checked for key parity by `scripts/i18n/check-catalogs.cjs` (`pnpm i18n:check`).
The preload module initializes the locale before Theia boots; `KairoLocaleCoordinator` mirrors
VS Code display language and Kairo language, requires reload, and re-renders React widgets via
`onDidChangeLanguage`. Native Electron menus are localized by `localizeMenuTemplate()` in
`main.ts`; all command labels, menus, status bar, settings descriptions, dialogs, empty states,
and accessibility labels follow the selected language. Audits confirm high catalog quality but
historically flagged call sites bypassing i18n (`as never` keys) — see §5.

### 3.18 Other product views (`theia-product`)

Welcome, Problems (localized replacement for stock Theia Problems), Shortcuts/Keymap widget and
shortcut cheat-sheet (`Ctrl+Shift+K`), Outline, TODO/FIXME (`kairo-todo`), Local History
(show/restore/compare), Bookmarks (toggle/list/show/goto/set 0–9/back-to-explorer), Notification
Center, Paste History (`Ctrl+Shift+V`), Performance Dashboard (`kairo.perf.toggleGraph`,
`kairo.perf.openDashboard`), Compliance panel (RBAC/audit/retention/SSO), Remote Development,
secondary-window support (`kairo.editor.moveIntoNewWindow`), focus-management commands
(`kairo.focus.editor/sidebar/bottomPanel/statusBar/terminal`, `kairo.focus.nextPanel/prevPanel`,
`kairo.hideActivePanel`), large-file commands (`kairo.largeFile.toggleFullFeatures`,
`.togglePreview`), editor commands (`kairo.organizeImports`, `kairo.toggleFormatOnSave`,
`kairo.toggleOrganizeImportsOnSave`).

---

## 4. User workflows verified by `tests/e2e/`

The E2E suites are the strongest evidence of *shipped* behavior:

| Suite / file | Workflows exercised |
|---|---|
| `tests/e2e/core-e2e.spec.ts` (E2E-01…10) | First launch → import → correct GBK tree; Java intelligence (completion/definition/references/rename); search → preview → replace → undo; build failure → problems → navigate → fix → rebuild; start Tomcat → JSP edit → immediate effect; Java modify → build → publish → recovery; debug breakpoint → hit → inspect → step → continue → stop; close/reopen recovery; port occupation diagnostics; agent/JDT LS crash recovery. |
| `tests/e2e/frontend-e2e.spec.ts` (FE-01…20) | Shell, import wizard, Java/JSP editing, encoding detection, build, deploy, server start/stop, log viewer, debug + breakpoints, Git ops, JSP scriptlet/EL/TLD, multi-file editing, command palette, search center, problems view, shortcuts, status bar, JSP hot reload. |
| `tests/e2e/comprehensive/ch*.spec.ts` | ch03 launch/shutdown, ch04 window/theme, ch05 menus (every File/Edit/View/Go/Terminal/Kairo/Help entry, context menus), ch06 tool bar, ch07 status bar, ch08 welcome, ch09 import, ch10–12 project/navigator/editor, ch13 Java, ch14 JSP (21 cases), ch15 XML, ch16 JSON/properties, ch17 encoding (16 cases), ch18 search / search-everywhere, ch19 build, ch20 Maven, ch21 Tomcat, ch22 run configurations, ch23 debug, ch24 Wave-4 views, ch26 tests, ch29–30 VCS, ch33–48 remaining, ch37 command palette. |
| `tests/e2e/windows-e2e.spec.ts` (WIN-01…12) | Path separators, file locks, long paths, GBK/UTF-8 switching, case-insensitive FS, process management, perf, temp dir, CRLF, reserved names, UNC paths, env vars in paths. |
| `tests/e2e/boundary-e2e.spec.ts` | 10 MB files, 10+ simultaneous editors, rapid operations, disconnect/reconnect, process crash, low disk, empty project, special filenames, encoding edges, concurrent API. |
| `tests/e2e/desktop/desktop-electron.spec.ts` | Real Electron workbench, BrowserWindow security boundary, preload API, native menu command IDs. |
| `tests/e2e/regression/shard-*`, `standalone-smoke`, `visual-gate`, `a11y-scan` | Shell/project/Java/multilang/build/debug/search/UI/context-menu/dialog/perf regression shards; axe accessibility; visual snapshots. |

---

## 5. REAL vs PARTIAL / FAKE / DEFERRED

Status is judged against current source where possible; "self-reported" means the claim comes from
`docs/MILESTONES.md` / `docs/ui-spec.md` and has not been independently re-verified here.

| Feature area | Current status | Evidence / caveat |
|---|---|---|
| Electron desktop shell, agent lifecycle, native menus, IPC, keychain credential storage, headless mode | **REAL** | `apps/desktop/src/main.ts`, `preload.ts`, `process-manager.ts`; E2E desktop suite. |
| Browser target (127.0.0.1, production bundle) | **REAL** | `apps/browser/package.json`, `esbuild.mjs`, `postbuild.cjs`; browser E2E. |
| Separate `apps/server/` product | **DEFERRED / REMOVED** | MILESTONES Wave-0 gate: "Server mode — deferred — `apps/server/` does not exist"; later architecture doc says placeholder was deleted. Headless desktop mode is the substitute. |
| Workspace/project import wizard, project selector, structure dialog | **REAL** | `project-extension` + ch09 / FE-02 E2E (happy path, GBK, paths, offline, duplicates). |
| GBK/GB18030/UTF-8-BOM encoding switching, detect, convert, byte integrity | **REAL** | 16 ch17 E2E cases + `encoding-*` unit tests + agent recode endpoints. |
| Build (Ant/javac), build view, Maven view, problems navigation | **REAL** | build-extension E2E ch19 / E2E-04; agent providers; 2026-10 fix makes `-encoding` explicit. |
| Tomcat start/stop/debug/restart, deploy, publish, Open Application | **REAL (environment-dependent)** | ch21 / ch05 menu E2E: several Tomcat cases are explicitly skipped in lane env ("Tomcat unavailable"). Feature wired; real Tomcat needed to prove end-to-end. |
| Static (JSP/CSS) hot sync | **REAL** | direct-docBase watcher; status `synced` only for staticSync (`hot-deploy-status-honesty.test.cjs`). |
| Compile-only / class-sync hot reload | **REAL BUT HONESTLY LIMITED** | classes are copied but Tomcat 6 `reloadable=false` won't load them; UI reports `restart_required`, never falsely `synced`. |
| Class HotSwap (JDWP redefine) | **PARTIAL / DEFERRED in docs** | `classHotSwap` mode exists in code but falls back to `restart_required` when no debugger attached; MILESTONES "Deferred" table lists "Class HotSwap — JDWP agent integration". |
| Context reload (`/servers/{id}/reload`) | **REAL** | ch16 Wave-16 gate; test asserts reload called once and status `synced`. |
| Log viewer | **REAL** | `log-viewer-widget.tsx`, E2E FE-09; paged (anti-pattern §7 forbids infinite scroll), not a mock. |
| Full-text search + replace + undo, Search Everywhere, find file/class/symbol/action | **REAL** | search-extension + ch18 / E2E-03; two-phase replace with validation. |
| Java completion/navigation/hierarchy/semantic tokens/live templates/Maven | **REAL** | java-extension E2E ch13, E2E-02; JDT LS with SHA-verified distribution. |
| Java debug (DAP + JDWP), IDEA debug tool window, breakpoints/variables/watch/console | **REAL (with caveat)** | Wave 9/13/16 + ch23 / E2E-07; **MILESTONES "Deferred" table still says "DAP/JDWP Debug — endpoint exists, no breakpoint proof"** — internal contradiction; later E2E makes the "REAL" side stronger, but doc was never reconciled. Two parallel debug UIs (old widgets vs IDEA tool window) were flagged as duplicated/unmaintained in the 2026-08 audit. |
| JSP highlighting, scriptlet completion, EL, TLD, JSP↔Servlet navigation, structure view | **REAL** | jsp-extension + highlighting-extension + 21 ch14 cases + 45 JSP tests. |
| JSP breakpoints (experimental CodeLens / Jasper mapping) | **PARTIAL / EXPERIMENTAL** | gated by `kairo.jsp.debugBreakpoints` preference; tc TC-JSP-021. |
| Git (status/history/blame/stash/cherry-pick/etc.) | **REAL** | git-extension Node backend + git CLI; 81 tests; ch29–30 / FE-12. Historical audit flags: commit "signoff/no-verify" dead UI, English-locale parsing assumptions. |
| SVN (18 commands, decorations, dialogs, status bar) | **REAL (platform caveats)** | Node backend + `svn` CLI; 11/11 integration after `withPathArgs` fix. Audit flags English-locale parsing and Windows `file://` decorator failures. |
| SQL Console / Oracle 11g | **REAL NOW (was FAKE)** | Old audit (`FULL_CODE_AUDIT_2026-08-04`, `gpt6` 2026-09-21) documented a hardcoded placeholder / unconditional 501 stub and discarded `connectionId`; current `sql_handler.go` + `internal/sql/oracle_driver.go` implement a pure-Go driver with connection store. Needs a real Oracle instance for true acceptance (no live DB in repo). |
| Plugin/.vsix install + allowlist | **REAL WITH SECURITY CAVEAT** | installer/scanner exist; 2026-08 audit rated "allowlist 装饰品" (decorative), Zip-Slip + no-signature chain flagged; check current scanner for fixes before trusting. |
| Remote Linux agent / panel | **PARTIAL / DEFERRED in docs** | Wave 11 marks remote panel "verified" (16 tests), but MILESTONES "Deferred" table lists "Remote Linux Server — multi-user, auth, audit, container isolation". Treat UI as built, full remote product as deferred. |
| Enterprise compliance panel (RBAC/audit/retention/SSO) | **REAL (frontend) / backend per MILESTONES** | Wave 14 + widget; audit log endpoint `/api/v1/audit` listed as **deferred** in MILESTONES Deferred table — verify which pieces actually serve data. |
| Performance dashboard / TODO / bookmarks / local history / notifications / keymap & cheat-sheet | **REAL** | factory IDs + E2E ch07/ch12/ch05; many i18n/CSS polish sessions in MILESTONES. |
| i18n en/zh-CN catalogs | **REAL** | 2 catalogs ~5,500 lines; key-parity gate; documented as highest-quality area. |
| Themes (dark ×2, light via overrides) | **REAL (dark) / PARTIAL (light)** | Only `kairo-dark` + `kairo-idea-dark` Theia themes registered; ui-spec promises two themes with light tokens, but no `kairo-light` Theia theme exists. |
| IDEA keymaps | **REAL with historical fake hints fixed** | win/mac keymaps real; 2026-10 campaign FIX-06 corrected four toolbar shortcut tooltips that advertised unbound `Cmd+R/D/./B`. `kairo-platform.ts` now computes real chords. |
| Diagnostics package export, idle-session expiry (30 min), cookie sessions | **DEFERRED / NOT IMPLEMENTED** | `docs/qa/2026-10-20-parallel-campaign/README.md` line 524 explicitly lists these as "未实现". |
| "No AI surface in v1" | **INTENTIONAL ABSENCE** | ui-spec §7; no AI panel exists. |

### Documentation drift / honesty notes

- `docs/MILESTONES.md` is stamped **2026-08-22** and contains internal contradictions: its wave
  tables mark Debug, Remote and Compliance "verified" while its *Deferred (post-v1)* table lists
  the same items as deferred. Prefer code + later E2E evidence.
- Several widgets cited in MILESTONES **no longer exist** in the tree:
  `debug-module-selector-widget.tsx`, `debug-hotswap-status-widget.tsx`,
  `debug-multimodule-widget.tsx`, `kairo-telemetry-settings.tsx` (all `MISSING`). Do not treat
  those doc rows as current features.
- ui-spec §16.1/§16.4 reference "§13.2 / §13.3" for server/log-viewer specs, but §13 is
  Internationalization — stale cross-references.
- ui-spec §15.1 menu order differs from the shipped native menu (`main.ts`); ui-spec §2 promises
  two themes, code registers two dark ones plus light color overrides.
- The 2026-07/08 audits (`AUDIT-2026-07-19.md`, `FULL_CODE_AUDIT_2026-08-04.md`,
  `REARCHITECTURE_AND_IMPLEMENTATION_PLAN.md`) describe an earlier, much rougher state
  (placeholder HTML server, SQL 501 stub, no EventBus, fake auth, i18n call-site gaps). Many items
  were subsequently fixed (SQL driver, headless mode, native menus, hot-reload honesty). They are
  valuable as a list of *failure modes to re-check*, not as the current inventory.
- `docs/gpt6/Kairo_IDE_Deep_Test_Report_2026-09-21.md` warns that some historical tests asserted
  mocks/stubs rather than real behavior (e.g., "501 stub 识别不等于 SQL 可用", security tests
  against local mock servers). The 2026-10 campaign added a real-binary agent harness and fixed
  several of these, but "tests pass" should not be read as "feature proven" without checking
  whether the test drives the real agent/Tomcat/Oracle.

---

## 6. Practical takeaway for a port/integration analysis

If the goal is to transplant these features into another editor (e.g. Zed), the load-bearing
pieces are: (1) the Go agent protocol (`packages/protocol` + `runtime-agent/internal/api`),
(2) the build/deploy/hot-reload state machines, (3) the encoding byte-integrity layer, and
(4) the JDT-LS + JSP virtual-document language stack. The pure-UI layers (tool bar, status bar,
themes, menus, i18n, React widgets) are numerous and polished but replaceable; the SQL/Remote/
Compliance surfaces are the ones most likely to be broader in docs than in proven behavior.
