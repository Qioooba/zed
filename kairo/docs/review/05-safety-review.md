# Round 1 — Safety / correctness review of the Zed-side Kairo integration

> Reviewer: security/correctness auditor. Branch `kairo/integration`.
> Scope: `kairo/crates/kairo_core/src/**`, `kairo/crates/kairo_ui/src/**`,
> `kairo/extensions/kairo-java/src/**`, plus the upstream keymaps, settings schema and
> registration lines. The Go agent was **not** re-verified; this report treats
> `docs/VERIFICATION.md` as authoritative for its behavior.
> Method: static reading only; no files modified except this report. No Rust toolchain is
> available in this environment, so nothing was compiled (see `PROGRESS.md`).

## 0. Verdict

The Zed side is a thin, type-safe client with several things genuinely right: DTOs keep
secrets out, the agent is launched loopback-only with a reaped process tree, Tomcat/JDWP
binding and atomic writes stay in the agent, the log view is bounded, and the debug adapter
is forced to `127.0.0.1`. **But four of the ten audited invariants are not implemented at
all** (fail-closed error surfacing, byte-faithful GBK, event sequence/gap, debug gate), and a
fifth (hot-reload honesty) is only "not dishonest by accident." Two of those — the missing
build-result polling and the missing save hook — are data-integrity/fail-closed blockers:
the first hides `toolchain_incompatible` from the user, the second can silently destroy GBK
source files on save. `PROGRESS.md` line 37 records the GBK decision as if done; it is not.

### Invariant scorecard

| # | Invariant | Result | Where |
|---|-----------|--------|-------|
| 1 | Fail-closed toolchain / real error in status | **FAIL (blocker)** | §1 |
| 2 | Byte-faithful GBK / save hook | **FAIL (blocker, no hook exists)** | §2 |
| 3 | Secrets not exposed via DTO/panel/settings | **PASS** (auth secret plumbing dead, §6) | §3 |
| 4 | Loopback-only + DAP loopback + no client bypass | **PASS** | §4 |
| 5 | Process identity / atomic writes delegated | **PASS** | §5 |
| 6 | Hot-reload honesty (`restart_required`/`compileOnly`) | **NOT IMPLEMENTED** (no false claim today) | §7 |
| 7 | Paged/bounded logs | **PASS** (200-line cap) | §8 |
| 8 | Event sequence / gap handling | **NOT IMPLEMENTED** (stubbed stream) | §9 |
| 9 | `/debug/adapter/status` gate before offering debug | **NOT IMPLEMENTED** | §10 |
| 10 | Keybinding conflicts | **FAIL** (8 collisions with defaults) | §11 |

---

## Severity-ordered findings

### BLOCKER-1 — `toolchain_incompatible` is never surfaced; builds are fire-and-forget

**Invariant 1. Files:** `service.rs:235-257`, `service.rs:38,58,180,208`,
`client.rs:123-126,361-365`, `kairo_ui.rs:17-27`, `kairo_panel.rs:159-161`.

The agent's `POST /api/v1/builds` is asynchronous (`02-kairo-runtime-agent.md` §3 #30,
`VERIFICATION.md` line 32): it returns a queued `BuildResult`; the real terminal state and
the `toolchain_incompatible` diagnostic only appear on `GET /api/v1/builds/{id}`. The Zed
client:

- never calls `KairoClient::build()` — it is dead code (`client.rs:123`);
- sets `status_message = "Build <id> is queued"` and stops (`service.rs:252`);
- has `is_toolchain_incompatible()` (`client.rs:361`) that is **never called**;
- has a `last_error` field that is **only ever assigned `None`** (`service.rs:58,180,208`)
  — the panel's error row (`kairo_panel.rs:159-161`) can never render anything;
- returns `Task<Result<()>>` from every action and all callers use `detach_and_log_err(cx)`
  (`kairo_ui.rs:24,35,47,58,69,80,91,107,119,130,141`), so even a synchronous typed
  `ApiError` goes to the log, not the panel.

Result: a developer with only JDK 21 sees "Build build_… is queued" forever and no
`toolchain_incompatible` — the exact masking ADR-0006's 2026-09-25 amendment forbids.
This is worse than a failed build: it is a *false green by omission*.

