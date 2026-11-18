# 03 — Git / Upstream-Sync Safety Review

- **Review date:** 2026-11-19
- **Reviewer:** sync-safety subagent (read-only; this report is the only file written)
- **Branch under review:** `kairo/integration` @ `5da1d9f`
- **Base:** `main` @ `846ff80` (`origin/main`; merge-base with HEAD is also `846ff80`)
- **Remotes:** `origin` = `Qioooba/zed` fork, `upstream` = `zed-industries/zed`
- **Diff reviewed:** `git diff main..HEAD` — 40 files, 5095 insertions, **0 deletions** of upstream content.

---

## Executive summary — top risks

| # | Risk | Severity | Detail |
|---|------|----------|--------|
| R1 | `SYNC-STRATEGY.md` verification command **omits `settings_content.rs`**, the one file with 2 of the 12 markers. A rebase could silently lose the settings schema hook. | **High** | §2.2 / §2.3 below |
| R2 | **`Cargo.lock` is tracked but not updated** despite 3 new workspace members; no lockfile commit exists, contradicting the documented "separate commit" policy. First `cargo` run on a Rust machine creates uncommitted churn that will pollute a rebase. | **High** | §2.4 / §4 |
| R3 | `initialize_panels` (3 insertions, not 1) and the `SettingsContent` field are **high-churn upstream hotspots**; strategy undercounts/handwaves them. | **Medium-High** | §2.1, §6.3, §6.4 |
| R4 | `use kairo_ui::KairoPanel;` in `zed.rs` is **out of rustfmt import order**; `cargo fmt --check` would likely fail (unverified — no Rust toolchain here). | **Medium** | §2.1, §6 step 7 |
| R5 | Strategy's factual claims about marker counts/positions are wrong in 3 places (2 vs 3 members; 1 vs 3 `zed.rs` lines; "sorted section"). All 12 real insertions *are* tagged, but the docs cannot be trusted as a rebase checklist as written. | **Medium** | §2.2 |
| R6 | All 6 upstream edits are bundled in a single commit (`f583d5f`); the "document at the time" rule was not honored. This is recoverable but makes per-edit cherry-picking coarse. | **Low-Medium** | §4 |

Good news: every new file is under `kairo/`, there are **no** unmarked kairo edits in upstream files, no accidental deletions, no generated junk, no `G:\spaces\kairo-ide` user files, and the `README.md` change is exactly the AGENTS.md-required two lines.

---

## 1. Task 1 — Upstream file/line inventory and marker verification

### 1.1 Complete list of modified upstream files

Only **6** upstream files are touched; every other changed path is under `kairo/`.

| Upstream file | Insertions | `kairo: integration` markers present? |
|---|---|---|
| `Cargo.toml` (root) | 3 workspace members + 1 marker comment + 2 dependency aliases | Yes — marker comment at L269 covers the 3 member paths; both aliases inline-tagged (L309, L310) |
| `README.md` | 2 review lines + 1 blank line | N/A — harness-mandated, not a kairo integration marker (strategy policy = `n/a`) |
| `crates/settings_content/src/settings_content.rs` | field block (4 lines) + `KairoSettingsContent` struct block (31 lines) | Yes — `// kairo: integration` at L202 and L1098 |
| `crates/zed/Cargo.toml` | 2 dependency lines | Yes — inline at L205, L206 |
| `crates/zed/src/main.rs` | 2 init lines | Yes — inline at L777, L778 |
| `crates/zed/src/zed.rs` | import + panel load + join arm (3 lines) | Yes — inline at L91, L784, L809 |

**Total: 12 grep markers** (`rg -n "kairo: integration"` across the 5 non-README upstream files reproduces exactly these).

### 1.2 Line-by-line verification

**`Cargo.toml` (root)**
- Members block (current L269–L273):
  ```toml
      # kairo: integration
      "kairo/crates/kairo_core",
      "kairo/crates/kairo_ui",
      "kairo/extensions/kairo-java",
  ```
  Marker present as a **block comment**. The three path lines are **not individually tagged**.
- Dependency aliases (L309–L310):
  ```toml
  kairo_core = { path = "kairo/crates/kairo_core" } # kairo: integration
  kairo_ui = { path = "kairo/crates/kairo_ui" } # kairo: integration
  ```
  Both tagged inline. Placed after `audio`, before `bench_metrics` — **not alphabetically sorted** (see §2.2).

