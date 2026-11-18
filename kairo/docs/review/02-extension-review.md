# 02 — Kairo Java / JSP WASM Extension & Config Review

- **Review date:** 2026-11-19
- **Reviewer:** extension/config strict-review subagent (read-only; this report is the only file written)
- **Branch under review:** `kairo/integration` @ `5da1d9f`
- **Scope:** `kairo/extensions/kairo-java/**`, `kairo/fixtures/legacy-sample/.zed/tasks.json`, root `Cargo.toml` registration
- **Ground truth:** real source in `crates/extension_api`, `crates/extension`, `crates/extension_host`, `crates/language_core`, `crates/task`, `crates/debug_adapter_extension`, `crates/dap`, plus in-tree `extensions/html`, `extensions/proto`, `extensions/test-extension`; published `zed_extension_api` 0.7.0 crate downloaded from crates.io; node types fetched from the pinned grammar commits.

---

## Executive summary — severity-ordered findings

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| F1 | **BLOCKER** | `languages/java/config.toml` `block_comment` omits required `tab_size`. `BlockCommentConfig`'s custom `Deserialize` requires all four fields in the new table form; with three it matches neither the `New` nor `Old` variant. Java language config fails to load. | Needs fix |
| F2 | **BLOCKER** | `languages/java/highlights.scm` references `(floating_point_literal)`, which does **not exist** in tree-sitter-java at the pinned commit. `Query::new` fails → Java highlighting query rejected. | Needs fix |
| F3 | Info (hypothesis disproven) | `zed_extension_api = "0.7.0"` is **fine**: DAP types/trait signatures exist in 0.7.0 and the host accepts API version 0.7.0 on every release channel. 0.7.0 is also exactly what in-tree `html`/`proto` pin. | Verified OK |
| F4 | Low | `[language_servers.jdtls] name = "..."` is not a field of `LanguageServerManifestEntry`; serde silently ignores it (no effect on server identity). | Optional cleanup |
| F5 | Low | `[[capabilities]] process:exec java -version` is valid syntax but unused — no code path in `kairo_java.rs` spawns a process. | Optional cleanup |
| F6 | Low | README's `cargo build -p kairo_java --target wasm32-wasip2` is not the canonical validation; `zed-extension test` is what would catch F1/F2 (no Rust toolchain here to run it). | Doc note |

Everything else checked (extension.toml fields, Cargo.toml, Rust source, JSP config, JSP queries, DAP schema, tasks.json, root registration) is valid.

---

## 1. extension.toml — verdict: VALID (one no-op field)

Checked against `ExtensionManifest` in `crates/extension/src/extension_manifest.rs:88-127` and `LanguageServerManifestEntry` at `:249-265`.

| Field in target | Schema reality | Verdict |
|---|---|---|
| `id`, `name`, `version`, `schema_version = 1`, `description`, `authors`, `repository` | Required/optional fields all present and correctly typed; `SchemaVersion::CURRENT = 1` (`:56`). | OK |
| `[grammars.java] commit = "..."` (and jsp) | `GrammarManifestEntry { repository, #[serde(alias = "commit")] rev, path }` at `:240-247`. `commit` is an explicit serde alias for `rev`. | OK — `commit` accepted |
| `[language_servers.jdtls] languages = ["Java", "JSP"]` | `LanguageServerManifestEntry.languages: Vec<LanguageName>` (`:255-256`); `languages()` prefers it over deprecated singular `language`. | OK (uses the modern field, unlike `extensions/html` which uses deprecated `language =`) |
| `[language_servers.jdtls.language_ids]` `"Java" = "java"`, `"JSP" = "jsp"` | `language_ids: HashMap<LanguageName, String>` (`:258`). Keys match the `name` values in both `config.toml` files. | OK |
| `[debug_adapters.java] schema_path = "..."` | `DebugAdapterManifestEntry { schema_path: Option<RelPathBuf> }` (`:305-308`); `build_debug_adapter_schema_path` (`:214-227`) uses it verbatim. | OK |
| `[[capabilities]] kind = "process:exec"` + `command`/`args` | `ExtensionCapability` is internally tagged with `#[serde(tag = "kind")]`, variant renamed `"process:exec"` (`crates/extension/src/capabilities.rs:13-16`); `ProcessExecCapability { command, args }` (`capabilities/process_exec_capability.rs:5-11`). `[[...]]` is correct TOML for `Vec`. | OK (see F5) |
| `[language_servers.jdtls] name = "Eclipse JDT Language Server"` | **No `name` field exists** on `LanguageServerManifestEntry` (`:249-265`). Unknown keys are ignored (no `deny_unknown_fields`). | **F4 — ignored, harmless** |

