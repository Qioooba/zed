# Round 1 — Reviewer 8: UX review (legacy-JSP maintainer persona)

> **Reviewer stance.** I evaluated the panel as the actual Kairo persona: a backend/ops engineer
> maintaining a 2008-era Servlet/JSP app on a **2 vCPU / 4 GB Windows 10 cloud desktop**, behind an
> intranet, with **JDK 6, Tomcat 6, mixed GBK/UTF-8 files**, no admin rights, and a Mac laptop for
> travel (§1.2 of `01-kairo-philosophy.md`). He does not keep devtools open, cannot install things,
> and has been burned before by "the IDE saved my GBK file as UTF-8" and "modern javac silently
> pretended to be Java 6". His question when he glances at the panel is always the same three
> things: **Am I on real JDK 6 / source 1.6? What encoding will this save use? Is Tomcat up?**
>
> **Scope read:** `kairo_panel.rs` (render/layout/buttons), `kairo_ui.rs` (actions), `actions.rs`
> (keybindings), `kairo_core/service.rs` (state/error flow, read for context), `UX-DESIGN.md`,
> `01-kairo-philosophy.md` (ui-spec constraints), `03-kairo-frontend-features.md` (original Kairo
> UX). No source files were modified.

**Overall verdict:** The panel is *honest where it counts* — fail-closed JDK handling reaches the
error line, the 200-line log cap is real, and the service re-renders the panel on state change.
But it fails the persona on three of the seven dimensions: the three critical facts are **not** at
a glance, several safety-critical actions are **only** in the command palette (and two of those
actions are stubs that do nothing but refresh), and a whole class of fail-closed errors is **silent
in the UI** because the early-return paths never call `set_error`. A real user would describe this
panel as "a wall of tiny text with nine identical buttons" and would not trust it for GBK saves.

Severity legend: **BLOCKER** (breaks a core promise), **ERROR** (materially harms the persona),
**WARNING**, **NIT**.

---

## 1. Do the three legacy-critical facts read at a glance?

**Verdict: No. They are present but scattered, tiny, and visually undifferentiated.**

| Fact | Where it is rendered today | Assessment |
|---|---|---|
| Active JDK / source level | "Toolchain Slots" column, one of four `flex_1` columns in the lower block (`kairo_panel.rs:301`): `Compiler JDK: <not set — JDK 6 required for 1.6>` / `Source/Target: 1.6/1.6` | **Buried.** Same `text_xs`-ish weight as every other line; a missing JDK 6 looks identical to a configured one. The most important question the persona has is answered in the fourth column of a cramped row. |
| Encoding | **Twice**, inconsistently: top line shows *detected* encoding via `state.encoding()` (`kairo_panel.rs:237-247`), while the slots block shows *configured* `settings.encoding` (`kairo_panel.rs:205`). | **Confusing.** Two rows both labelled "Encoding" with potentially different values, and no indication which one governs the save. `SaveWithEncoding` in `service.rs:863` uses `settings.encoding`, but the user sees the *detected* value first. |
| Tomcat status | "Servers" column: `{id}: {state} (gen {generation}, http {port})` (`kairo_panel.rs:302-304`) plus the free-form top status line. | **Weak.** When no server exists the column renders just the header `Servers\n` — there is no explicit "Tomcat: STOPPED". The persona cannot tell "stopped" from "agent never connected". |

Additional problems:

- The top status line (`kairo_panel.rs:232`, `text_sm`) is the only always-human-readable line and
  it is generic ("Kairo agent is not running" / "Kairo operation failed"). It does not carry the
  three facts that UX-DESIGN §2 says must be a permanent status surface.
- The panel does not implement the ui-spec's **4-state view** or "one primary action" — it dumps
  every field with equal weight. This is the opposite of "status bar a glance away" in
  `01-kairo-philosophy.md` §5.1/§5.4 (status bar constantly surfaces `☕ JDK 6 … · Source 6`,
  encoding, EOL, Tomcat port).