**`README.md`** (top of file)
  ```markdown
  > [!IMPORTANT]
  > Remove this line to confirm you've reviewed this PR before submitting.

  # Zed
  ```
  Exactly the two AGENTS.md lines plus a separating blank line. No other change. ✅

**`crates/settings_content/src/settings_content.rs`**
- In `pub struct SettingsContent` (L200–L203): doc comment, `// kairo: integration`, `pub kairo: Option<KairoSettingsContent>,`, inserted between `call_hierarchy` and `git_panel`. Tagged. ✅
- Standalone `pub struct KairoSettingsContent` (L1096–L1126 region): doc comment, `// kairo: integration`, `#[with_fallible_options]`, derives, 12 `Option<...>` fields. Tagged. ✅
- No `assets/settings/default.json` edit — confirmed absent from the diff. ✅
- Cross-check: `kairo_core/src/settings.rs:44` reads `content.kairo.as_ref()`, so the field name and `Option<...>` shape are consistent with the schema. ✅

**`crates/zed/Cargo.toml`** (L205–L206): both lines tagged, inserted between `svg_preview` and `sysinfo` (again **not** alphabetically sorted — `kairo_*` belongs near `k`). ✅ for marker.

**`crates/zed/src/main.rs`** (L777–L778): both `::init(cx)` lines tagged, immediately after `terminal_view::init(cx);`. ✅

**`crates/zed/src/zed.rs`**:
- L91 `use kairo_ui::KairoPanel; // kairo: integration` — tagged, but placed between `terminal_view` and `theme` imports, i.e. **out of sorted order** in a contiguous rustfmt group.
- L784 `let kairo_panel = KairoPanel::load(...); // kairo: integration` — tagged, beside other `*_panel` loads.
- L809 `add_panel_when_ready(kairo_panel, ...), // kairo: integration` — tagged, inside `futures::join!`.

### 1.3 Flagged unmarked / weakly-marked upstream edits

1. **`Cargo.toml` member path lines (L270–L272)** are the only kairo insertions without an inline `# kairo: integration` marker. They are recoverable via the single block comment immediately above them, so risk is low, but this deviates from the literal "every insertion carries a marker" convention and from the strategy's "each tagged" wording.
2. **`README.md` L1–L2** are unmarked, but intentionally so (harness rule, not a kairo edit). Not a finding.
3. **No other unmarked upstream edit exists.** Every `+` line in the 6 upstream files is accounted for above.

---

## 2. Task 2 — Merge-conflict risk and SYNC-STRATEGY accuracy

### 2.1 Per-edit conflict risk

| Edit | Likely upstream churn | Risk | Notes |
|---|---|---|---|
| Root `Cargo.toml` `members` (3 paths + marker) | New crates/extensions added most releases; occasional re-sorting | **Medium** | Re-add the 3 `kairo/...` paths under a fresh marker at the tail of a stable section. Pure additions rarely hard-conflict. |
| Root `Cargo.toml` dependency aliases (2) | Workspace deps added/removed regularly | **Medium** | Inline markers make reapplication mechanical. Section is not actually sorted, so a `taplo`/`cargo sort` pass upstream could move lines (churn, not semantic conflict). |
| `settings_content.rs` `SettingsContent` field | Settings struct grows in nearly every release | **Medium-High** | Inserting in the middle (between `call_hierarchy` and `git_panel`) is a classic conflict anchor. See §6.4. |
| `settings_content.rs` `KairoSettingsContent` struct | Low — new standalone item between two other setting structs | **Low-Medium** | No conflict expected; watch for macro/derive convention drift (compile fix in a kairo-owned design, but defined upstream here). |
| `crates/zed/Cargo.toml` deps (2) | Dependencies change often | **Medium** | Not in sorted position → possible reformat churn. |
| `main.rs` init lines (2) | The `::init(cx)` block is edited frequently as modules are added | **High** | Re-add next to whatever stable init call exists; keep `kairo_core` before `kairo_ui`. |
| `zed.rs` import (1) | Imports are touched by most upstream UI work; rustfmt reorders them | **Medium** | Marker travels with the line even if rustfmt moves it. Current placement is already wrong (§2.1 R4). |
| `zed.rs` `initialize_panels` (2) | Panels are added/removed/reordered; helper signature can change | **High** | See §6.3 for the explicit refactor playbook. |

### 2.2 Accuracy of `SYNC-STRATEGY.md` claims