`languages` is not declared in the manifest, but `populate_defaults` auto-discovers `languages/*/config.toml` (`:373-392`), so Java/JSP are picked up. Root registration is correct and marked: `Cargo.toml:269-272` (`kairo/extensions/kairo-java` is a workspace member) and `:309-310` (`kairo_core`/`kairo_ui` aliases); edition is 2024 (`:287`).

---

## 2. Cargo.toml + API version — verdict: 0.7.0 PIN IS CORRECT (F3, hypothesis disproven)

The review prompt flagged `"0.7.0"` as a likely blocker. It is not:

- **Published versions:** crates.io confirms `zed_extension_api` max/default version is **0.7.0** (published 2025-09-12); the local workspace copy is `0.8.0` with `publish = false` (`crates/extension_api/Cargo.toml:3,10`).
- **0.7.0's actual source** (downloaded crate): `src/extension_api.rs:337` generates from `./wit/since_v0.6.0` (there is no `since_v0.7.0` folder — 0.7.0 had no WIT changes). The DAP re-exports at `src/extension_api.rs:22-27` include **all** types the extension uses: `DebugAdapterBinary`, `DebugTaskDefinition`, `StartDebuggingRequestArguments{,Request}`, `TcpArgumentsTemplate`, `TcpArguments`, `resolve_tcp_template`.
- **`serde_json` re-export:** `pub use serde_json;` at `extension_api.rs:11`, so `zed_extension_api::serde_json::{Value, json}` works.
- **`TcpArgumentsTemplate.host`:** in 0.7.0's `since_v0.6.0/dap.wit:29-33` it is `option<u32>`, and in 0.8.0's `since_v0.8.0/dap.wit:37-41` it is `option<ip-address>`. Either way `host: None` is well-typed. (The `u32` → `IpAddress` change landed only in 0.8.0.)
- **Trait signatures match exactly** (0.7.0 `extension_api.rs:198-217` vs target): `get_dap_binary(&mut self, String, DebugTaskDefinition, Option<String>, &Worktree) -> zed::Result<DebugAdapterBinary>` and `dap_request_kind(&mut self, String, Value) -> zed::Result<StartDebuggingRequestArgumentsRequest>`; `zed::Result<T, E = String>` (`:58`). `DebugTaskDefinition.config` is a `String` in both WIT versions, so `serde_json::from_str(&config.config)` is correct.
- **Host load range:** `crates/extension_host/src/wasm_host/wit.rs:60-70` — `wasm_api_version_range` is `0.0.1 ..= since_v0_6_0::MAX_VERSION` for Stable/Preview and `..= latest::MAX_VERSION` for Dev/Nightly; `since_v0_6_0.rs:12` sets `MAX_VERSION = 0.7.0`. So a 0.7.0 module loads on **all** channels (instantiated through the `since_v0_6_0` bindings, `wit.rs:127-135`).
- **Convention:** in-tree `extensions/html/Cargo.toml:17` and `extensions/proto/Cargo.toml:17` both pin `zed_extension_api = "0.7.0"`.

**Conclusion:** no change needed. Do not bump to `0.8.0` — that version is unreleased/`publish = false` and would be rejected on Stable/Preview.

---

## 3. src/kairo_java.rs — verdict: COMPILES

Verified line by line against 0.7.0 and current 0.8.0:

- `use zed::settings::LspSettings;` before `use zed_extension_api::{self as zed, ...};` — valid: Rust item order is irrelevant for name resolution, and `extensions/html/src/html.rs:2-3` uses this exact pattern. `zed::settings` is a public module (`extension_api.rs:5`) re-exporting `LspSettings` from `wit/since_v0.2.0/settings.rs` via `src/settings.rs:3,8`.
- `LspSettings::for_worktree(language_server_id.as_ref(), worktree)?` — `LanguageServerId: AsRef<str>` (`extension_api.rs:574-578`); returns `Result<LspSettings, String>`, so `?` propagates into `zed::Result`. `CommandSettings { path, arguments, env: HashMap<String,String> }` matches `binary.path/arguments/env` usage; `env.into_iter().collect()` produces the WIT `env-vars` (`Vec<(String,String)>`) required by `Command.env`.
- `let-chains` (`if let Some(binary) = ... && let Some(path) = ...`, lines 28-29) require edition 2024 / Rust ≥ 1.88 — the workspace is edition 2024 (`Cargo.toml:287`), so OK.
- `resolve_tcp_template(TcpArgumentsTemplate { port, host: None, timeout })?` — correct field set; `resolve_tcp_template` is a WIT host import re-exported at root. Host `configure_tcp_connection` (`crates/dap/src/dap.rs:27-40`) defaults `host()` to loopback, so attaching to `127.0.0.1:port` behaves as documented.
- `DebugAdapterBinary { command: None, arguments, envs, cwd: None, connection: Some(...), request_args }` — all six fields present and typed correctly. Zed selects TCP transport whenever `connection.is_some()` (`crates/dap/src/transport.rs:92-100`) and does not require `command`, so the "already-running bridge, attach over TCP" model is supported.
- `register_extension!` macro matches `extension_api.rs:289-334` exactly.

No compile errors found. (Not compiled here — no Rust toolchain, per `docs/VERIFICATION.md`.)

---

## 4. Language config.toml — verdict: Java BLOCKER (F1), JSP OK

Language config deserialization is `LanguageConfig::load` → `toml::from_str` (`crates/language_core/src/language_config.rs:154-160`), and `zed-extension test` calls it at `crates/extension_cli/src/main.rs:362` with `?`, so a bad config fails validation and language registration.

| Field | Struct evidence | Java | JSP |
|---|---|---|---|
| `name`, `grammar`, `path_suffixes`, `line_comments`, `debuggers` | `language_config.rs:39,81,147,235` | OK | OK (`autoclose_before` is `String`, `:73` — OK) |
| `brackets` with `start/end/close/newline/not_in` | `BracketPair { start, end, close, surround(default), newline }` (`:460-473`) + `BracketPairContent.not_in` (`:431-437`) | OK | OK |
| `block_comment` | Custom `Deserialize` (`:329-366`): new table form requires **all four** of `start`, `end`, `prefix`, `tab_size`; `tab_size` has **no serde default** (`:341`). Old form is `[start, end]`. | **FAIL** — `{ start = "/*", prefix = "* ", end = " */" }` has only 3 fields; matches neither variant → `data did not match any variant of untagged enum BlockCommentConfigHelper`. | OK — has `tab_size = 0` |

Every real in-tree example includes `tab_size` (e.g. `extensions/html/languages/html/config.toml:5`: `block_comment = { start = "<!--", prefix = "", end = "-->", tab_size = 0 }`).

**Fix F1:** change line 5 of `kairo/extensions/kairo-java/languages/java/config.toml` to:

```toml
block_comment = { start = "/*", prefix = "* ", end = " */", tab_size = 0 }
```