- `default_size()` is **280 px** (`kairo_panel.rs:103-105`) for a bottom dock that must contain
  status + error + 9 buttons + 4 columns + a 200-line log. On a low-res 4 GB box this will be
  cramped and the lower columns/log will be clipped. The original Kairo used *separate* Build,
  Servers, Deployments and Log views (§3.6–3.7 of `03-kairo-frontend-features.md`) for this reason.
- No color is used for state at all. This satisfies "no color-only semantics" trivially, but it
  also throws away a useful redundant cue: a disabled compiler JDK should be **text-labelled**
  *and* toned, not rendered as plain grey text.

**Finding (ERROR):** No dedicated, labelled status surface for the three critical facts; two
competing Encoding lines; no "Tomcat: STOPPED" state.

---

## 2. Are fail-closed errors visible and actionable, not buried?

**Verdict: The happy fail-closed messages are visible but under-styled; a significant class of
errors is completely silent in the UI. This is the most serious finding in the review.**

What works:

- `KairoService::set_error` (`service.rs:201-205`) writes `last_error` and calls `cx.notify()`.
- Toolchain incompatibility is honest end-to-end: the build poller detects the closed error code
  and writes the explicit message "a real JDK 6 is required for source level 1.6; importing a
  modern JDK is not a substitute" (`service.rs:486-493`). The panel's error line
  (`kairo_panel.rs:249-255`) then shows it. Good — no fake green.
- The debug gate failure also reaches `last_error` (`service.rs:638-642`).

What is broken:

- **BLOCKER — silent early-return errors.** These methods return `Task::ready(Err(...))` *without*
  calling `set_error`:
  - `start_build` — "Kairo runtime agent is not running" / "no active Kairo project — use Kairo:
    Select Project" (`service.rs:405-411`)
  - `deploy` — "agent not running" / "no active Kairo project" / **"no successful Kairo build to
    deploy — run Compile first"** (`service.rs:508-523`)
  - `start_server`, `hot_deploy`, `check_debug_gate`, `scan_workspace`, `open_workspace`, `search`,
    `load_logs` — same pattern.
  Every caller uses `.detach_and_log_err(cx)` (panel buttons at `kairo_panel.rs:261-289`, actions at
  `kairo_ui.rs:23-232`). The `Err` goes **only to the log file**. The user clicks Deploy with no
  build, nothing visibly happens, and the reason is not on screen. On a 4 GB box without devtools
  this is indistinguishable from a hung UI. This directly violates "绝不把失败折叠成空状态"
  (never collapse failure into an empty state) in UX-DESIGN §2/§3 and the "no error may disappear
  into the console" comment in `service.rs:199-200`.
- The error line is `text_xs` with no emphasis, icon, or wrapping (`kairo_panel.rs:249-255`). Build
  failures put the **entire compiler output** into `last_error` (`service.rs:492`) — a long string
  rendered as one unconstrained text node. In a bottom dock it will be clipped horizontally; there
  is no `.whitespace_pre_line()` / `.w_full()`, no "view full output", no dismiss/retry affordance.
- `last_error` is cleared on the next successful refresh/status update, so there is no sticky
  failure history; the persona cannot compare "which build failed and why".
- `select_project` and `search_encoded` are *stubs* (see §3) — they silently refresh instead of
  doing the named operation, which is a fail-closed/honesty problem in its own right: the command
  palette advertises an action that lies about what it does.

**Finding (BLOCKER):** Early-return `Err` paths never reach `last_error`; the most common user
mistakes (Deploy before build, any action with no active project) are invisible.

---

## 3. Button labels, grouping, primary action, panel reachability

**Verdict: Good verb-phrase labels, but no visual hierarchy, no wrapping, missing primary actions,
and two advertised actions that are fake.**

Present buttons (`kairo_panel.rs:256-290`), in order: Compile, Clean Build, Build & Deploy, Deploy,
Hot Deploy, Start Server, Debug, Stop, Refresh.