**Fix:** after `start_build`, poll `client.build(&id)` (GPUI executor timer, per
`AGENTS.md`) until state is `success`/`failure`/`cancelled`, with a bound (e.g. agent's
135 s write budget). On failure, build the message from `BuildResult.diagnostics` and the
`ApiError`, call `is_toolchain_incompatible()` and mark it non-retryable / "real JDK 6
required" in the UI. Set `service.last_error = Some(...)`, clear it only on the next
successful operation, and make the panel display it. Do **not** let a queued build be the
final status. Also propagate errors into the panel instead of only `detach_and_log_err`.

### BLOCKER-2 — No byte-faithful GBK path and no save hook at all

**Invariant 2. Files:** `client.rs:189-199,343-351`, `service.rs` (no save wiring),
`domain.rs:329-351`, all of `crates/zed` (`kairo` appears only in `main.rs:777-778`,
`zed.rs:91,784,809`, `settings_content.rs:202-203,1097-1126`).

`PROGRESS.md:37` claims "GBK handled at file open/save boundary via agent `/encoding/*`
endpoints (no `String::from_utf8_lossy`)." In the code:

- `detect_encoding()` and `recode()` are defined (`client.rs:189,197`) and **never called**;
- `EncodingDetection`, `RecodeRequest` (`domain.rs:329-351`) are dead DTOs;
- there is no `will_save`/`did_save`/buffer-write hook registered anywhere in the
  integration — no code even references Zed buffer/routing APIs from the Kairo crates;
- no per-file encoding/EOL provenance is stored in `KairoService` or the panel;
- the only `String::from_utf8_lossy` in the whole integration is `client.rs:327`, used to
  truncate an **HTTP error body for display** — it does not touch file bytes, so it is not
  itself a corruption vector, but it is the only occurrence and shows the claim was never
  implemented in the data path.

Consequence: Zed opens and saves files through its UTF-8 buffer model. A GBK JSP/source
file will either be shown as replacement characters or be rewritten as UTF-8 on save —
exactly the "IDE auto-saved as UTF-8" data loss ADR-0007 exists to prevent, and risk R-004.

**Fix (what is missing):**
1. A save hook for buffers in a Kairo project: before write, get the file's encoding via
   `POST /encoding/detect`; if it is not the buffer's encoding, serialize through
   `POST /encoding/recode` (atomic, temp+rename on the agent), never through Rust
   `String::from_utf8_lossy` or a re-encode in Zed.
2. A per-file provenance cache (`byteOrderMark | detected | explicit`, EOL) attached to the
   buffer, plus an explicit "Save with encoding" command with a transformation preview and
   lossy warning.
3. Open GBK files via agent-decoded bytes / a byte-preserving buffer path; route GBK search
   through the agent's `/search` (as the feature matrix already plans) rather than Zed's
   UTF-8 search.
4. `.properties` always ISO-8859-1 + `\uXXXX`; preserve EOL and BOM on every write.
Until this exists, the integration must **not** claim GBK support in docs or UI.

### ERROR-3 — Debug features are offered without the `/debug/adapter/status` gate

**Invariant 9. Files:** `extensions/kairo-java/src/kairo_java.rs:63-103`,
`client.rs:212-215`, `service.rs:287-314`, `actions.rs:23,49`, `kairo_ui.rs:83-93`.

`KairoClient::debug_adapter_status()` exists but is **dead code**. The extension's
`get_dap_binary()` unconditionally returns an attach configuration; the
`StartDebugServer` action unconditionally calls `start_server(true, …)`. No code starts the
`kairo-jdi-bridge.jar` that the comment at `kairo_java.rs:78-79` says "is started by the
Kairo panel/agent." The result is a connection timeout instead of the required ADR-0016
"gate failed → no debug / unsupported-platform banner" behavior.

Two related correctness defects in the same path:
- `service.rs:301-303` sends `configuration: Some("debugPort=8000")` — a raw string, not a
  serialized `TomcatRunConfiguration`; the agent's `debug` flag already allocates a JDWP
  port. This will likely be rejected (`invalid_request`/`not_found`) — and the error is
  swallowed by `detach_and_log_err`.
