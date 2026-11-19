# Review 12 — Theme family and panel i18n

## Scope

Two additions to the Kairo integration on branch `kairo/integration`:

1. A valid Zed theme family JSON for the kairo-java extension.
2. A self-contained bilingual (English / Simplified Chinese) string table for
   the Kairo panel, driven by a new `kairo.language` setting.

## Part 1 — Theme family

### Files

- `kairo/extensions/kairo-java/themes/kairo-legacy.json` (new)
- `kairo/extensions/kairo-java/extension.toml` (added
  `themes = ["themes/kairo-legacy.json"]`)

### What was done

- Modelled the file on a real in-repo theme family,
  `assets/themes/one/one.json`, which uses
  `$schema: https://zed.dev/schema/themes/v0.2.0.json`.
- Family `Kairo Legacy`, author `Kairo Integration`, with two themes:
  - `Kairo Legacy Dark` — `appearance: dark`, dark accent `#4a9eff` (the
    accent documented in `docs/analysis/03-kairo-frontend-features.md` §3.16).
  - `Kairo Legacy Light` — `appearance: light`, using a darker blue
    `#005fb8` so the accent keeps contrast on a light surface.
- Each theme carries 139 style keys: borders, surfaces, text, icons, status
  bar, tabs, editor, terminal ANSI palette, version-control/status colors,
  player colors, and a compact syntax map. Only the accent/surface hues were
  changed from the reference theme; the key set is copied verbatim from a
  shipped Zed theme so parsing is known-good.
- Verified against `crates/theme_settings/src/schema.rs`: `ThemeFamilyContent`
  requires `name`, `author`, `themes[]`; each `ThemeContent` requires `name`,
  `appearance`, `style`; every style field is `Option<ThemeColor>` with
  `#[serde(default)]`, so a partial style map would also have been valid.
  A `scales` key is **not** part of `ThemeFamilyContent` (it is derived at
  runtime from `default_color_scales()`), so it was deliberately omitted.
- Verified with Python that the file is valid JSON and that every `#...`
  value is a full 8-digit `#RRGGBBAA` color (two typos were found and fixed).

## Part 2 — i18n

### Files

- `kairo/crates/kairo_core/src/i18n.rs` (new)
- `kairo/crates/kairo_core/src/kairo_core.rs` (`pub mod i18n;`)
- `kairo/crates/kairo_core/src/settings.rs` (`language` field, default `"en"`)
- `crates/settings_content/src/settings_content.rs`
  (`KairoSettingsContent.language: Option<String>`)
- `kairo/crates/kairo_ui/src/kairo_panel.rs` (labels, tooltips, status/error
  lines wired to the string table)
- `kairo/crates/kairo_ui/src/kairo_views.rs` (secondary-view labels wired to
  the string table)

### What was done

- `Lang { En, ZhCn }` resolves from `kairo.language`. Accepted values for
  Chinese: `zh`, `zh-CN`, `zh_CN`, `zh-Hans`, `cn` (case-insensitive,
  underscore normalized to hyphen); everything else falls back to English.
- `KairoStrings` is a `Copy` struct of `&'static str` fields covering all
  action buttons, view tabs, the panel icon tooltip, error/status labels
  (`error_prefix`, `no_agent`, `no_project`,
  `toolchain_incompatible_guidance`, `agent_not_running`,
  `operation_unavailable`, `operation_failed`), empty-state labels, and
  section headings. Two full constant tables (`EN`, `ZH_CN`) back it.
- `pub fn strings(cx: &App) -> KairoStrings` is the single entry point.
- The panel now shows `no_agent`, `no_project`, and the toolchain
  incompatibility guidance as conditional status lines, localizes the error
  prefix, and uses the table for every button, tab, and the icon tooltip.
  The secondary views (Events, Deployments, Maven, SQL, Tests, SVN) use the
  table for their headings, empty states, and hints.

### Settings plumbing note

The task description warned that changing `SettingsContent` requires updates
to the `flattened_deserialize!` options list and the `vscode_import.rs`
literal. On inspection, both enumerate the **top-level** `kairo` section, not
the fields inside `KairoSettingsContent`:

- `crates/settings_content/src/settings_content.rs:421` already has
  `kairo, // kairo: integration` in the options list.
- `crates/settings/src/vscode_import.rs:216` already has
  `kairo: None, // kairo: integration`.

No edits were needed in either place. A search confirmed there are no struct
literals of `KairoSettingsContent` elsewhere that would need the new field.

## Verification

- `cargo fmt -p kairo_core -p kairo_ui` — exit 0.
- `cargo check -p kairo_core -p kairo_ui` — exit 0, no compiler warnings or
  errors (only transient crates.io index warnings on the first run; a
  follow-up `--offline` run finished clean in 2.33 s).
- Theme JSON — valid JSON, all colors are `#RRGGBBAA`, family/themes match the
  requested names and appearances.

## Remaining issues / limitations

- The Kairo service (`kairo_core/src/service.rs`) still writes its status and
  error messages in English (for example, "Kairo agent is not running").
  Those strings are backend-owned and are shown verbatim in the panel's
  status line. Fully localizing them would require threading `Lang` through
  the service, which is beyond the requested scope. The panel-level
  equivalents now exist in the string table and are used for the status and
  error labels.
- The action registration layer (`kairo_ui/src/kairo_ui.rs`) does not render
  text, so it needs no changes. Action names in the command palette remain
  Zed's English action names; Zed has no plugin-based command-label
  replacement.
- The light-theme accent is `#005fb8` rather than `#4a9eff`; this is a
  deliberate contrast choice, not an omission. The documented `#4a9eff`
  accent is used exactly on the dark theme.
- The root `README.md` already contained the required
  `[!IMPORTANT]` review-confirmation block, so no README change was made.
