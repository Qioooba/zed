# Real end-to-end verification (Go runtime agent)

> Executed 2026-11-19 on Windows with the actual `kairo-runtime.exe` from
> `G:\spaces\kairo-ide\runtime-agent\`, the actual `legacy-sample` fixture, and
> real bundled Tomcat 6.0.53. These results prove that the backend the Zed
> integration drives performs the real work — they do **not** prove the Rust
> client compiles (no Rust toolchain is installed here; see `PROGRESS.md`).

## Setup

```
kairo-runtime.exe -port 0 -bind 127.0.0.1 `
  -data-dir D:\GPU-Tools\Temp\opencode\kairo-verify `
  -bundled-dir G:\spaces\kairo-ide\bundled -log-level warn
```

The agent wrote `<data-dir>/agent-state.json` with `status: "ready"` and an
ephemeral port (`62464`), exactly the discovery mechanism implemented in
`kairo_core/src/agent.rs`.

## Results

| Step | API / action | Result |
|------|--------------|--------|
| Liveness | `GET /api/v1/health` | `ok:true`, platform `windows/amd64`, reports bound port |
| Port discovery | `GET /api/v1/endpoints` | `{ http: "127.0.0.1:62464", events: "..." }` |
| Workspace open | `POST /api/v1/workspaces` | `ws_jmosjdqhbhhhkgtqh6wdl5zhae`, canonical path |
| Legacy layout scan | `POST /api/v1/workspaces/{id}/scan` | detected Ant, `src/main/java`, `WebRoot`, `lib`, `build.xml`, `.jsp → gbk`, `.properties → iso-8859-1`, 2 servlets |
| Project detection | `POST /api/v1/projects/detect` | source/target `1.6`, encoding `gbk`, confidence 0.85 |
| Toolchain import | `POST /api/v1/toolchains/import` | Temurin 21 imported, SHA-256 fingerprint recorded |
| Project import | `POST /api/v1/projects/import` | `project-legacy-sample` created |
| **Fail-closed compile** | `POST /api/v1/builds` with `source=1.6` on JDK 21 | `state=failure`, diagnostic code `toolchain_incompatible`: "compiler toolchain <jdk> does not support requested level 1.6 (minimum supported level is 8)" — **no fake success** |
| **Real compile** | project updated to `1.8`, `POST /api/v1/builds` | `state=success`, `exitCode=0`, **7 files compiled** |
| **Real deploy** | `POST /api/v1/deployments` | `state=success`, **49 files / 960,185 bytes**, `hotReloadMode=staticSync` |
| **Real Tomcat 6 start** | `POST /api/v1/servers` | `state=running`, pid 63776, HTTP 127.0.0.1:18082, generation 1 |
| **Real JSP serving** | `GET /src/main/webapp/index.jsp` | HTTP 200, rendered HTML `<h1>Hello</h1>` |
| **JSP hot reload** | modify the served JSP in place, wait 3 s, request again | probe comment appeared (HTTP 200), file restored byte-identical — **no server restart / Context Reload** |
| Logs | `GET /api/v1/servers/{id}/logs?tail=8` | real catalina log lines with monotonic ordinals |

The server's generated `<Context path="/" docBase="G:\spaces\kairo-ide\legacy-sample">`
confirms the agent mounts the legacy project directly, so static JSP edits take
effect immediately.

## Caveats observed

- `GET /WebRoot/hello.jsp` returned 500 on JDK 21 (JSTL/GBK legacy combination);
  this is the documented "modern JDK cannot perfectly emulate Java 6" gap — the
  correct response is to import a real JDK 6, not to hide the failure.
- Toolchain catalog starts empty on a fresh data dir; a JDK must be imported
  (the agent never silently picks one), matching ADR-0006.
- `toolchain` compatibility gate reports `min level 8` for JDK 21, so a genuine
  JDK 6 is required to compile `source=1.6` — exactly the invariant the Zed
  client must surface rather than mask.

## What this means for the Zed integration

Every operation exposed by `kairo_core::KairoService` maps to one of the
endpoints proven above. The client's job is to spawn the agent (verified
mechanism), call these endpoints, and display the results in `KairoPanel` —
all of which is implemented in the new crates. The remaining work is
compilation/validation on a machine with Rust and a full UI/E2E pass.