- The adapter attaches to settings `debug_port` (default **8000**, `settings.rs:35`), but the
  agent allocates JDWP ports in **18000–18049** (`02-…md` §4.6) and exposes the chosen one as
  `ServerResponse.ports.debug` (`domain.rs:305-314`). That field is never read, so the
  adapter will attach to the wrong port even when debug starts.

**Fix:** call `debug_adapter_status()` before registering/offering the `java` DAP adapter
and before `StartDebugServer`; grey out the action and show the gate's message when
unavailable. On `start_server`, drop the bogus `configuration` string and instead read
`server.ports.debug` from the response; make the DAP port come from that value (store it in
`KairoService`, pass it to debug tasks). Wire bridge startup explicitly or stop claiming it
exists.

### ERROR-4 — Hot-reload decision logic is absent; `HotDeploy` is a mislabeled alias

**Invariant 6. Files:** `kairo_ui.rs:61-71`, `service.rs:259-285`,
`client.rs:174-180,217-223`, `actions.rs:19,46`.

`HotDeploy` simply calls `service.deploy()` — the same full deploy as `Deploy`. The client's
`reload_server()`, `jvm_compile()` and `jvm_redefine()` are dead code, and nothing in the
integration reads `DeployResult.hot_reload_mode` (`domain.rs:270-272`) or an agent
`restart_required`/`compileOnly`/`synced` status. No hot-reload watcher integration exists.

To be precise: there is **no literal "synced" claim** anywhere in the crates (grep confirms),
so the client never *lies* about a successful sync. But it also never tells the user when a
Context Reload is required, and the action name promises decision logic that is not there —
a softer form of the same "false green" problem.

**Fix:** split `HotDeploy` from `Deploy`. Consume the agent's hot-reload status events and
`hot_reload_mode`; if the agent reports `restart_required` or `compileOnly`, show exactly
that in `status_message` (e.g. "Restart required — classes changed, static sync only"),
and only present a success state for `staticSync`/verified reload. Call `reload_server()`
for the verified-restart path. Do not label a full deploy as "hot deploy."

### ERROR-5 — Event stream is a stub; sequence and gap handling entirely absent

**Invariant 8, and `API_CONTRACT_SPEC.md` event vocabulary. Files:**
`client.rs:229-237,366-381`, `service.rs:319-341`.

`subscribe_events()` returns `futures::stream::empty()` — its own comment calls it a
placeholder. No caller exists. `KairoEvent.sequence` (`domain.rs:370`) is never read; there
is no subscriber, no sequence tracking, no `snapshot.required` / `event.gap` handling, and
no WS subprotocol auth. The only liveness is a 2-second `GET /servers/{id}/logs?tail=200`
poll (`service.rs:319-341`) which updates **logs only** — server state, build results and
crashes are never pushed to the panel.

The agent's EventHub (1000-event ring, 256-buffer per subscriber, sequence/gap semantics)
therefore provides zero value to the Zed client today.

**Fix:** implement `/api/v1/events` WS with the `kairo-secret-v1` subprotocol; keep the last
seen sequence; on `event.gap` or `snapshot.required`, discard incremental state and call
`refresh()` (plus a fresh build poll) to resync; dispatch `build.*`, `server.*` and
`hotreload.status` into `KairoService` so the panel reacts without polling. Preserve the
event type strings exactly.

### WARNING-6 — Auth secret is stored but never sent; no way to authenticate

**Invariant 3 (secrets posture) and §4.2 auth. Files:** `client.rs:28-49,257-295`,
`service.rs:150-170`, `agent.rs:104-116`, `settings.rs:6-20`.

`KairoClient.secret` is set to `None` in `service.rs:170` and never read — no
`X-Kairo-Secret`, session, CSRF or WS-subprotocol header is ever attached. The agent is
spawned without `-secret` and without `KAIRO_DESKTOP=1`, so it currently runs unauthenticated
behind loopback (which is why `VERIFICATION.md` works). There is also no setting for a
secret. This works only by accident of the launch configuration; if the agent is ever
started with a secret (desktop mode) or a user points `agent_path` at a secured agent, every
call returns `unauthenticated` and the panel shows nothing (see BLOCKER-1).

