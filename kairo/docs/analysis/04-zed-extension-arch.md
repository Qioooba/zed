# 04 — Zed extension & external-tooling architecture

> Scope: `E:\github\zed` (Zed, Rust workspace, ~255 crates).
> Purpose: precise technical map of everything needed to add legacy Java / JSP /
> Tomcat (JDK 1.6) support, with an eye to keeping the diff against upstream
> small.  Complements `05-zed-core-layout.md` (which covers crate registration,
> panel insertion, and process spawning at the *core* level); this document
> focuses on the **WASM extension API**, **languages**, **LSP**, **DAP**,
> **tasks**, **actions/keybindings**, **settings**, **panels**, and the boundary
> between "can be done in an extension" and "needs Rust core".
>
> All paths are relative to `E:\github\zed`. Line numbers are from the checked-out
> tree and are given only as anchors — re-check them after a rebase.

---

## 0. Executive summary

This checkout is a *modern* Zed: there is **no `assets/languages/` directory**
anymore. Built-in languages live as Rust modules in `crates/languages/src/*.rs`
and their grammars/queries in `crates/grammars/src/<name>/`. Everything else is a
**WASM extension** under `extensions/<id>/` (`extension.toml` manifest +
`languages/<name>/config.toml` + `.scm` queries + optional Rust `src/` compiled to
`wasm32-wasip2` (`RUST_TARGET` in `crates/extension/src/extension_builder.rs:27`,
WASI preview2 + component model). The in-tree extension dirs are only `glsl`, `html`, `proto`,
`test-extension`, and `workflows` (a CI workflow folder, not a Zed extension).

**Critical finding for Kairo:** there is **no Java or JSP grammar, language, LSP
adapter, or DAP adapter anywhere in this checkout.** All four must be added.
The cheapest path is a **new Kairo-owned extension** for syntax/JSP grammar + JDT
LS / JSP LS registration + a JDWP debug adapter, plus **two small Rust crates**
(`kairo_core`, `kairo_ui`) for the Go-agent HTTP client, tasks, actions,
keybindings, settings, and the control panel. The extension contributes *zero*
lines to upstream files; the two Rust crates need the ~5 registration lines
already catalogued in `05-zed-core-layout.md` §7.2.

| Area | Extension-only? | Needs Rust core? | Kairo recommendation |
|---|---|---|---|
| Java/JSP syntax highlighting & grammar | **Yes** (`grammars` + `languages`) | no | Extension |
| JDT LS / Eclipse JSP LS launch & settings | **Yes** (`language_servers` + `language_server_command`) | optional | Extension |
| JDWP debugging attach | **Yes** (`debug_adapters` + `get_dap_binary`) | optional | Extension |
| Slash commands (`/compile`, `/deploy`) | **Yes** (`slash_commands`) | no | Optional; prefer keybound tasks |
| Context servers (MCP) | **Yes** (`context_servers`) | no | Not needed |
| Compile/deploy/start/stop tasks | no (extensions can't register static tasks) | **Yes** (`task` + `workspace`) | Rust: `TaskTemplate` + `task::Spawn::ByName` keybindings |
| Custom actions & keybindings | no | **Yes** (`gpui::Action`, `cx.bind_keys`) | Rust |
| Settings schema/defaults | no (only *read* via `get-settings`) | **Yes** (`Settings` trait) | Rust |
| Custom panel / worktree view | no | **Yes** (`workspace::Panel`) | Rust (`kairo_ui`) |
| Spawning javac/ant/catalina/Go agent | restricted (`zed::Command::output`, capability-gated) | **Yes** (`util::command`, `util::process`) | Rust for long-lived processes; extension only for short probes |

---

## 1. WASM extension API

### 1.1 Crates and files

| Crate | Path | Role |
|---|---|---|
| `extension_api` | `crates/extension_api/` | The API *extension authors compile against* (`zed_extension_api`). WIT definitions under `wit/since_v0.8.0/`. |
| `extension` | `crates/extension/` | Core-side *manifest model* and host-side `Extension` trait. |
| `extension_host` | `crates/extension_host/` | Loads WASM, grants capabilities, wires extension contributions into core registries. |
| `language_extension` | `crates/language_extension/` | Bridges extension LSP contributions to `LanguageRegistry` (`ExtensionLspAdapter`). |
| `debug_adapter_extension` | `crates/debug_adapter_extension/` | Bridges extension DAP contributions to `DapRegistry` (`ExtensionDapAdapter`). |
| `theme_extension` | `crates/theme_extension/` | Bridges extension themes. |
| `extension_cli` | `crates/extension_cli/src/main.rs` | `zed-extension` build/validate tool (`--source-dir --output-dir --scratch-dir`). |
| `extensions_ui`, `extension_store` | — | UI/install side (not needed for authoring). |

Key files:

- `crates/extension_api/src/extension_api.rs` — the `zed_extension_api::Extension` **guest** trait (lines 68–284) and `register_extension!` macro (289–334).
- `crates/extension_api/wit/since_v0.8.0/extension.wit` — the authoritative WIT world: imports `context-server`, `dap`, `github`, `http-client`, `platform`, `process`, `nodejs`; exports `init-extension`, `language-server-command`, `run-slash-command`, `context-server-command`, `get-dap-binary`, etc. (183 lines).
- `crates/extension_api/wit/since_v0.8.0/{process,dap,slash-command,context-server,lsp,platform,nodejs,http-client}.wit` — typed interfaces.
- `crates/extension_api/src/process.rs` — `Command::new/arg/args/env/envs/output()` convenience wrapper over `process::run-command`.
- `crates/extension_api/src/settings.rs` — re-export of `LspSettings`, `ContextServerSettings`, `CommandSettings`.
- `crates/extension/src/extension.rs` — the core-side `Extension` trait (lines 51–191) plus `WorktreeDelegate` / `ProjectDelegate` / `KeyValueStoreDelegate`.
- `crates/extension/src/extension_manifest.rs` — `ExtensionManifest` struct (lines 87–127) and `provides()` (129–166).
- `crates/extension/src/capabilities.rs` + `capabilities/{process_exec,download_file,npm_install_package}_capability.rs` — capability model.

### 1.2 Guest trait: what an extension can do

`zed_extension_api::Extension` (`extension_api.rs:69`) has these overridable methods (all have no-op/error defaults):

| Method | Purpose |
|---|---|
| `fn new() -> Self` | constructor (registered via `register_extension!(Type)`) |
| `language_server_command(&mut self, &LanguageServerId, &Worktree) -> Result<Command>` | **spawn command for an LSP** |
| `language_server_initialization_options`, `..._workspace_configuration`, `..._schema` (init & workspace) | LSP JSON payloads/schemas |
| `language_server_additional_initialization_options` / `..._workspace_configuration` | inject config into *another* language server (e.g. JSP ↔ JDT LS coupling) |
| `label_for_completion`, `label_for_symbol` | custom `CodeLabel` rendering |
| `complete_slash_command_argument`, `run_slash_command` | Assistant slash commands |
| `context_server_command`, `context_server_configuration` | MCP-style context servers |
| `suggest_docs_packages`, `index_docs` | `/docs` indexing |
| `get_dap_binary`, `dap_request_kind`, `dap_config_to_scenario`, `dap_locator_create_scenario`, `run_dap_locator` | **debug adapters and debug locators** |

Host helpers available from WIT: `download_file` (with extraction), `make_file_executable`, `set_language_server_installation_status`, `get_settings(path, category, key)`, `node_binary_path`, `npm_install_package`, `current_platform`, `latest_github_release`, `Command::output()` (synchronous child process).

### 1.3 Manifest: what an extension can *contribute*

`ExtensionManifest` (`extension_manifest.rs:88`):

```rust
pub id, name, version, schema_version, description, repository, authors,
pub lib: LibManifestEntry,             // kind: Rust => compile src/ to WASM
pub themes: Vec<RelPathBuf>,
pub icon_themes: Vec<RelPathBuf>,
pub languages: Vec<RelPathBuf>,        // paths to languages/<name>/config.toml
pub grammars: BTreeMap<Arc<str>, GrammarManifestEntry>,   // {repository, rev, path}
pub language_servers: BTreeMap<LanguageServerName, LanguageServerManifestEntry>,
pub context_servers: BTreeMap<Arc<str>, ContextServerManifestEntry>,
pub slash_commands: BTreeMap<Arc<str>, SlashCommandManifestEntry>,
pub snippets: Option<ExtensionSnippets>,
pub capabilities: Vec<ExtensionCapability>,
pub debug_adapters: BTreeMap<Arc<str>, DebugAdapterManifestEntry>, // {schema_path}
pub debug_locators: BTreeMap<Arc<str>, DebugLocatorManifestEntry>,
pub language_model_providers: BTreeMap<Arc<str>, LanguageModelProviderManifestEntry>,
```

`provides()` (lines 129–166) maps these to `cloud_api_types::ExtensionProvides` (Themes, Languages, Grammars, LanguageServers, ContextServers, Snippets, DebugAdapters). Note: in this checkout slash commands **alone** are a deprecated feature in the extension CLI (`extension_cli/src/main.rs:207-228, 798-813`) — declare them alongside another feature.

`populate_defaults()` (355–483) auto-discovers `languages/*/config.toml`, `themes/*.json`, `icon_themes/*.json`, and `snippets.json`, so a language extension does **not** need an explicit `languages = [...]` entry in `extension.toml`.

### 1.4 Capability gating of external processes (important for javac/ant)

`ExtensionManifest::allow_exec` (168–187) checks the manifest's `[[capabilities]]`; `ProcessExecCapability::allows` (`capabilities/process_exec_capability.rs:14-39`) supports exact args, `*` (single wildcard arg) and trailing `**`. `CapabilityGranter::grant_exec` (`extension_host/src/capability_granter.rs:23-47`) requires **both** a manifest declaration **and** a host grant. Host grants come from `ExtensionSettings::granted_capabilities` (`extension_host/src/extension_settings.rs:8-20, 45-64`), sourced from `assets/settings/default.json:2413-2417`:

```json
"granted_extension_capabilities": [
  { "kind": "process:exec", "command": "*", "args": ["**"] },
  { "kind": "download_file", "host": "*", "path": ["**"] },
  { "kind": "npm:install", "package": "*" }
]
```

So by default any manifest-declared command is allowed, but the user can restrict it. The WASM host enforces it in `extension_host/src/wasm_host/wit/since_v0_8_0.rs:899-918` (`impl process::Host for WasmState::run_command`). The in-tree example is `extensions/test-extension/extension.toml:17-25`.

**Recommendation:** do *not* try to run `javac`, `ant`, or `catalina.sh` from the WASM extension. `Command::output()` is synchronous, stdout-buffered (no PTY), and capability-gated; a Tomcat server is long-lived. Drive external tools from `kairo_core` (Rust) via `util::command` / `util::process`, and use the extension only to *locate* binaries (`Worktree::which("java")`) and to resolve LSP/DAP commands.

### 1.5 Registration path (how contributions reach core)

`extension_host/src/extension_host.rs` `load_extensions` (≈1480–1749):

1. `proxy.register_grammars(grammars_to_add)` (1506) — WASM grammars downloaded to `<installed_dir>/<ext>/grammars/<name>.wasm`.
2. `proxy.update_languages(..., registrations)` (1542–1545) with `LanguageRegistration { name, grammar_name, matcher, hidden, load }` (1521–1539); `load_plugin_language` reads config + queries from disk.
3. `WasmExtension::load(...)` (1667) compiles/instantiates the WASM module.
4. For each manifest entry: `proxy.register_language_server(...)` (1700–1708), `register_context_server(...)` (1710–1713), `register_debug_adapter(...)` (1715–1730), `register_debug_locator(...)` (1732–1735).

The language-server proxy implementation is `crates/language_extension/src/extension_lsp_adapter.rs:54-69` (`ExtensionLanguageServerProxy::register_language_server` → `LanguageRegistry::register_lsp_adapter` with an `ExtensionLspAdapter`). The DAP proxy is `crates/debug_adapter_extension/src/debug_adapter_extension.rs:33-56` and `extension_dap_adapter.rs:79-121`.

### 1.6 Worked example (annotated): `extensions/html`

```
extensions/html/
├── extension.toml              # [language_servers.vscode-html-language-server] + [grammars.html]
├── languages/html/config.toml  # name, grammar, path_suffixes, brackets, comments, ...
├── languages/html/{highlights,indents,injections,outline,overrides,brackets}.scm
├── Cargo.toml                  # zed_extension_api dependency, crate-type cdylib
└── src/html.rs                 # impl zed::Extension
```

`extensions/html/extension.toml`:
```toml
[language_servers.vscode-html-language-server]
name = "vscode-html-language-server"
language = "HTML"
[language_servers.vscode-html-language-server.language_ids]
"HTML" = "html"
"CSS" = "css"
[grammars.html]
repository = "https://github.com/tree-sitter/tree-sitter-html"
commit = "bfa075d83c6b97cd48440b3829ab8d24a2319809"
```

`src/html.rs:58-112` shows the canonical LSP pattern: `worktree.which(BINARY_NAME)` first, else download an npm package via `npm_package_latest_version` / `npm_install_package` with `set_language_server_installation_status`, then return `zed::Command { command, args, env }`; `language_server_workspace_configuration` reads `LspSettings::for_worktree(server_id, worktree).settings`.

`extensions/proto/src/proto.rs:22-45` shows **multiple language servers** dispatched on `language_server_id.as_ref()`; each lives in `src/language_servers/*.rs`.

### 1.7 How to integrate "Kairo Java/JSP extension"

Create `extensions/kairo-java/` (or `kairo/extensions/kairo-java/`) with:

```
extension.toml           # id="kairo-java", [grammars.java], [grammars.jsp],
                         # [language_servers.jdtls], [language_servers.jsp-language-server],
                         # [debug_adapters.java] + [[capabilities]] for probing `java -version`
languages/java/config.toml, *.scm
languages/jsp/config.toml, highlights.scm, injections.scm, indents.scm, ...
debug_adapter_schemas/java.json   # JSON schema for Zed's debug.json "java" adapter
src/kairo_java.rs         # impl zed_extension_api::Extension
```

To ship it with the fork without touching more upstream files than necessary:
add one line to root `Cargo.toml` `members` (`"extensions/kairo-java",` — the file already lists `extensions/glsl` etc. as members) and one entry under `assets/settings/default.json:2405` `auto_install_extensions` (`"kairo-java": true`). That is 2 upstream lines; a user can also just install it manually with zero core changes.

---

## 2. Language support (grammars, queries, config)

### 2.1 Two tracks

| Track | Where | How |
|---|---|---|
| Built-in languages | `crates/languages/src/{bash,c,cpp,css,go,json,python,rust,typescript,yaml,...}.rs` + `crates/languages/src/lib.rs::init` (58–336) + grammars/queries in `crates/grammars/src/<name>/` | `LanguageInfo { name, adapters, context, toolchain, manifest_name, semantic_token_rules }` array (91–229), registered by `register_language()` (348–390). Grammars compiled in via `grammars::native_grammars()` (`crates/grammars/src/grammars.rs:19-46`). |
| Extension languages | `extensions/<id>/languages/<name>/` | Discovered by `ExtensionManifest::populate_defaults`, loaded by `load_plugin_language` in `extension_host`. Grammar WASM declared in `[grammars.<name>]` and fetched at build time by `extension_cli`. |

`LanguageRegistry::register_language(name, grammar_name, matcher, hidden, manifest_name, load)` is at `crates/language/src/language_registry.rs:439-467`; extension bulk updates use `update_extension_languages(...)` (474+).

### 2.2 `config.toml` schema

`LanguageConfig` is defined in `crates/language_core/src/language_config.rs:28-148`, `FILE_NAME = "config.toml"` (151). Notable fields for JSP:

- `name`, `grammar`, flattened `matcher` (`path_suffixes`, `first_line_pattern` — `LanguageMatcher` at line 232)
- `brackets`, `autoclose_before`, `line_comments`, `block_comment`, `wrap_characters`
- `increase_indent_pattern` / `decrease_indent_pattern` / `decrease_indent_patterns`
- `injections`-related: `scope_opt_in_language_servers`
- `tab_size`, `hard_tabs`, `word_characters`, `completion_query_characters`, `linked_edit_characters`
- `hidden` (for injection-only languages such as the Java fragments inside a JSP)
- **`debuggers: IndexSet<SharedString>`** (line 147) — declare preferred debugger names (e.g. `"java"`) so `debugger::Start` picks the JDWP adapter by default.

`html/languages/html/config.toml` (19 lines) is a compact template; see §1.6.

### 2.3 Query files

`crates/language_core/src/queries.rs:9-30` enumerates exactly these optional `.scm` files:
`highlights.scm`, `brackets.scm`, `outline.scm`, `indents.scm`, `injections.scm`, `overrides.scm`, `redactions.scm`, `runnables.scm`, `debugger.scm`, `textobjects.scm`.
`language_core::grammar` uses them for syntax highlighting, indentation, outlines, injections, and inline runnable/debugger gutter markers.

### 2.4 How to integrate JSP syntax

1. Author/find a `tree-sitter-jsp` grammar (or reuse `tree-sitter-java` plus a JSP grammar with injections). Reference it in `[grammars.jsp]` with a pinned commit — `GrammarManifestEntry` is `{repository, rev (alias "commit"), path?}` (`extension_manifest.rs:240-247`).
2. Create `languages/jsp/config.toml` with `name = "JSP"`, `grammar = "jsp"`, `path_suffixes = ["jsp"]`, `debuggers = ["java"]`, and Java-style comments/brackets.
3. Add `highlights.scm`, `indents.scm`, and especially `injections.scm` to highlight Java scriptlets (`<% … %>`), EL (`${…}`), HTML, CSS and JS inside the JSP. Use `hidden = true` for a `jsp-java` injection-only language if needed.
4. Optional: add a Java language config (`name = "Java"`, `path_suffixes = ["java"]`) so `.java` files highlight too — no built-in Java exists in this checkout.

Tradeoff: extension languages are loaded from the extension install directory and are **rejected if a native language with the same name exists** (`language_registry.rs:335-344` for LSP adapters, and `ExtensionLanguagesUpdateOutcome` for languages). Since no Java/JSP native language exists, there is no conflict now — but if upstream ever adds one, the extension's language registration will be shadowed. That is acceptable (and desirable) for rebasing.

---

## 3. LSP — attaching JDT LS / JSP language server

### 3.1 Core types

| Symbol | File |
|---|---|
| `pub trait LspAdapter: DynLspInstaller` | `crates/language/src/language.rs:523-715` |
| `pub trait LspAdapterDelegate` | `crates/language/src/language.rs:499-520` |
| `pub trait LspInstaller` / `DynLspInstaller` | `crates/language/src/language.rs:717-776` |
| `LanguageServerBinary { path, arguments, env }` | `crates/lsp/src/lsp.rs:95-101` |
| `LanguageServerBinaryOptions` | `crates/lsp/src/lsp.rs:103-110` |
| `LanguageServer::new(...)` — **actual process spawn** | `crates/lsp/src/lsp.rs:516-552` (`util::command::new_command(&binary.path)` + piped stdio + `kill_on_drop(true)`) |
| `LanguageRegistry::register_lsp_adapter` / `::register_available_lsp_adapter` | `crates/language/src/language_registry.rs:332-357` / `281-302` |
| `LspStore::get_language_server_binary` | `crates/project/src/lsp_store.rs:790-...` |
| `ExtensionLspAdapter` (extension → LspAdapter bridge) | `crates/language_extension/src/extension_lsp_adapter.rs:147-...` |
| `LspSettings` (core) | `crates/settings_content/src/project.rs:274` |
| `LspSettings` (WASM author view) | `crates/extension_api/wit/since_v0.8.0/settings.rs` |

`LspAdapter` key methods: `name()`, `initialization_options()`, `workspace_configuration()`, `settings_schema()`, `initialization_options_schema()`, `language_ids()`, `is_opt_in_for()`, `is_extension()`, plus completion/symbol label hooks. `LspAdapterDelegate` exposes `which()`, `shell_env()`, `read_text_file()`, `http_client()`, `npm_package_installed_version()`, `try_exec()`.

### 3.2 Registration & launch flow (extension case)

```
extension.toml [language_servers.jdtls] languages=["Java","JSP"]
  → extension_host.rs:1700 register_language_server(...)
  → language_extension/extension_lsp_adapter.rs:63 ExtensionLspAdapter::new(...)
  → LanguageRegistry::register_lsp_adapter(LanguageName, adapter)
  → LspStore (per worktree) resolves adapter via get_language_server_binary()
       ├─ "lsp.<name>.binary.path" setting overrides everything (lsp_store.rs:801-838)
       └─ adapter.get_language_server_command(...) → ExtensionLspAdapter → WASM
            Extension::language_server_command() → zed::Command { command, args, env }
  → LanguageServer::new() spawns the process (lsp.rs:540-552)
```

### 3.3 Settings

- Per-language server selection: `assets/settings/default.json:2470` `"languages": { "Java": { "language_servers": ["jdtls", "..."] } }`, or omit jdtls from the list to disable. `"..."` means "all other registered servers"; `"!name"` disables one (`LanguageSettings::language_servers` at `crates/language/src/language_settings.rs:101-107`).
- Server-specific `initialization_options`, `settings`, and `binary` are under top-level `"lsp": { "<server name>": {...} }` (`default.json:2804-2816`); the extension reads these via `LspSettings::for_worktree(server_id.as_ref(), worktree)` (pattern in `extensions/html/src/html.rs:93-109`, `extensions/proto/src/proto.rs:47-63`).
- `register_available_lsp_adapter` (`language_registry.rs:281`) lets a server be enabled by name for *any* language through settings; `register_lsp_adapter` binds it to a specific language by default. Use both for JDT LS: bind to `"Java"`/`"JSP"` via the manifest and optionally register globally if you want users to opt in on other languages.

### 3.4 How to integrate JDT LS / JSP LS

In `extensions/kairo-java/extension.toml`:
```toml
[language_servers.jdtls]
name = "Eclipse JDT Language Server"
languages = ["Java", "JSP"]
[language_servers.jdtls.language_ids]
"Java" = "java"
"JSP" = "jsp"

[language_servers.jsp-language-server]
name = "Eclipse JSP Language Server"
languages = ["JSP"]
```
In `src/kairo_java.rs::language_server_command`, match on `language_server_id.as_ref()`:

- `jdtls`: prefer a workspace-local binary via `worktree.which("jdtls")`; otherwise locate the Kairo-managed JDT LS distribution (the Go agent can expose its path over HTTP; cache it in a struct field). Return `zed::Command { command: java_binary, args: ["-Declipse.application=org.eclipse.jdt.ls.core.id1", "-Dosgi.bundles.defaultStartLevel=4", "-Declipse.product=org.eclipse.jdt.ls.core.product", "-Dlog.level=ALL", "-Xmx1g", "-jar", "<plugins/org.eclipse.equinox.launcher_*.jar>", "-configuration", ..., "-data", <workspace>, "--stdio"], env }`. Note: on JDK 1.6-era projects JDT LS itself needs a modern JVM — run the *compiler/launcher* with JDK 6 separately via tasks and keep JDT LS on its own JVM.
- `jsp-language-server`: similar, with the JSP LS launcher jar.
- Use `language_server_additional_initialization_options("jsp-language-server", "jdtls", ...)` if the JSP LS needs to know the JDT LS settings (the WIT supports targeting another server; see `extension_api.rs:101-115`).
- Implement `language_server_initialization_options` by reading `LspSettings::for_worktree(...).initialization_options` so users can tune JVM args / vmargs in `settings.json` without a recompile.

Then in `assets/settings/default.json` (only if shipping Kairo as built-in) add:
```json
"languages": {
  "Java": { "language_servers": ["jdtls", "..."] },
  "JSP":  { "language_servers": ["jdtls", "jsp-language-server", "..."] }
}
```
If you want **zero** edits to this hot file, register defaults from Rust instead: in `kairo_core::init`, call `SettingsStore::update_global` + `set_extension_settings` like `crates/languages/src/lib.rs:311-321` does — that pattern exists specifically so extensions/add-ons can inject per-language defaults.

---

## 4. DAP — JDWP debugging

### 4.1 Core types

| Symbol | File |
|---|---|
| `pub trait DebugAdapter` | `crates/dap/src/adapters.rs:349-399` |
| `DebugAdapterBinary { command, arguments, envs, cwd, connection, request_args }` | `crates/dap/src/adapters.rs:193-202` |
| `DapRegistry` (`add_adapter`, `add_locator`, `adapters_schema`, `adapter`) | `crates/dap/src/registry.rs:33-97` |
| `DapLocator` trait | `crates/dap/src/registry.rs:14-31` |
| `dap_adapters::init` (built-ins: CodeLLDB, Python, JS, Go, GDB) | `crates/dap_adapters/src/dap_adapters.rs:29-42` |
| `ExtensionDapAdapter` (bridge) | `crates/debug_adapter_extension/src/extension_dap_adapter.rs:21-121` |
| `DapStore::get_debug_adapter_binary` path | `crates/project/src/debugger/dap_store.rs:255-306` |
| DAP process spawn (TCP and stdio transports) | `crates/dap/src/transport.rs:555-597` and `:726+` (`util::command::new_std_command` + `util::process::Child::spawn`) |
| Breakpoints | `crates/project/src/debugger/breakpoint_store.rs`, `crates/dap/src/dap_command.rs`, editor action `ToggleBreakpoint` at `crates/editor/src/actions.rs:889` |
| Debugger settings | `crates/dap/src/debugger_settings.rs`; `assets/settings/default.json:2818-2825` (`"dap": { "<name>": { "env": ... } }`) |

`DebugAdapter` methods: `name()`, `config_from_zed_format(ZedDebugConfig) -> DebugScenario`, `get_binary(delegate, definition, user_installed_path, user_args, user_env, cx) -> DebugAdapterBinary`, `adapter_language_name()`, `request_kind()`, `dap_schema()`.

`DebugAdapterBinary.connection: Option<TcpArguments>` is the key field for JDWP: if set, Zed uses TCP transport (and forwards the port for remote projects); if `command` is also set, Zed spawns that process and dials it (`transport.rs:567-597`).

### 4.2 Extension contribution path

```
extension.toml [debug_adapters.java] (schema_path defaults to debug_adapter_schemas/java.json)
  → extension_host.rs:1715-1730 register_debug_adapter(extension, name, path)
  → debug_adapter_extension/debug_adapter_extension.rs:33 register_debug_adapter(ExtensionDapAdapter::new(...))
  → DapRegistry::add_adapter
  → DapStore on "debugger::Start"/"Rerun" → adapter.get_binary(...)
       → ExtensionDapAdapter::get_binary → WASM Extension::get_dap_binary(...) → DebugAdapterBinary
  → dap::transport spawns/attaches, DAP session starts
```

The extension-side methods map 1:1 (`extension_api.rs:219-283`):
`get_dap_binary(adapter_name, config: DebugTaskDefinition, user_installed_path, worktree) -> DebugAdapterBinary`,
`dap_request_kind(adapter_name, config) -> StartDebuggingRequestArgumentsRequest`,
`dap_config_to_scenario(DebugConfig) -> DebugScenario`, plus the locator pair.
`DebugConfig`/`DebugScenario`/`DebugAdapterBinary` WIT records are in `dap.wit:43-130` — `DebugScenario.config` is a JSON string handed verbatim to the adapter.

### 4.3 How to integrate JDWP

Two viable shapes:

1. **Via the JDWP bridge already in Kairo (recommended).** Your Go agent exposes a JDI bridge that listens on a TCP port and speaks DAP. In `get_dap_binary("java", ...)` return:
   ```rust
   zed_extension_api::DebugAdapterBinary {
       command: None,                       // bridge already running (started by task/panel)
       arguments: vec![],
       envs: vec![],
       cwd: None,
       // WIT type: IpAddress::Ipv4([127, 0, 0, 1]) — or call resolve_tcp_template(...)
       connection: Some(zed_extension_api::TcpArguments { port, host, timeout: None }),
       request_args: StartDebuggingRequestArguments {
           configuration: json!({ "request": "attach", "hostName": "localhost", "port": port }).to_string(),
           request: StartDebuggingRequestArgumentsRequest::Attach,
       },
   }
   ```
   Use `resolve_tcp_template`/`configure_tcp_connection` (`dap.wit:5`, `crates/dap/src/transport.rs`) for a user-configurable port. Provide `debug_adapter_schemas/java.json` so Zed's `debug.json` validates the config. `dap_request_kind` should return `Attach` for JDWP.
2. **Spawn an adapter process.** If you package a Java-based DAP adapter jar, set `command` to the `java` binary, `arguments` to `-jar <dap-adapter.jar>`, and let Zed manage stdio (no `connection`). The extension must first resolve/locate the jar (optionally via `download_file` + a `[[capabilities]] download_file` entry).

Set `debuggers = ["java"]` in `languages/java/config.toml` and `languages/jsp/config.toml` (§2.2) so Zed picks this adapter for the "Debug" action. Build tasks for "compile before debug" use the locator pair (`dap_locator_create_scenario` / `run_dap_locator`) — see the long doc comment at `extension_api.rs:248-264` for the `ant compile` → locate `.class` → attach example.

Breakpoints need no new code: Zed's `BreakpointStore` is language-agnostic; the `debugger.scm` query file can supply breakpoint-worthy positions.

---

## 5. Tasks (build / deploy / hot-deploy)

### 5.1 Core types

| Symbol | File |
|---|---|
| `TaskId`, `SpawnInTerminal`, `ResolvedTask`, `VariableName` | `crates/task/src/task.rs:38,42,114,153` |
| `TaskTemplate`, `TaskTemplates`, `RevealStrategy`, `HideStrategy`, `SaveStrategy`, `TaskHook` | `crates/task/src/task_template.rs:24,141,...` |
| `StaticSource` (JSON file → tasks) | `crates/task/src/static_source.rs:14-123` |
| `VsCodeTaskFile`, `VsCodeDebugTaskFile` | `crates/task/src/{vscode_format,vscode_debug_format}.rs` |
| `Inventory::update_file_based_tasks`, `list_tasks`, `resolve_task` | `crates/project/src/task_inventory.rs:761,448,210` |
| `Workspace::schedule_task` / `schedule_resolved_task` / `spawn_in_terminal` | `crates/workspace/src/tasks.rs:44,70,228` |
| `TerminalProvider` trait + setter | `crates/workspace/src/workspace.rs:327`, `set_terminal_provider` ≈3545 |
| `task::Spawn` / `task::Rerun` actions | `crates/zed_actions/src/lib.rs:735-788` |

`TaskTemplate` fields (see `task_template.rs:24-105`): `label`, `command`, `args`, `env`, `cwd`, `use_new_terminal`, `allow_concurrent_runs`, `reveal`, `reveal_target`, `hide`, `tags`, `shell`, `show_summary`, `show_command`, `save`, `hooks`. Substitutions available: `$ZED_FILE`, `$ZED_FILENAME`, `$ZED_DIRNAME`, `$ZED_WORKTREE_ROOT`, `$ZED_SYMBOL`, `$ZED_ROW`, `$ZED_COLUMN`, `$ZED_SELECTED_TEXT`, `$ZED_LANGUAGE`, git variables, and `Custom(...)` (`task.rs:153-201`).

### 5.2 How to integrate compile/deploy/start/stop

**Zero-new-action approach (strongly recommended for small diffs):**
Ship a `.zed/tasks.json` (and optionally an `ant` build file) in the sample project / generated worktree with tasks like:

```json
[
  { "label": "Kairo: Compile",       "command": "ant", "args": ["compile"], "cwd": "$ZED_WORKTREE_ROOT", "reveal": "always", "hide": "on_success" },
  { "label": "Kairo: Hot Deploy",    "command": "curl", "args": ["-X", "POST", "http://localhost:8080/manager/text/reload?path=/app"] },
  { "label": "Kairo: Start Tomcat",  "command": "catalina.sh", "args": ["run"], "use_new_terminal": true, "allow_concurrent_runs": false, "hide": "never" },
  { "label": "Kairo: Stop Tomcat",   "command": "catalina.sh", "args": ["stop"] }
]
```

Then bind keys directly to the existing action (`assets/keymaps/default-*.json` line ~722 pattern):
```json
"ctrl-alt-b": ["task::Spawn", { "task_name": "Kairo: Compile" }]
```
`task::Spawn::ByName { task_name, reveal_target }` is defined at `zed_actions/src/lib.rs:739-745`; no new action or Rust code is needed for compile/deploy.

**Programmatic approach (when the Go agent drives the operation):**
In `kairo_core`, construct a `TaskTemplate` and call `Workspace::schedule_task` (or build a `ResolvedTask` and call `schedule_resolved_task`) from a Kairo action handler. This reuses Zed's PTY, reveal/hide, and task history without a `TerminalProvider` fight. For long-lived processes (Tomcat, the Go agent, the JDI bridge) prefer `util::process::Child` managed by `kairo_core` and stream logs into the Kairo panel (see `05-zed-core-layout.md` §4-5).

`tasks.json` is *file-based* only (`StaticSource`); extensions cannot register static tasks. If tasks must be generated from the Kairo settings, call `Inventory::update_file_based_tasks(TaskSettingsLocation, Some(&json))` from `kairo_core` after project load (the inventory is reachable via `project.task_store().read(cx).task_inventory()`).

---

## 6. Actions & keybindings

- Actions are declared with the `gpui::actions!` macro or `#[derive(Action)]`. Examples: `actions!(debugger, [Start, Continue, ...])` at `crates/debugger_ui/src/debugger_ui.rs:29-88`; data-carrying actions as structs at `:91-101`; `zed_actions` central namespace at `crates/zed_actions/src/lib.rs` (`task::Spawn` at 735, `debug_panel::Toggle` at 828+).
- Registration is per-workspace: `cx.observe_new(|workspace: &mut Workspace, _, _| { workspace.register_action(...).register_action(...) })` — `debugger_ui.rs:114-...`, `terminal_view/src/terminal_panel.rs:55-75`.
- Runtime keybinding registration (avoids editing `assets/keymaps/default-*.json`): `cx.bind_keys([KeyBinding::new(...)])` — pattern cited in `05-zed-core-layout.md` §3.3 (`crates/agent_ui/src/agent_panel.rs:9515`).
- Default keybinding files: `assets/keymaps/default-{linux,macos,windows}.json` (65–68 KB each; high conflict risk on rebase). Task/debugger examples: `default-windows.json:720-728` (`task::Rerun`, `task::Spawn`), `:747-751` (debugger step keys).

**Recommendation:** define only the Kairo-specific UI actions (`kairo::TogglePanel`, `kairo::Compile`, `kairo::Deploy`, `kairo::StartServer`, `kairo::StopServer`, `kairo::HotDeploy`) with `actions!(kairo, [...])` in `kairo_core`/`kairo_ui`, bind them with `cx.bind_keys` at init, and implement them either as `workspace.schedule_task(...)` calls or HTTP calls to the Go agent. Where the operation maps exactly to a task, prefer the `task::Spawn::ByName` keybinding and skip the custom action.

---

## 7. Settings

| Concern | Location |
|---|---|
| `Settings` / `RegisterSetting` traits, `SettingsStore` | `crates/settings/src/settings_store.rs` (`Settings::register` ≈86) |
| Per-language settings | `crates/language/src/language_settings.rs:42-120` (`AllLanguageSettings` with `RegisterSetting`) |
| Content types (`LanguageSettingsContent`, `LspSettings`, `SemanticTokens`, …) | `crates/settings_content/src/` (`project.rs:274` for `LspSettings`) |
| Global defaults | `assets/settings/default.json` — 129 KB, hot rebase file. `languages` at 2470, `lsp` at 2804, `dap` at 2818, `global_lsp_settings` at 2827, `granted_extension_capabilities` at 2413 |
| Extension-visible settings | `get-settings` WIT import (`extension.wit:55`); `LspSettings::for_worktree` in `extensions/html/src/html.rs:98` |

**Recommendation:** implement `KairoSettings` with `#[derive(RegisterSetting)]` + `impl Settings` in `kairo_core` and call `KairoSettings::register(cx)` from `kairo_core::init` (pattern: `crates/extension_host/src/extension_settings.rs:8-20,39-66`). This registers the schema automatically — **do not edit `default.json`**. If per-language defaults are needed for Java/JSP, use `SettingsStore::set_extension_settings(...)` as `crates/languages/src/lib.rs:311-321` does (it literally exists to let non-core code inject `all_languages` defaults). A dedicated `"kairo": { ... }` section can also be registered programmatically; the user can override it in their own `settings.json`.

---

## 8. Panels / UI

| Symbol / file | Role |
|---|---|
| `pub trait Panel: Focusable + EventEmitter<PanelEvent> + Render + Sized` | `crates/workspace/src/dock.rs:36-104` |
| `PanelHandle` (blanket impl for `Entity<T: Panel>`) | `dock.rs:106-154` |
| `Workspace::add_panel`, `panel::<P>`, `toggle_panel_focus::<P>`, `close_panel::<P>` | `crates/workspace/src/workspace.rs:2921-2948`, `≈5071` |
| Startup panel loading | `crates/zed/src/zed.rs:778-818` (`initialize_panels`, helper `add_panel_when_ready` at 788, `futures::join!` at 803) |
| Examples | `crates/terminal_view/src/terminal_panel.rs` (`init` 55, `load` 237, `impl Panel` ≈1692), `crates/debugger_ui/src/debugger_panel.rs`, `crates/git_ui/src/git_panel.rs`, `crates/project_panel/src/project_panel.rs` |

`Panel` requires `persistent_name()`, `panel_key()`, `position()/set_position()`, `default_size()`, `icon()/icon_tooltip()`, `toggle_action()`, `activation_priority()`; `enabled(cx)` can hide the panel when no Kairo project is open.

**Recommendation:** `kairo_ui::KairoPanel` holds `WeakEntity<Workspace>`/`WeakEntity<Project>` plus a `kairo_core` service handle; render server status, compile state, and streamed Go-agent/JDI logs. Register actions in `kairo_ui::init` with `cx.observe_new`, add the panel with one line in `crates/zed/src/zed.rs` `futures::join!` (see `05-zed-core-layout.md` §6.2). A worktree "Tomcat context" view can be a separate small panel or a `project_panel` context-menu extension (`explorer_command_injector` crate is an example of injecting commands into the project panel without editing it).

---

## 9. Process / exec helpers

| Layer | API | Notes |
|---|---|---|
| Core command builder | `util::command::new_command(program)` and `util::command::new_std_command` | `crates/util/src/command.rs:14-18`; cross-platform `Command` wrapper over `smol::process::Command` (`:28-128`), Windows `CREATE_NO_WINDOW` flag (`:11-12,36`), async `output()`/`status()` (`:117-123`) |
| Safe process-tree spawn | `util::process::Child::spawn(command, stdin, stdout, stderr)` | `crates/util/src/process.rs:11,33/56`; new session (Unix) / Job Object (Windows) so Tomcat + JDI bridge descendants are reaped |
| Task shell | `ShellBuilder`, `Shell`, `ShellKind`, `get_system_shell` | `crates/util/src/{shell_builder,shell}.rs`, re-exported from `crates/task/src/task.rs:29-30` |
| LSP spawn | `LanguageServer::new` | `crates/lsp/src/lsp.rs:516-552`, piped stdio, `kill_on_drop(true)` |
| DAP spawn | `dap::transport` | `crates/dap/src/transport.rs:567-597` (TCP variant), `:726+` (stdio variant) |
| Terminal/task spawn | `spawn_task_subprocess` | `crates/terminal/src/terminal.rs:3376+` |
| Extension-side exec | `zed_extension_api::Command::output()` | `crates/extension_api/src/process.rs:41-43` → WIT `process::run-command`; synchronous, buffered, capability-gated (§1.4) |
| Async scheduling | `cx.spawn`, `cx.spawn_in`, `cx.background_spawn`, GPUI timers | Keep `Task`s in fields or detach deliberately (AGENTS.md) |

Concrete Kairo launch:
```rust
let mut cmd = util::command::new_command(&agent_path);
cmd.arg("--config").arg(config_path);
let child = util::process::Child::spawn(cmd.into(), Stdio::null(), Stdio::piped(), Stdio::piped())?;
let wait: Task<Result<ExitStatus>> =
    cx.background_spawn(async move { child.wait().await.context("kairo agent exited") });
```
For `javac` one-shot builds, prefer a `TaskTemplate` (§5) so output lands in Zed's terminal with correct `cwd`/env; for the long-lived Tomcat/Go agent/JDI bridge, own the child in `kairo_core` as above.

---

## 10. Minimal core changes vs extension-only — tradeoffs

### Option A — "Everything in the extension"

| Pros | Cons |
|---|---|
| Zero/zero-few upstream edits; trivially rebasable | Cannot add actions, keybindings, settings schema, panels, or static tasks — no compile/deploy commands or Kairo UI are possible |
| Language/grammar/LSP/DAP are first-class extension features | `Command::output` is synchronous + capability-gated; unsuitable for Tomcat / Go agent / long-running builds |
| Installable by users without a fork | No persistent app state except `KeyValueStore` (docs-only) and settings reads |
| — | The Go-agent HTTP bridge (async client, process lifecycle, panel) cannot live in WASM |

**Verdict:** insufficient alone; good for syntax + tooling registration only.

### Option B — "Two Rust crates + one extension" (recommended)

| Piece | Owner | Upstream lines touched |
|---|---|---|
| `kairo-java` extension (grammars, Java/JSP languages, JDT LS/JSP LS, JDWP DAP adapter, slash commands) | `kairo/extensions/kairo-java/` or `extensions/kairo-java/` | 0 if user-installed; 2 to ship built-in (`Cargo.toml` member + `auto_install_extensions`) |
| `kairo_core` (settings, actions, task definitions, Go-agent client, process lifecycle) | `kairo/crates/kairo_core/` | root `Cargo.toml` (2), `crates/zed/Cargo.toml` (1), `crates/zed/src/main.rs` (1 `init`) |
| `kairo_ui` (KairoPanel, keybindings via `cx.bind_keys`, menus) | `kairo/crates/kairo_ui/` | root `Cargo.toml` (2), `crates/zed/Cargo.toml` (1), `main.rs` (1), `zed.rs` (1 `KairoPanel::load`) |
| Task definitions | generated `.zed/tasks.json` in sample projects | 0 |

Total: ~5–9 lines across 4 upstream files (exact accounting in `05-zed-core-layout.md` §7.2), plus 0–2 optional lines for the built-in extension. Everything else lives under `kairo/`.

### Option C — "Everything in core"

Adding Java/JSP as native languages (`crates/languages/src/java.rs`, `crates/grammars/src/java/`, `LanguageInfo` entries), native `LspAdapter`/`DebugAdapter` implementations (like `crates/languages/src/go.rs` and `crates/dap_adapters/src/go.rs`), plus static defaults in `default.json`:

- Pros: no WASM toolchain/capability constraints; full control over initialization options and process lifecycle.
- Cons: touches **the hottest rebase files** — `crates/languages/src/lib.rs`, `crates/languages/Cargo.toml`, `crates/grammars/*`, `crates/dap_adapters/src/dap_adapters.rs`, `assets/settings/default.json` — easily 50–200 lines of upstream conflict surface for features that the extension API already covers.
- **Verdict:** avoid. Only consider if you need deep editor behaviors an extension cannot express (e.g. custom `LspAdapter::prepare_initialize_params` on every Java buffer, or custom `CodeLabel` logic beyond `label_for_symbol`) — and even then, those hooks are exposed to extensions.

### Risk notes for syncing with upstream

- `assets/settings/default.json`, `assets/keymaps/default-*.json`, `crates/zed/src/zed.rs`, `crates/zed/src/main.rs`, root `Cargo.toml` are high-conflict; keep edits to the single marked lines and tag them `// kairo: integration` (Cargo allows `# kairo: integration`).
- Extension languages are shadowed if upstream ever adds a native `Java`/`JSP` language — this is a *safe* failure mode (you lose Kairo highlighting, the rest of Zed keeps working).
- WIT API version is pinned by `crates/extension_api/wit/since_v0.8.0/` and the `zed:api-version` custom section (`extension.rs:193-230`, `extension_api.rs:350-353`). After a rebase that bumps the API version, rebuild the extension; the CLI validates it.

---

## Appendix — quick symbol reference

| Need | Symbol | File |
|---|---|---|
| Guest extension trait | `zed_extension_api::Extension` | `crates/extension_api/src/extension_api.rs:69` |
| Register extension | `register_extension!` | `extension_api.rs:289-334` |
| Core-side extension trait | `extension::Extension` | `crates/extension/src/extension.rs:52` |
| Manifest model | `ExtensionManifest` | `crates/extension/src/extension_manifest.rs:88` |
| Contribution flags | `ExtensionManifest::provides` | `extension_manifest.rs:129` |
| Exec capability | `ProcessExecCapability::allows`, `CapabilityGranter::grant_exec` | `capabilities/process_exec_capability.rs:14`; `extension_host/src/capability_granter.rs:23` |
| Process gating at host | `impl process::Host for WasmState::run_command` | `extension_host/src/wasm_host/wit/since_v0_8_0.rs:899` |
| Extension loading | `ExtensionHost::load_extensions` | `crates/extension_host/src/extension_host.rs:1480-1749` |
| Language config model | `LanguageConfig`, `LanguageMatcher` | `crates/language_core/src/language_config.rs:28,232` |
| Query files | `QueryFile` | `crates/language_core/src/queries.rs:9-30` |
| Language registry | `LanguageRegistry::{register_language, register_lsp_adapter, register_available_lsp_adapter}` | `crates/language/src/language_registry.rs:439,332,281` |
| LSP adapter | `LspAdapter`, `LspAdapterDelegate` | `crates/language/src/language.rs:523,499` |
| LSP process spawn | `LanguageServer::new` | `crates/lsp/src/lsp.rs:516` |
| LSP binary resolution | `LspStore::get_language_server_binary` | `crates/project/src/lsp_store.rs:790` |
| Extension LSP bridge | `ExtensionLspAdapter`, `ExtensionLanguageServerProxy` | `crates/language_extension/src/extension_lsp_adapter.rs:147,54` |
| LSP settings (user) | `LspSettings` | `crates/settings_content/src/project.rs:274`; `assets/settings/default.json:2804` |
| DAP adapter trait | `dap::adapters::DebugAdapter` | `crates/dap/src/adapters.rs:349` |
| DAP binary | `DebugAdapterBinary` | `crates/dap/src/adapters.rs:193` |
| DAP registry | `DapRegistry::{add_adapter, add_locator}` | `crates/dap/src/registry.rs:49,54` |
| Extension DAP bridge | `ExtensionDapAdapter` | `crates/debug_adapter_extension/src/extension_dap_adapter.rs:21` |
| DAP resolution | `DapStore` (`get_debug_adapter_binary` path) | `crates/project/src/debugger/dap_store.rs:255-306` |
| DAP process spawn | TCP / stdio transports | `crates/dap/src/transport.rs:555,726` |
| Breakpoints | `BreakpointStore`, `editor::actions::ToggleBreakpoint` | `crates/project/src/debugger/breakpoint_store.rs`; `crates/editor/src/actions.rs:889` |
| Task currency | `SpawnInTerminal`, `TaskTemplate`, `ResolvedTask` | `crates/task/src/{task.rs:42,task_template.rs:24,task.rs:114}` |
| Schedule a task | `Workspace::{schedule_task, schedule_resolved_task}` | `crates/workspace/src/tasks.rs:44,70` |
| Task actions | `task::Spawn`, `task::Rerun` | `crates/zed_actions/src/lib.rs:735,768` |
| Settings registration | `Settings::register`, `RegisterSetting` | `crates/settings/src/settings_store.rs:69,86` |
| Panel trait / add | `workspace::Panel`, `Workspace::add_panel` | `crates/workspace/src/dock.rs:36`; `workspace.rs:2921` |
| Startup panel load | `initialize_panels`, `add_panel_when_ready` | `crates/zed/src/zed.rs:778,788` |
| Core process helpers | `util::command::new_command`, `util::process::Child::spawn` | `crates/util/src/command.rs:16`; `crates/util/src/process.rs:33/56` |
| Extension build/validate CLI | `zed-extension --source-dir … --output-dir … --scratch-dir …` | `crates/extension_cli/src/main.rs:50-130` |