(`tab_size = 1` is also common for C-style comments — `extensions/glsl/languages/glsl/config.toml:17` — but 0 is safe and matches the JSP file's convention.)

---

## 5. tree-sitter queries — verdict: Java BLOCKER (F2), JSP OK

`zed-extension test` compiles every query with `Query::new(grammar, &query_source)?` (`crates/extension_cli/src/main.rs:406-416`); an unknown node name returns `QueryError::InvalidNodeType` and aborts.

**Java `highlights.scm`** — checked every named node and `name:` field against `src/node-types.json` at pinned commit `e10607b45ff745f5f876bfa3e94fbcc6b44bdc11`:
- Valid: `line_comment`, `block_comment`, `string_literal`, `character_literal`, `true`, `false`, `null_literal`, `decimal_integer_literal`, `hex_integer_literal`, `identifier`, `type_identifier`; `name` is a real field on `class_declaration`, `interface_declaration`, `enum_declaration`, `method_declaration`; all 34 keyword literals in the `[...] @keyword` list exist as anonymous nodes.
- **Invalid: line 10 `(floating_point_literal) @number`.** The grammar defines `decimal_floating_point_literal` and `hex_floating_point_literal` only; no `floating_point_literal` node (named, anonymous, or supertype).

**Fix F2:** replace line 10 of `kairo/extensions/kairo-java/languages/java/highlights.scm` with:

```scm
(decimal_floating_point_literal) @number
(hex_floating_point_literal) @number
```

Also consider adding `(octal_integer_literal) @number` / `(binary_integer_literal) @number` for completeness. No other Java highlight issues.

**JSP queries** — all 16 named nodes in `highlights.scm` (`comment`, `directive_name`, `directive_modifier`, `directive_dynamic_argument_value`, `attribute_name`, `attribute_value`, `quoted_attribute_value`, `tag_name`, `start_tag`, `end_tag`, `self_closing_tag`, `text`, `jsp`, `interpolation`, `raw_text`, `erroneous_end_tag_name`) and both nodes in `injections.scm` (`jsp`, `interpolation`) **exist** in `merico-ai/tree-sitter-jsp` at pinned commit `e02ba40fefd64776b56e0d19d7771e6aadab3ec4`. The `(#set! injection.language "java")` directive is a standard Zed injection pragma. No issues.

---

## 6. debug_adapter_schemas/java.json — verdict: VALID

`crates/extension_cli/src/main.rs:499-531` only requires the file to be parseable JSON (`serde_json::Value::from_str`); `crates/debug_adapter_extension/src/extension_dap_adapter.rs:33-45` likewise loads it as an opaque `Value`. The file is well-formed JSON with a conventional draft-07 object schema (`$schema`, `title`, `type`, `properties`, `required`, `additionalProperties`), a `"request"` enum of `["attach"]`, and `port` bounded 1–65535. Consistent with the extension's `get_dap_binary` (attach, port from config) and `[debug_adapters.java]` + `debuggers = ["java"]`. No issues.

---

## 7. tasks.json — verdict: VALID

Validated against `TaskTemplate` in `crates/task/src/task_template.rs:22-81`: `label`, `command`, `args`, `cwd`, `reveal`, `hide` are all real fields; `"always"` is `RevealStrategy::Always` (`:103-111`), `"on_success"` is `HideStrategy::OnSuccess` (`:116-124`), both `#[serde(rename_all = "snake_case")]`. `TaskTemplates` is a newtype over `Vec<TaskTemplate>` (`:141`), so the top-level array is correct, and `FILE_NAME = "tasks.json"` (`:144`) matches `.zed/tasks.json`. `$ZED_WORKTREE_ROOT` is a valid Zed task variable (used verbatim as `cwd` in Zed's own bootstrap task, `crates/zed/src/zed.rs:2537-2538`). All three tasks deserialize cleanly.

---

## Compile / load verdict

**Current state: WILL NOT PASS `zed-extension test`; Java language will not load; Java highlighting will not work.**

- `cargo build -p kairo_java --target wasm32-wasip2`: expected to **succeed** (Rust source and 0.7.0 API usage are all valid — §2, §3).
- `zed-extension --source-dir kairo/extensions/kairo-java ...` (test/validate): expected to **fail twice** — `LanguageConfig::load` on `languages/java/config.toml` (F1, `extension_cli/main.rs:362`) and `Query::new` on `languages/java/highlights.scm` (F2, `:416`).
- At runtime: JSP language, JDT LS attachment, and the `java` DAP adapter registration are structurally sound; the Java language config failure prevents the Java language from registering, and the query failure would disable Java highlighting even if the config were fixed.
- After applying the two one-line fixes (F1, F2), the extension is expected to compile, pass `zed-extension test`, and load on Stable, Preview, Dev, and Nightly. F4–F6 are cosmetic/doc-only and do not block.