The positive side of invariant 3 holds: no marker token, `CatalinaBase`, `CatalinaHome`,
shutdown port, JVM options or `Env` are in the DTOs or panel; `ServerResponse`
(`domain.rs:285-303`) carries only opaque id/state/generation/ports/context/url/runtime id;
settings contain no secret fields. Keep it that way.

**Fix:** add a non-logged secret source (env `KAIRO_SECRET` or a settings field with
`#[serde(skip_serializing_if)]`-style redaction); send `X-Kairo-Secret` on every request and
the WS subprotocol; pass `-secret` at spawn; add CSRF/session headers if session auth is
used. Never include the secret in `status_message`, errors, or debug logs.

### WARNING-7 — Eight runtime keybindings collide with Zed defaults

**Invariant 10. Files:** `actions.rs:41-52`, `assets/keymaps/default-{windows,linux,macos}.json`.

GPUI precedence makes this more than cosmetic: a binding created with `KeyBinding::new(chord, action, None)` has no context predicate, and `Keymap::binding_enabled` (`crates/gpui/src/keymap.rs:274-279`) returns `Some(contexts.len())` for it — the maximum depth — and `cx.bind_keys` bindings are user-source (meta 0) and added after defaults. So these chords **shadow the default bindings** in the active context (at minimum they are ambiguous):

| Chord | Kairo action | Zed default it collides with |
|---|---|---|
| `ctrl-alt-b` | Compile | `workspace::ToggleRightDock` (Windows `default-windows.json:672`, Linux `default-linux.json:679`, `Workspace` context); `editor::MoveToPreviousSubwordStart` (macOS `:868`) |
| `ctrl-alt-shift-b` | CleanBuild | `editor::SelectToPreviousSubwordStart` (Linux `:820`, macOS `:872`) |
| `ctrl-alt-d` | Deploy | `editor::DeleteToNextSubwordEnd` (Linux `:816`, macOS `:866`) |
| `ctrl-alt-r` | HotDeploy | `task::Rerun` (Linux `:732`), `terminal::RerunTask` (Linux `:1362`, Windows `:1379`), `outline_panel::/project_panel::RevealInFileManager` (Windows `:1001`, `:1035`) |
| `ctrl-alt-s` | StartServer | `agent::ToggleSteerFirstQueuedMessage` (AcpThread; Windows `:330`, Linux `:327`), `workspace::SaveAll` (Linux `:715`) |
| `ctrl-alt-shift-s` | StopServer | `workspace::ToggleWorktreeSecurity` (Linux `:40`) |
| `ctrl-alt-shift-d` | StartDebugServer | `dev::ResetFrameOverlayStats` (Windows `:942`) |
| `ctrl-alt-p` | TogglePanel | `agent::ManageProfiles` (AcpThread; Linux `:249`) |

**Fix:** do not register bare global chords for these. Use a Zed-style two-stroke prefix
(e.g. `ctrl-k ctrl-alt-b`) or ship a Kairo keymap file with explicit contexts (`Workspace`
for compile/deploy, `!Editor` where needed), and audit all three platform defaults. Also
provide no-keybinding command-palette access as the fallback.

### WARNING-8 — Several actions and settings are inert or miswired

**Correctness, adjacent to invariants 1/6. Files:** `kairo_ui.rs:39-49,94-121`,
`kairo_panel.rs:175-177`, `settings.rs:6-20`.

- `BuildAndDeploy` (`kairo_ui.rs:39-49`) only calls `compile()` — it never deploys.
- `RestartServer` (`kairo_ui.rs:111-121`) only calls `refresh()` — it never restarts.
- `StopServer` (`kairo_ui.rs:101-107`) stops `servers().last()`, not a selected/tracked
  server; with multiple servers it can stop the wrong one.
- The panel's **"Start Server"** button (`kairo_panel.rs:175-177`) calls
  `service.start_agent()`, which starts the *agent*, not Tomcat — a misleading label.
