# Upstream Sync Strategy

Goal: be able to `git rebase` onto new Zed upstream releases with minimal,
mechanical conflict resolution, so future work can be contributed back / kept in
step with the Zed authors.

## Branch model

- `main` — pristine upstream Zed (never committed to by Kairo work).
- `kairo/integration` — the only working branch. All Kairo integration commits
  live here (created from `main` on 2026-11-19).
- When syncing: `git fetch upstream && git rebase upstream/main` from
  `kairo/integration`.

## Ownership map

| Path | Owner | Conflict risk | Policy |
|------|-------|---------------|--------|
| `kairo/**` | Kairo | none (upstream never touches it) | Free to create/modify. |
| `Cargo.toml` (root) | upstream | high | **3** `members` lines (`kairo_core`, `kairo_ui`, `kairo-java` under one `# kairo: integration` block comment) + **2** dependency aliases, each inline-tagged. Aliases are deliberately placed after `audio` (not alphabetically sorted) to keep the diff local. |
| `Cargo.lock` | upstream (generated) | medium | **Pending:** the three new workspace members are not in `Cargo.lock` yet because this machine has no Rust toolchain. The first `cargo` run on a Rust machine must commit the regenerated lockfile in a separate commit before any rebase. |
| `crates/settings_content/src/settings_content.rs` | upstream | medium | 3 tagged insertions: the `kairo` field on `SettingsContent`, the `KairoSettingsContent` struct, and `kairo` in the `flattened_deserialize!` options list. No `default.json` edit. |
| `crates/settings/src/vscode_import.rs` | upstream | low | 1 tagged line: `kairo: None` in the `SettingsContent` literal used by VS Code settings import. |
| `crates/zed/Cargo.toml` | upstream | medium | 2 dependency lines, tagged. |
| `crates/zed/src/main.rs` | upstream | high | 2 init lines next to `terminal_view::init(cx)`, tagged `// kairo: integration`. |
| `crates/zed/src/zed.rs` | upstream | high | 3 tagged lines: import in rustfmt order, panel `load` call, and the `add_panel_when_ready` arm. |
| `assets/settings/default.json` | upstream | very high | **Do not edit.** Register settings from `kairo_core` via `Settings::register`. |
| `assets/keymaps/default-*.json` | upstream | very high | **Do not edit.** Bind keys at runtime via `cx.bind_keys`. |
| `crates/project/**`, `crates/languages/**`, `extensions/**` | upstream | very high | **Do not edit.** Put JSP/DAP/LSP contributions in Kairo-owned files and register through public APIs. |
| `README.md` | upstream + harness rule | n/a | Keep the two `> [!IMPORTANT]` review lines at the top (AGENTS.md forbids removing them). |

## Tagging convention

Every upstream-file insertion carries a grep marker so a rebase can find and
reapply them:

- Rust: `// kairo: integration`
- Cargo/TOML: `# kairo: integration`

Verify after any rebase:

```
rg "kairo: integration" Cargo.toml crates/settings_content/src/settings_content.rs crates/settings/src/vscode_import.rs crates/zed/Cargo.toml crates/zed/src/main.rs crates/zed/src/zed.rs
cargo check -p kairo_core -p kairo_ui -p zed
./script/clippy
```

## Commit discipline

- One logical change per commit, with a short imperative subject.
- Never amend/squash upstream commits; only Kairo-owned commits on this branch.
- Keep generated files (`Cargo.lock`) in a separate commit from hand-written code.
- Document any unavoidable upstream edit in `PROGRESS.md` at the time it is made.