Positives: verb phrases, not icons (matches UX-DESIGN §2.4); Compile/Start Server are placed early;
Stop is late. All nine have click handlers wired to real service methods.

Problems:

- **ERROR — Missing primary actions from the panel.** The following exist only in the command
  palette (`kairo_ui.rs`) and have **no button**, even though the persona will need them constantly:
  - `SelectProject` (`kairo_ui.rs:166-176`) — and its handler only calls `service.refresh(cx)`! The
    `start_build` error message literally says "use Kairo: Select Project", but that command does
    not select anything, and there is no panel control. This is the single most disorienting gap.
  - `SaveWithEncoding` (`kairo_ui.rs:177-199`) — the **highest-stakes GBK data-safety action** is
    absent from the panel. UX-DESIGN §2/§3.4 requires the encoding line to be a click target that
    opens "Save with encoding"; here the encoding line is plain, unclickable text.
  - `OpenApplication` (`kairo_ui.rs:137-147`) — after starting Tomcat, "see my JSP in the browser"
    is the natural next step; no panel button.
  - `ScanProject` (`kairo_ui.rs:148-165`) — the first-run entry point for a non-standard 2008
    layout; only in the palette. Refresh is not equivalent.
  - `RestartServer`, `ReloadContext`, `ReconnectAgent`, `RefreshStatus` — no panel controls.
    `Restart`/`Reload` matter because hot-deploy honestly reports `restart_required`.
- **ERROR — `SearchEncoded` is fake** (`kairo_ui.rs:200-212`): its comment claims the panel owns a
  query string, but the panel has **no query input** and the handler just calls `refresh()`.
  The action name and doc comment ("Run an encoding-aware search (required for GBK projects)") lie.
  `service.search(query, cx)` exists (`service.rs:937`) and is never called from the UI.
- **WARNING — no hierarchy or grouping.** All nine buttons use the identical `action_button`
  style (`kairo_panel.rs:50-67`: `border_1`, `px_2`, `py_1`). Compile and Stop look the same; there
  is no `ButtonStyle::Filled`/accent on the primary actions, no separator before the destructive
  Stop, and no `flex_wrap()` on the row (`kairo_panel.rs:257`) — nine buttons will overflow on a
  narrow window. No `.tooltip()` on any button, so the persona cannot discover the keybindings or
  what "Debug" does (Java 6 gate) without hovering the palette.
- **WARNING — no disabled/busy state.** Buttons are always enabled and re-entrant: clicking Compile
  twice starts two builds; Deploy/Start are clickable with no agent or no project even though those
  paths are the silent errors from §2.
- **WARNING — drag-to-right is advertised but does nothing.** `position_is_valid` accepts
  `DockPosition::Right` (`kairo_panel.rs:91-93`) but `set_position` is an empty body
  (`kairo_panel.rs:95-101`). UX-DESIGN §1 promises "可拖到右侧". The panel is added to the dock in
  `zed.rs:784/799`, so it is reachable, but the promised repositioning is not implemented.

Reachability summary: the panel itself is fine (added via `workspace.add_panel` alongside the
other panels, toggled by `TogglePanel`), but **most of the Kairo capability surface is not on it**.

---

## 4. Keybindings: sensible and non-colliding?

**Verdict: Fine on the primary Windows box; the "no collisions" claim in the source is false, and
the Mac-travel half of the persona is affected.**

Verified against `assets/keymaps/default-{windows,linux,macos}.json`:

| Chord | Action | Collides? |
|---|---|---|
| `ctrl-alt-shift-j` | Compile | none found |
| `ctrl-alt-shift-m` | Deploy | none found |
| `ctrl-alt-shift-i` | HotDeploy | **Linux:** `edit_prediction::ToggleMenu` (`default-linux.json:38`) |
| `ctrl-alt-shift-v` | StartServer | none found |
| `ctrl-alt-shift-o` | StartDebugServer | **Linux + macOS:** `dev::ResetFrameOverlayStats` (`default-linux.json:943`, `default-macos.json:999`) |
| `ctrl-alt-shift-k` | TogglePanel | none found |