- 8 of 12 settings are inert: `language_server_java_home`, `compiler_java_home`,
  `tomcat_java_home`, `source_level`, `target_level`, `encoding`, `tomcat_http_port`,
  `auto_start_agent` are read into `KairoSettings` but never used in any request. The user
  can set `compiler_java_home` and believe a JDK 6 is configured while the agent uses its
  own project config. Only `agent_path`, `data_dir`, `bundled_dir`, `debug_port` are used.

**Fix:** wire each action to the named operation; track the active server id; rename the
button to "Connect Agent" (or add a real "Start Server" action); either send the settings
through project import/update so the agent's config reflects them, or mark them explicitly
as "managed by the agent / read-only here" in the settings schema and docs.

### WARNING-9 — Log polling is bounded but single-server and never stopped

**Invariant 7 — mostly PASS. Files:** `kairo_panel.rs:16,143-150`, `service.rs:319-341,364-376`.

The cap is correct: `MAX_VISIBLE_LOG_LINES = 200`, server-side `tail=200`, and the panel
reverses/takes 200 — no unbounded DOM, matching `ui-spec.md` §7. However,
`start_log_polling` overwrites `self.poll_task` on every `start_server`, so only the most
recently started server's logs are polled; the old task is cancelled. `stop_server` does
not cancel the task, so it keeps polling until the request errors.

**Fix:** key the poll task by server id (or cancel explicitly in `stop_server`), and clear
logs on stop. Keep the 200-line bound.

### NIT-10 — DAP port parsing silently clamps; debug port never comes from the agent

**Invariant 4 (loopback) PASS, correctness NIT. File:** `kairo_java.rs:72-98`.

`port...min(u16::MAX as u64)` silently clamps an out-of-range config value instead of
returning an error, and the default `8000` is not reconciled with `ServerResponse.ports.debug`
(see ERROR-3). `host: None` correctly defaults to `Ipv4Addr::LOCALHOST`
(`task/src/debug_format.rs:30-35`), and the attach request forces `hostName: 127.0.0.1`, so
the loopback invariant for DAP holds. **Fix:** validate the port range and fail with a clear
message; source the port from the agent-reported debug port.

---

## 5. Invariants that do pass (with the evidence)

- **3 — Secrets:** no secret/marker/env/JVM-options/paths leak through `domain.rs` DTOs or
  `kairo_panel.rs`; settings contain no secrets. Only the JDK `home` path is exposed via
  `Toolchain`, which is user-imported provenance, not a secret. (Auth plumbing is dead —
  WARNING-6 — but that is a missing capability, not a leak.)
- **4 — Loopback:** `agent.rs:86-91` rejects any bind address other than
  `127.0.0.1`/`localhost`; `service.rs:158` hardcodes `127.0.0.1` and uses port 0
  (ephemeral). The DAP template omits `host` → `127.0.0.1` (`kairo_java.rs:80-84`), and the
  attach args hardcode `hostName: 127.0.0.1`. Tomcat/JDWP binding remains entirely in the
  agent. No client code binds a listener.
- **5 — Process identity / atomic writes:** all deploy, compile, recode and server
  operations go through `KairoClient` → agent endpoints; the client performs no
  project-file writes. `spawn_agent` removes only the stale `agent-state.json` before
  launch (`agent.rs:96-102`) and uses `util::process::Child` so dropping the handle reaps
  the process tree (`agent.rs:50-57,118`). Generation/marker verification stays in the
  agent; the client does not bypass it (it never sends a kill signal by PID).
- **7 — Bounded logs:** 200-line server cap + 200-line UI cap (WARNING-9 is about scope,
  not boundedness).

## 6. Recommended fix order

1. BLOCKER-1 — build-result polling + `last_error` surfacing (fail-closed).
2. BLOCKER-2 — save hook / encoding provenance, or explicitly remove GBK claims.
3. ERROR-3 — debug gate + agent-reported debug port.
4. ERROR-4 — honest hot-reload status and separate actions.
5. ERROR-5 — real event stream with sequence/gap recovery.
6. WARNING-6/7/8/9, NIT-10.

Do not mark Round 1 green until 1–5 have targeted re-review; the code currently compiles
or not is secondary to these semantic gaps (and remains unverified — no Rust toolchain).