| Claim (line) | Verdict | Evidence / correction |
|---|---|---|
| Root `Cargo.toml`: "exactly 2 `members` + 2 dependency alias lines, each tagged" (L20) | ❌ **Inaccurate** | There are **3** members (`kairo_core`, `kairo_ui`, `kairo-java`); only the block comment is tagged, the 3 path lines are not individually tagged. |
| Root `Cargo.toml`: "placed in the sorted section" (L20) | ❌ **Misleading** | Dependency aliases are inserted after `audio`, before `bench_metrics` — not alphabetical. `crates/zed/Cargo.toml` deps are likewise out of order (`kairo_*` between `svg_preview` and `sysinfo`). |
| `settings_content.rs`: "2 tagged insertions ... No `default.json` edit" (L22) | ✅ Accurate | Both insertions tagged; `default.json` untouched. (Minor: the field insertion is *between* `call_hierarchy` and `git_panel` — document the anchor.) |
| `crates/zed/Cargo.toml`: "2 dependency lines, tagged" (L23) | ✅ Accurate | — |
| `main.rs`: "2 init lines next to `terminal_view::init(cx)`, tagged" (L24) | ✅ Accurate | — |
| `zed.rs`: "**1** panel-load line in `initialize_panels`, tagged" (L25) | ❌ **Inaccurate** | There are **3** tagged insertions: the `use` import (L91), the `KairoPanel::load` call (L784), and the `add_panel_when_ready(...)` arm (L809). All 3 are tagged. |
| `default.json` / keymaps / `crates/project` / `crates/languages` / `extensions` "Do not edit" (L26–L28) | ✅ Accurate | None appear in the diff. |
| `README.md` policy (L29) | ✅ Accurate | Exactly the two review lines. |
| Tagging convention (L31–L37) | ⚠️ Mostly accurate | Holds for 11 of 12 insertions; see §1.3 for the members-block exception. |
| Verify block: `rg "kairo: integration" Cargo.toml crates/zed/Cargo.toml crates/zed/src/main.rs crates/zed/src/zed.rs` (L42) | ❌ **Dangerous omission** | **`crates/settings_content/src/settings_content.rs` is missing** from that command — 2 of 12 markers would be invisible after a rebase. Correct command in §6 step 7. |
| `Cargo.lock`: "Commit Cargo's mechanical diff; resolve by regeneration on rebase" (L21, L51) | ❌ **Not honored** | No `Cargo.lock` change exists in the branch. See §2.4. |
| "Document any unavoidable upstream edit in `PROGRESS.md` at the time it is made" (L52) | ⚠️ Not honored | Upstream edits landed in `f583d5f`, which does not touch `PROGRESS.md`; the decisions log was written earlier in `40450f6`. |
| `PROGRESS.md` L39 / L73 "2 lines" for settings | ⚠️ Imprecise | The two *insertions* are 4 and 31 lines; "2 tagged insertions" is correct, "2 lines" is not. |

### 2.3 Additional sync-safety observations

- **Fork vs upstream base:** local `main` tracks `origin/main` (`Qioooba/zed`), yet the strategy calls it "pristine upstream". Today `main == upstream/main` is unverified after a fetch; rebase directly onto `upstream/main` (as the strategy says) rather than trusting local `main`. Verify with `git diff --stat upstream/main..main` after fetching.
- **Marker grep should also assert an exact count** (12), otherwise a rebase that drops one marker passes silently.
- No edits to `assets/**`, keymaps, `crates/project/**`, `crates/languages/**`, or upstream `extensions/**` — the "zero upstream language/DAP edits" design goal is genuinely met.

### 2.4 `Cargo.lock` gap

`Cargo.lock` is tracked in the repo (`git ls-files Cargo.lock`) but **unchanged** in this branch. Adding `kairo_core`, `kairo_ui`, and `kairo-java` as workspace members necessarily adds `[[package]]` entries and dependency edges. Consequences:
- The first `cargo check` on a Rust-enabled machine will rewrite `Cargo.lock`, producing an uncommitted modification (the branch was never compiled — `PROGRESS.md:52` confirms no toolchain).
- That churn will interact with any upstream `Cargo.lock` changes during a rebase.
- The strategy's "generated files in a separate commit" rule has no corresponding commit.

This must be handled explicitly in the rebase runbook (§6, steps 1 and 8).

---

## 3. Task 3 — New-file location check

`git diff --name-status main..HEAD` lists **34 added files**; every one is under `kairo/`:

- `kairo/README.md`
- `kairo/crates/kairo_core/**` (Cargo.toml + 7 `.rs`)
- `kairo/crates/kairo_ui/**` (Cargo.toml + 2 `.rs`)
- `kairo/docs/**` (5 design docs + 5 analysis docs + `PROGRESS.md`, `SYNC-STRATEGY.md`, `VERIFICATION.md`, plus this new untracked `review/` dir)
- `kairo/extensions/kairo-java/**` (Cargo.toml, README, extension.toml, DAP schema, language configs, `.scm` queries, 1 `.rs`)
- `kairo/fixtures/legacy-sample/.zed/tasks.json`
- `kairo/scripts/verify.ps1`

✅ **Nothing was added outside `kairo/`.** No absolute-path or `G:\spaces\kairo-ide` files were added (those paths appear only as *string values* inside `PROGRESS.md` / `VERIFICATION.md` configuration examples, which is harmless).

Note: `kairo/docs/review/` currently shows as untracked (`??`) — expected, since this report family is being produced outside the integration commits. It should either be committed deliberately with the other docs or left untracked consistently; it will not affect a rebase.

---

## 4. Task 4 — Commit hygiene

Commits (`git log --oneline main..HEAD`), oldest first:

| Commit | Scope | Verdict |
|---|---|---|
| `40450f6` docs: add full Kairo IDE analysis and Zed integration design | 10 files, all `kairo/docs/**` + `kairo/README.md` | ✅ Coherent docs-only change. |
| `f583d5f` feat: add Kairo legacy Java/JSP tooling integration for Zed | 28 files: all 6 upstream files + all new crates/extension/fixture | ⚠️ Coherent as "initial implementation", but bundles every upstream edit into one commit and includes no `Cargo.lock` commit. Strategy's "document at the time" rule not satisfied here. |
| `6c25f34` fix: wire StartServer action to server start and clean up panel state use | 2 files, both `kairo_ui` | ✅ Single fix. |
| `8bacafc` docs: record real Tomcat 6 verification; fix DTOs to match observed wire format | `VERIFICATION.md` + 3 code files | ⚠️ Mixed docs + code fix; the `docs:` prefix undersells the DTO correction. Low severity. |
| `12ef071` feat: poll Tomcat logs into the Kairo panel with GPUI timers | `service.rs` only | ✅ Single feature. |
| `5da1d9f` chore: add verification script and update progress with real E2E evidence | `PROGRESS.md` + `verify.ps1` | ✅ Coherent. |

Checks:
- **One logical change per commit:** mostly yes; two ⚠️ items above, none blocking.
- **No accidental deletions:** upstream files are insertion-only; all deletions in `--stat` are internal kairo refactors (`domain.rs`, `service.rs`, `kairo_panel.rs`, `kairo_ui.rs`, `PROGRESS.md`). ✅
- **No generated junk:** added files are source/config/docs only — no binaries, no `target/`, no build artifacts. ✅ (The opposite problem: missing generated `Cargo.lock`, §2.4.)
- **No user files from `G:\spaces\kairo-ide`:** all paths are repo-relative under `kairo/`. ✅
- **Local identity** is set (`Kairo Integration <kairo-integration@local>`); no upstream commits were amended or squashed. ✅

---

## 5. Task 5 — `README.md` vs AGENTS.md HARD RULE

AGENTS.md requires: when source files are modified, prepend exactly

```
> [!IMPORTANT]
> Remove this line to confirm you've reviewed this PR before submitting.
```

as the first two lines of `README.md`, if not already present; never remove them.

**Finding:** `README.md` diff is exactly those two lines plus one blank separator, inserted above `# Zed`. Nothing else changed. ✅ Fully compliant. This must be preserved through every rebase (§6.5).

---

## 6. Task 6 — Concrete rebase runbook

All 6 upstream edits live in **one** commit (`f583d5f`). Later commits touch only `kairo/**`, so once `f583d5f` is replayed successfully the rest should apply cleanly (upstream never touches `kairo/`).

### Step 0 — Pre-flight (Rust machine required)

```powershell
cd E:\github\zed
git status --short --branch          # expect only "?? kairo/docs/review/"
git fetch upstream
git merge-base --is-ancestor upstream/main kairo/integration; echo $LASTEXITCODE
#   0 = already up to date, nothing to rebase. 1 = continue.
git diff --stat upstream/main..main    # confirm local main is still pristine
```

### Step 1 — Deal with `Cargo.lock` BEFORE rebasing (closes R2)