- On **Windows** (the primary persona's machine) none of the six collide — good. But
  `actions.rs:5-6` states "Default chords were chosen from combinations that do not appear in
  Zed's shipped keymaps", which is factually wrong for Linux/macOS. Given Kairo's explicit
  "don't lie / fail closed" culture, a false comment in the security/UX-facing keymap file should
  be corrected.
- The persona explicitly travels with a **Mac laptop** (§1.2), where `ctrl-alt-shift-o` is taken;
  `dev::ResetFrameOverlayStats` is a debug-build action so the practical impact is low, but the
  Linux `ctrl-alt-shift-i` collision with a real, everyday action (`edit_prediction::ToggleMenu`)
  is genuine.
- Mnemonics are arbitrary (J=compile, M=deploy, I=hot-deploy, V=start, O=debug, K=panel). The
  original Kairo exposed *real* chord hints computed from the platform (`kairo-platform.ts:
  toolbarShortcut`, §3.1 of `03-kairo-frontend-features.md`); here the panel shows no hints at all.
- Four-modifier chords plus a **Chinese Windows IME** are a poor combination — Ctrl+Alt is AltGr on
  many layouts and IMEs swallow Ctrl+Alt+Shift sequences. A legacy GBK maintainer typing Chinese
  comments will lose these bindings. Prefer function-key style actions (original Kairo used
  Shift+F10 / Ctrl+F10 / Shift+F9) for the most common operations, or at least offer them.
- Only 6 of 18 actions are bound. **`SaveWithEncoding` has no binding at all** — the
  data-integrity-critical action is the least reachable.

**Finding (ERROR):** False "no collisions" claim; two real cross-platform collisions; no binding
or panel affordance for `SaveWithEncoding`; no chord hints in the UI.

---

## 5. Is the log view bounded and readable?

**Verdict: Bounded — yes. Readable — largely no.**

Bounded (good):

- `MAX_VISIBLE_LOG_LINES = 200` and `.iter().rev().take(200)` (`kairo_panel.rs:16,177-184`);
  the service also requests only 200 lines from the agent (`service.rs:790,819`). This correctly
  implements the ui-spec ban on infinite-scroll log views for 4 GB machines.

Readability problems:

- It is one `SharedString` built by joining lines with `\n` and rendered as a **single default-size
  text node** (`kairo_panel.rs:306-310`). No monospace font, no line numbers, no timestamp/level
  markers, no level filter, no live/paused toggle, no auto-scroll, no load-more control — the
  original Kairo log viewer had all of these (§3.7 of `03-kairo-frontend-features.md`).
- **No scroll container.** In a 280 px bottom dock with status + error + buttons + columns already
  consuming the space, 200 lines in a single `div().flex_1()` will be clipped with no way to reach
  them. Needs `.overflow_y_scroll()` (and a min-height).
- Ordering is inconsistent with expectations: `.rev()` puts **newest at the top** with no label.
  Either render chronologically with auto-scroll-to-bottom, or label it "newest first".
- `start_log_polling` (`service.rs:808-832`) loops every 2 s forever and is never cancelled by
  `stop_server` (`service.rs:706-733`): it keeps polling a stopped server and calling `cx.notify()`
  every 2 s, forcing a full panel re-render indefinitely. On a 2 vCPU box this is wasted work and
  will also fight any scroll position once scrolling is added.

**Finding (WARNING):** Bounded but effectively unreadable/unscrollable; runaway polling after stop.

---

## 6. Does the panel re-render on state change?

**Verdict: Yes — this is solid.**

- `KairoPanel::new` registers `cx.observe(&service, |_, _, cx| cx.notify())` and stores the
  `Subscription` in `_subscriptions` (`kairo_panel.rs:21,30-36`). The subscription lives as long as
  the panel.
- Every service mutation calls `cx.notify()`: `set_error`, `refresh`, `activate_project`,
  `open_workspace`, `scan_workspace`, build polling, `deploy`, `hot_deploy`, `start_server`,
  `stop_server`, log polling, encoding, recode, search (`service.rs`, throughout).
- The `Panel` wiring (`zed.rs:784/799`, `toggle_panel_focus` in `kairo_ui.rs:236-242`) is correct.

Caveats: because log polling notifies every 2 s with no stop condition, the whole panel — all four
joined column strings plus the log string — is rebuilt every 2 s even when the user is not looking
at logs. Once a scrollable log view is added, re-renders must preserve scroll offset (e.g. keep the
scroll handle in a field or restore `.scroll_top()`).

**Finding (NIT):** Re-render works; throttle/scope log notifications and preserve scroll position.

---

## 7. Five concrete UX improvements (highest impact first)

### Improvement 1 — Make every error visible in the UI (BLOCKER)
**Where:** `kairo_core/src/service.rs` + `kairo_ui/src/kairo_panel.rs`.

Add a single helper and route *all* early returns and button/action failures through it:

```rust
fn unavailable(&mut self, message: impl Into<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
    let message = message.into();
    self.set_error(&anyhow!(message.clone()), cx);
    Task::ready(Err(anyhow!(message)))
}
```

Replace every `Task::ready(Err(anyhow!(...)))` in `start_build`, `deploy`, `start_server`,
`check_debug_gate`, `scan_workspace`, etc. with `return self.unavailable(..., cx);`.

Change `action_button` so its handler can surface returned errors instead of relying on
`.detach_and_log_err(cx)`: have it call a shared

```rust
fn run(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut KairoService, &mut Context<KairoService>) -> Task<Result<()>>)
```
that attaches `.map(|result| if let Err(e) = result { KairoService::global(cx).update(cx, |s, cx| s.set_error(&e, cx)); })`,
or simply make the service's `set_error` the only error sink. Give the error line
`.w_full().whitespace_pre_line().text_color(cx.theme().colors().text_error)` **plus** a literal
`ERROR:` prefix (already present) and a sticky "last failure" that survives the next refresh.

### Improvement 2 — Put the missing primary actions on the panel and stop the two fake actions
**Where:** `kairo_panel.rs`, `kairo_ui.rs`.

- Add buttons (with `.tooltip(...)` showing the chord): **Select Project**, **Save with encoding**,
  **Open App**, **Scan**, **Restart**, **Reload**. Group them into labelled rows —
  `div().flex().flex_wrap().gap_2()` for Build (Compile · Clean Build · Build & Deploy · Deploy ·
  Hot Deploy), a separator, then Server (Start Server · Debug · Restart · Reload · Stop · Open App),
  then Tools (Select Project · Scan · Save with encoding · Refresh). Use Zed's `ui::Button` with
  `ButtonStyle::Filled` for Compile / Start Server and `ButtonStyle::Danger` for Stop.
- Make the encoding row a click target dispatching `SaveWithEncoding`, per UX-DESIGN §3.4:
  `.on_click(cx.listener(|_, _, window, cx| window.dispatch_action(&actions::SaveWithEncoding.boxed_clone(), cx)))`.
  Collapse the detected/configured duplication into one labelled line:
  `Encoding: gbk (configured) · detected: gbk — click to save byte-faithfully`.
- Implement `SelectProject` for real: add `KairoService::select_project(id, cx)` (reuse
  `activate_project`) and open a `Picker` (Zed `picker::Picker::new`/`PickerDelegate`) listing
  `state.projects()`, or at minimum cycle through them on repeated invocation. Replace the
  `refresh()`-only handler in `kairo_ui.rs:166-176`.
- Implement `SearchEncoded` for real: add a small query input to the panel and call
  `service.search(query, cx)` (`service.rs:937` already exists). Remove the lying comment.

### Improvement 3 — Add a dedicated "three facts" status strip above everything
**Where:** `kairo_panel.rs:226-255`.

Insert, directly below the status line and above the error/buttons:

```rust
.child(
    div().flex().gap_3().items_center()
        .child(jdk_chip)      // "COMPILER JDK: <not set> — COMPILE DISABLED" or "JDK 6 1.6.0_45"
        .child(source_chip)   // "SOURCE/TARGET 1.6/1.6"
        .child(encoding_chip) // "ENCODING gbk"
        .child(tomcat_chip)   // "TOMCAT STOPPED" / "TOMCAT RUNNING http://127.0.0.1:8080/app"
)
```

Compute each chip from the same data the columns use, but with an **explicit text token**
(`STOPPED`, `NOT SET`, `FAIL-CLOSED`) and a tokenized background from `cx.theme().colors()`
(`warning_background` / `error_background` for bad states) — never colour alone. When
`state.servers()` is empty, render `TOMCAT: STOPPED` instead of an empty column. This is the
4-state/status-surface requirement from the ui-spec and makes the three critical facts readable in
under a second.

### Improvement 4 — Make the bounded log view actually readable and stop runaway polling
**Where:** `kairo_panel.rs:306-310`, `service.rs:706-733,808-832`.

```rust
.child(
    div().flex_1().min_h(px(120.)).overflow_y_scroll()
        .child(div().text_xs().font_family(monospace).whitespace_pre()
            .child(SharedString::from(format!("Tomcat logs (last {MAX_VISIBLE_LOG_LINES}, newest first)\n{logs}"))))
)
```

Add a `Live`/`Paused` toggle field on the panel (skip appending to the rendered string when paused),
a `Load earlier` button calling `service.load_logs(server_id, cx)` (paging, still capped at 200),
and prefix each line with its level text (`INFO`/`WARN`/`ERROR`) so severity is not colour-only.
Cancel the poll task in `stop_server` (`self.poll_task.take();` before returning) so a stopped
server does not re-render the panel every 2 s. Preserve scroll offset across `cx.notify()`
re-renders.

### Improvement 5 — Honest, discoverable keybindings
**Where:** `kairo_core/src/actions.rs`, `kairo_ui/src/kairo_panel.rs`.

- Fix the false "no collisions" comment (`actions.rs:5-6`) to state the real situation, or better,
  rebind `HotDeploy` off `ctrl-alt-shift-i` and `StartDebugServer` off `ctrl-alt-shift-o` to chords
  verified against **all three** `assets/keymaps/default-*.json` files.
- Add a binding for `SaveWithEncoding` (verify it is free on Windows/Linux/macOS first) — it is the
  highest-stakes action and currently has neither a binding nor a panel button.
- Prefer low-modifier alternatives for the top actions (original Kairo used Shift+F10 start,
  Ctrl+F10 update, Shift+F9 debug), which survive Chinese IME layouts.
- Show the chord in each button's `.tooltip()` (and the panel icon tooltip), so muscle-memory users
  can learn the bindings from the panel rather than the palette.

---

## Non-goals / out of scope for this review
I did not evaluate the Go agent, keymap JSON upstream merge strategy, or the WASM/grammar layers;
those are covered by reviewers 1–5. Everything above is scoped to the panel's rendering, action
wiring, keybindings and the UX constraints in the two design documents.

## Bottom line for the triage table
- **BLOCKER:** silent early-return errors (Deploy-before-build, no-project, agent-down) — §2.
- **ERROR:** missing panel actions + `SelectProject`/`SearchEncoded` stubs — §3; false/colliding
  keymaps and unbound `SaveWithEncoding` — §4; no at-a-glance critical-facts strip — §1.
- **WARNING:** unreadable log view and runaway polling — §5; `set_position` no-op — §3; cramped
  280 px default size — §1.
- **NIT:** full-panel re-render every 2 s — §6.