```powershell
cargo check -p kairo_core -p kairo_ui -p zed
git add Cargo.lock
git commit -m "chore: regenerate Cargo.lock for Kairo workspace members"
```
This satisfies the "generated files in a separate commit" policy. If you skip this, re-run it after the rebase and commit the lockfile as a new tip commit instead.

### Step 2 — Start the rebase

```powershell
git switch kairo/integration
git rebase upstream/main
```

Expect conflicts while replaying `f583d5f` in: `Cargo.toml`, `crates/zed/Cargo.toml`, `crates/settings_content/src/settings_content.rs`, `crates/zed/src/main.rs`, `crates/zed/src/zed.rs` (and possibly `README.md` if upstream edited its header).

**General rule:** resolve every conflict by taking the **upstream (theirs)** side of shared code, then re-adding *only* the tagged kairo lines. Never delete an upstream line. Quick way for a badly-conflicted file:

```powershell
git checkout --theirs Cargo.toml crates/zed/Cargo.toml crates/settings_content/src/settings_content.rs crates/zed/src/main.rs crates/zed/src/zed.rs README.md
```
then re-apply the snippets below.

### Step 3 — `Cargo.toml`

Re-add under a stable section (keep upstream's content intact):

```toml
    # kairo: integration
    "kairo/crates/kairo_core",
    "kairo/crates/kairo_ui",
    "kairo/extensions/kairo-java",
```
and in `[workspace.dependencies]`:
```toml
kairo_core = { path = "kairo/crates/kairo_core" } # kairo: integration
kairo_ui   = { path = "kairo/crates/kairo_ui" }   # kairo: integration
```
Bonus: fix R5 by tagging each member line inline too (`"kairo/crates/kairo_core", # kairo: integration`) and update the strategy's "2 members" claim to 3.

### Step 4 — `crates/zed/Cargo.toml`

Add to `[dependencies]` (ideally in sorted position to avoid future churn):
```toml
kairo_core.workspace = true # kairo: integration
kairo_ui.workspace = true # kairo: integration
```

### Step 5 — `crates/settings_content/src/settings_content.rs` (includes "struct gains fields upstream")

Re-add, in `pub struct SettingsContent`:
```rust
    /// Kairo integration settings for legacy JDK 6 / Tomcat 6 / JSP projects.
    // kairo: integration
    pub kairo: Option<KairoSettingsContent>,
```
and re-add the whole `KairoSettingsContent` struct (12 `Option<...>` fields, `#[with_fallible_options]`, derives `Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, PartialEq`) next to the other setting-content structs.

**If `SettingsContent` gained fields upstream:**
1. Keep **both** upstream's new fields and the `kairo` field. Git will usually present them in the same hunk; accept all lines and delete only conflict markers.
2. Keep the field typed `Option<KairoSettingsContent>`. `kairo_core/src/settings.rs:44` calls `content.kairo.as_ref()`; changing the shape requires a matching edit in that kairo-owned file.
3. If upstream changed schema conventions (e.g. renamed `with_fallible_options`, changed `MergeFrom`/`JsonSchema` derive requirements, or made all fields non-`Option`), update the `KairoSettingsContent` definition in this same commit so it still compiles. This is a kairo-owned design change expressed at an upstream insertion point, not a reason to edit other upstream files.
4. Never add these defaults to `assets/settings/default.json`; registration already happens via `KairoSettings::register` in `kairo_core/src/kairo_core.rs:16`.

### Step 6 — `crates/zed/src/main.rs`

Re-add in the init block (use whatever stable `::init(cx)` call upstream now has as the anchor; currently right after `terminal_view::init(cx);`):
```rust
        kairo_core::init(cx); // kairo: integration
        kairo_ui::init(cx); // kairo: integration
```
Keep `kairo_core` first — it installs `KairoService` as a global (`service::init_global`), which the actions registered by `kairo_ui::init` later resolve.

### Step 7 — `crates/zed/src/zed.rs` (includes "`initialize_panels` changes shape")

Re-add all **three** lines:
```rust
use kairo_ui::KairoPanel; // kairo: integration
...
        let kairo_panel = KairoPanel::load(workspace_handle.clone(), cx.clone()); // kairo: integration
...
            add_panel_when_ready(kairo_panel, workspace_handle.clone(), cx.clone()), // kairo: integration
```

**If `initialize_panels` changed shape:**
- **Panels added/removed/reordered upstream:** keep every upstream `let *_panel = ...::load(...)` and every upstream `add_panel_when_ready(...)` / `join!` arm; insert the kairo load and kairo arm alongside, mirroring the sibling panel exactly.
- **`add_panel_when_ready` signature/bound changed:** match the new call pattern used by `terminal_panel`. `KairoPanel::load` currently returns a plain `impl Future<Output = anyhow::Result<Entity<KairoPanel>>>`; if upstream now requires `Task<...>`, either (a) wrap the call using the `cx`/`window` already captured by the `cx.spawn_in` closure, or (b) preferably change `KairoPanel::load` in `kairo_panel.rs` to return `Task<anyhow::Result<Entity<Self>>>` via `AsyncWindowContext::spawn`, matching other panels. Option (b) is kairo-owned and avoids future churn.
- **`initialize_panels` refactored (e.g. panel list/registry instead of `join!`):** register `KairoPanel` through the new mechanism, using its `KairoPanel::load` loader and its identifiers `persistent_name() == "KairoPanel"`, `panel_key() == "kairo_panel"` (`kairo_panel.rs:56-62`). Do not edit any other upstream panel file.
- **Function moved/renamed:** move the tagged lines to the new location and update the stale comment in `kairo_ui/src/kairo_ui.rs:13` ("one tagged upstream line" → now three).
- **`Panel` trait gained required methods:** implement them in `kairo_panel.rs` (`impl Panel for KairoPanel`) — a compile fix, not a rebase conflict.
- **Rustfmt:** place the `use kairo_ui::KairoPanel;` line in **alphabetical order** within its import group (currently it sits between `terminal_view` and `theme`, which is wrong). Keep the trailing marker comment — it survives auto-reordering.

### Step 8 — `README.md`

If upstream touched the header, keep upstream's content but ensure the first two lines remain exactly:
```markdown
> [!IMPORTANT]
> Remove this line to confirm you've reviewed this PR before submitting.
```
**Never delete them** (AGENTS.md HARD RULE).

### Step 9 — Finish and verify

```powershell
git add Cargo.toml crates/zed/Cargo.toml crates/settings_content/src/settings_content.rs crates/zed/src/main.rs crates/zed/src/zed.rs README.md
git rebase --continue
# repeat conflict resolution if any later commit also conflicts; then:

rg -n "kairo: integration" Cargo.toml crates/zed/Cargo.toml crates/settings_content/src/settings_content.rs crates/zed/src/main.rs crates/zed/src/zed.rs
#   MUST print 12 matches: 3 + 2 + 2 + 2 + 3
git diff --name-only upstream/main..HEAD
#   MUST be exactly the 6 upstream files + kairo/** (+ Cargo.lock if committed in step 1)
git diff --check
cargo check -p kairo_core -p kairo_ui -p zed
cargo build -p kairo_java --target wasm32-wasip2
cargo fmt --all -- --check
./script/clippy     # from a Nix devShell on Windows; pwsh cannot run this bash script directly
git status --short  # Cargo.lock should be clean; commit it separately if regenerated here
```

If `cargo check` rewrites `Cargo.lock` again after the rebase, resolve upstream lock changes by regeneration (`cargo check` / `cargo generate-lockfile`) and keep upstream's unrelated package entries; commit the result in its own `chore:` commit.

### Step 10 — Update docs to match reality

While the rebase is fresh, fix the inaccuracies from §2.2 in `kairo/docs/SYNC-STRATEGY.md`:
- "2 members" → "3 members"; note the block-comment marker (or tag each path).
- Remove "sorted section" wording / actually sort the alias lines.
- `zed.rs`: "1 panel-load line" → "3 tagged lines (import, load, join arm)".
- Add `crates/settings_content/src/settings_content.rs` to the `rg` verification command and assert a count of 12.
- Record the real `Cargo.lock` handling.
- In `PROGRESS.md`, change "2 lines" to "2 tagged insertions" and note the post-rebase Rust verification results.

---

## 7. Reviewer conclusion

The integration is structurally sync-safe: all new code is isolated under `kairo/`, upstream edits are minimal (6 files, insertion-only), and every kairo edit carries a grep marker. The blockers are **documentation/process**, not code placement: the verification command is missing the settings file, the `Cargo.lock` policy is unfulfilled, three SYNC-STRATEGY line counts are wrong, and the two high-churn hotspots (`SettingsContent` field, `initialize_panels`) need the explicit playbook above before the next upstream rebase. Apply §6 step 10 to make the strategy trustworthy as a rebase checklist.
