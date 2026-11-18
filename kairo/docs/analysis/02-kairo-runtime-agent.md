# Kairo IDE — Runtime Agent (Go backend) Complete Feature Inventory

> Target project: `G:\spaces\kairo-ide` — module `github.com/Qioooba/kairo-ide/runtime-agent` (Go 1.25).
> Analysis scope: everything under `G:\spaces\kairo-ide\runtime-agent\` (source, DTOs, routes, tests).
> Build artifacts (`bin/`, `tmp/`, `runtime-agent/`, `coverage`) and `third_party/go-ora` are excluded from source analysis but noted where relevant.

---

## 1. Overview

The Runtime Agent is the execution backend of Kairo IDE. The Electron/Theia frontend speaks a JSON **`/api/v1` wire protocol** to it over a loopback HTTP port (default `18080`, port `0` = ephemeral). The agent owns:

- the workspace/project/toolchain catalogs (disk-backed),
- a real `javac` and Ant build engine for **JDK 1.6 / legacy Java web projects**,
- an encoding engine tailored to **GBK / GB18030 / ISO-8859-1 / UTF-16 / UTF-8**,
- an encoding-aware workspace **text search** engine (sync + WebSocket streaming),
- a **Tomcat 6** lifecycle manager (start/stop/restart/debug/recover/reload) with strict **process identity verification** and Windows Job-Object child reaping,
- a **deploy engine** (merge/mirror, atomic writes, path containment) into exploded webapps,
- a **JDT LS distribution installer** and launch-descriptor bridge (the Theia backend owns the actual LSP process and stdio frames),
- a **JDT Eclipse project-model generator** (`.project`, `.classpath`, `.settings`, `KairoJavaConfig.ini`) for non-Maven legacy projects,
- one-click **Java run / JUnit** (JUnit4 single-method runner) and managed debug JVM sessions,
- a **JDWP-based debug/hotswap** stack (`RedefineClasses`, breakpoints, stepping, variables),
- experimental **Maven**, **Oracle SQL (pure-Go driver)**, and **SSH debug tunnel** (`-tags remote`) features,
- a cross-cutting **security** layer: workspace path sandbox, loopback+shared-secret auth, session/CSRF, login throttling, per-IP rate limiting, safe-origin CORS, audit log, HMAC deployment-owner tokens.

Two top-level binaries:
- `cmd/kairo-runtime` — the main agent (`kairo-runtime.exe`).
- `cmd/kairo-remote` — a separate, fail-closed TLS file gateway for explicitly authorized workspace roots (no process execution).

A key architectural fact for porting: there are **two API layers**. The production layer is `api.Server` (`internal/api/server.go` + `handlers.go` + handler files), wired by `bootstrap.NewContainer` → `services.NewMemoryServices`. A second typed layer (`api.APIHandler` in `api_handler.go`, plus `dto.go` conversions, `security/rbac.go`, `security/sso.go`, `security/retention.go`) is behind `//go:build unwired` and is **never compiled into the production binary** — it exists for unit tests/future multi-user work. Do not reimplement the unwired layer unless targeting that future.

---

## 2. Package map

| Package (path under `runtime-agent/`) | Responsibility | Key files |
|---|---|---|
| `cmd/kairo-runtime` | Process entry: config bind, audit, graceful shutdown (signal/restart/port handoff), agent-state file, serve | `main.go`, `console_utf8_{windows,other}.go`, `main_test.go` |
| `cmd/kairo-remote` | Standalone authenticated-TLS file gateway binary | `main.go` |
| `internal/bootstrap` | Composition root: wires services, EventHub, sandbox, registers managed Catalina roots, shutdown phases | `container.go` |
| `internal/config` | Config struct, defaults, YAML/env/flag precedence, validate | `config.go`, `osfile.go` |
| `internal/api` | **Production HTTP layer**: router, middleware, envelopes, all handlers, docs DTOs | `server.go`, `handlers.go`, `services.go`, `protocol/types.go`, `health.go`, `lifecycle.go`, `build_handlers.go`, `hot_deploy_handlers.go`, `run_configurations.go`, `launch_jobs.go`, `java_run_handlers.go`, `java_run_sessions.go`, `java_run_toolchain.go`, `search_events.go`, `encoding` handlers, `sql_handler.go`, `debug_handlers.go`, `debug_tunnel*.go`, `antclasspath_handlers.go`, `port_diagnostics.go`, `operation_registry.go`, `rate_limiter.go`, `login_throttle.go`, `project_*.go` |
| `internal/api/protocol` | Hand-written Go mirror of `packages/protocol/src/index.ts` (wire types, error codes) | `types.go` |
| `internal/domain` | Core entities & interfaces (Project, ServerRecord, BuildPlan, DeployPlan, RuntimePlan, tokens, state machines) | `project.go`, `run_configuration.go`, `errors.go`, `path.go`, `build_state_test.go` |
| `internal/app` | Use cases (build/deploy/server) + plan resolver — used mainly by unwired layer | `build_usecase.go`, `deploy_usecase.go`, `server_usecase.go`, `plan_resolver.go` |
| `internal/services` | Concrete service implementations wired into `api.Services` (build engine, deployer, server runner, searcher, encoder, auth, JDTLS/JDT wrappers, scanner, orchestrator) | `services.go`, `config.go`, `build.go`, `deploy.go`, `server.go`, `search.go`, `encoding.go`, `auth.go`, `scanner.go`, `project_detector.go`, `jdtls.go`, `jdtproject.go`, `launch_orchestrator.go`, `workspace.go`, `password.go`, `encoding_guard.go`, `run_configuration.go` |
| `internal/build` | javac compiler, argfile, diagnostics, manifest, custom command executor | `compiler.go`, `manifest.go`, `custom_executor.go`, `custom_executor_{unix,windows}.go`, `probe_{unix,windows}.go` |
| `internal/deploy` | Safe deploy plan builder + engine (path containment, symlink checks, atomic file writes) | `plan.go`, `sync.go`, `package.go`, `atomic.go` |
| `internal/tomcat6` | Tomcat 6 command construction, catalina base prep, server.xml generation, readiness, lifecycle | `tomcat6.go` |
| `internal/catalinabase` | Catalina-base layout, owner metadata, context path normalization, planner/preparer | `layout.go`, `owner.go`, `planner.go`, `preparer.go` |
| `internal/provider/runtime` | RuntimeProvider implementations: Tomcat6Provider, Ant build provider, hot-reload watcher | `tomcat6_provider.go`, `ant_provider.go`, `hot_reload.go` |
| `internal/runtimeplan` | Runtime plan resolution, port allocation, deployment target + owner token issuance, runtime registry | `resolver.go`, `ports.go`, `registry.go`, `fakes.go` |
| `internal/jdtls` | JDT LS distribution installer (pinned 1.55.0), host JRE resolution, deprecated process manager | `distribution.go`, `jre.go`, `jdtls.go` |
| `internal/jdtproject` | JDT Eclipse project model generator + XML/INI renderers | `generator.go`, `render.go`, `os_helpers.go` |
| `internal/encoding` | Detection/conversion/validation/strict decode/properties encoding | `encoding.go` |
| `internal/search` | Encoding-aware content search (sync + streaming), glob, filename listing | `search.go`, `glob.go` |
| `internal/security` | WorkspaceRoots sandbox, origin/WebSocket policy, read-file path handle (Windows), plus unwired RBAC/SSO/retention | `sandbox.go`, `origin.go`, `read_file_path_windows.go`, `read_file_path_other.go` (+ `rbac.go`, `sso.go`, `retention.go` unwired) |
| `internal/pathpolicy` | Lexical path validation / containment (`DefaultPathPolicy`, URI/path resolution, IDs) | `path.go`, `root.go`, `id.go` |
| `internal/atomicfile` | Crash-safe file writes (write+fsync+rename, dir sync; platform rename backends) | `atomicfile.go`, `atomic_rename_{unix,windows}.go`, `sync_dir_{unix,windows}.go` |
| `internal/proc` | Managed process abstraction: identity token, process groups, Job Object, log ring, Windows close-handle path | `proc.go`, `proc_unix.go`, `proc_windows.go`, `output_decode.go`, `fake.go` |
| `internal/log` | Structured JSON logger w/ redaction + capture ring | `log.go`, `ring.go` |
| `internal/transport/events` | EventHub (sequence/history/gap detection), WebSocket adapter | `eventhub.go`, `websocket.go`, `wsadapter.go` |
| `internal/audit` | NDJSON audit log, categories, HMAC signing, daily/size rotation | `audit.go` |
| `internal/repository` | File-backed repos (workspace, project, toolchain, build/server history, run configs), versioned YAML/JSON, project.yaml | `*.go` |
| `internal/antpath` | Ant `build.xml` parser/classpath resolver (classpaths, filesets, inline javac) | `model.go`, `parser.go`, `resolver.go`, `detector.go`, `inline_fileset.go` |
| `internal/toolchain` | JDK registry: detect (java -version, sha256 fingerprint, probe source levels), sorted list | `toolchain.go`, `helpers.go`, `lookpath_{unix,windows}.go` |
| `internal/jdkmanager` | Host JDK 17+ detection for JDI/DAP bridge; download/extract of JDK17 | `manager.go`, `download.go` |
| `internal/junitrunner` | JUnit4 single-method runner source + legacy cleanup | `source.go`, `legacy.go` |
| `internal/maven` | Maven detect/dependency tree/lifecycle task execution | `maven.go`, `lifecycle.go`, `settings.go` |
| `internal/sql` | Oracle connector, pure-Go driver, bind escape, null projection, result budget | `oracle.go`, `oracle_driver.go`, `connection_store.go`, `oracle_null_projection.go`, `oracle_result_budget.go` |
| `internal/debug` | JDWP client, breakpoints, stepping, variables, stackframes, sourcemap, hotswap, redefine-live, multi-VM | `jdwp.go`, `hotswap.go`, `redefine_live.go`, `breakpoint.go`, `step_control.go`, `variable.go`, `stackframe.go`, `sourcemap.go`, `gate_probe.go`, `multi_vm_orchestrator.go` |
| `internal/remote` | Fail-closed TLS file gateway; remote-tagged SSH tunnel/container isolation (build tag `remote`) | `gateway.go`, `server.go`, `auth.go`, `ssh_tunnel.go`, `agent_proxy.go`, `container_isolation.go`, `file_sync.go`, `doc.go` |
| `internal/testsupport/oraclefixture` | Test fixture builder for Oracle | `fixture.go` |
| `test/{integration,e2e,fault}`, `internal/test/integration` | Tagged end-to-end / fault-injection tests | `smoke_test.go`, `encoding_roundtrip_test.go`, `sandbox_bypass_test.go`, etc. |

Build tags in play: `unwired` (unused typed API + RBAC/SSO/retention), `remote` (SSH tunnel / container isolation), `integration` / `e2e` / `fault` (test suites).

---

## 3. FULL API endpoint table

All endpoints are under `/api/v1`. Unless noted, POST requests accept an **envelope** `{"requestId","correlationId?","workspaceId?","projectId?","payload":{...}}` (bare payloads are also tolerated by `readEnvelopeAndBody` / `extractPayload`). Success responses are `{"requestId","correlationId?","ok":true,"payload":...}`; failures are `{"requestId","correlationId?","ok":false,"error":{"code","message","details?","retryable?"}}` (see `protocol/types.go`). GET/DELETE carry IDs in the path or headers (`X-Kairo-Request-Id`, `X-Kairo-Correlation-Id`, `X-Kairo-Workspace-Id`). Public endpoints (no auth): `health`, `endpoints`. WebSocket endpoints (`events`, `search/stream`) self-authenticate via the `kairo-secret-v1` subprotocol.

| # | Path | Method | Request / payload | Response payload (purpose) | Handler / file |
|---|---|---|---|---|---|
| 1 | `/health` | GET | — | `HealthResponse{ok,version,agentVersion,uptimeSec,platform{os,arch},bindAddress,port,activeSessions}` (liveness) | `health.go` |
| 2 | `/endpoints` | GET | — | `RuntimeEndpoints{http,events}` = `host:port` (dynamic port discovery; replaced hardcoded 18099) | `handlers.go:handleEndpoints` |
| 3 | `/runtime/restart` | POST | no body | `{status:"restarting"}` (200, then async respawn + port handoff) | `handlers.go:handleRuntimeRestart`, `server.go:doRestart` |
| 4 | `/workspaces` | GET | — | `[]WorkspaceRecord` (list, optionally filtered by caller workspace) | `handlers.go:handleWorkspaces` |
| 5 | `/workspaces` | POST | `{rootPath|root,name}` | `WorkspaceRecord` (canonicalizes path, rejects `..`, opens + registers sandbox root) | `handlers.go:handleWorkspaces` |
| 6 | `/workspaces/{id}` | GET | — | `WorkspaceRecord` | `handleWorkspacesSub` |
| 7 | `/workspaces/{id}` | DELETE | — | `{ok:true}` (close workspace) | `handleWorkspacesSub` |
| 8 | `/workspaces/{id}/scan` | POST | `{rootPath?,deep?}` (strict) | `{detected:[DetectedProjectLayout]}` (legacy project structure detection) | `handleWorkspacesSub`, `services/scanner.go` |
| 9 | `/workspaces/{id}/projects/import` | (POST) | — | delegates to legacy project import | `handleWorkspacesSub` → `handleProjectImport` |
| 10 | `/workspaces/{ws}/java/launch-descriptor?projectId=` | GET | query `projectId` | JDT LS launch descriptor (command/args/workingDir/envAllowlist); triggers JDT project-model generation | `handlers.go:handleJDTLSLaunchDescriptor` |
| 11 | `/workspaces/{ws}/java/prepare` | POST | — | install report (ensures JDT LS distribution) | `handlers.go:handleJDTLSPrepare` |
| 12 | `/workspaces/{ws}/run-configurations` | GET | — | `RunConfigurationDocument` (repairs stale projectIds) | `run_configurations.go:handleRunConfigurations` |
| 13 | `/workspaces/{ws}/run-configurations` | PUT | `RunConfigurationDocument` | saved document (replace all) | same |
| 14 | `/workspaces/{ws}/run-configurations` | POST | `TomcatRunConfiguration` | saved document (create) | same |
| 15 | `/workspaces/{ws}/run-configurations/{configuration}` | GET | — | `TomcatRunConfiguration` | `handleRunConfigurationByID` |
| 16 | `/workspaces/{ws}/run-configurations/{configuration}` | PUT | `TomcatRunConfiguration` | saved document | same |
| 17 | `/workspaces/{ws}/run-configurations/{configuration}` | DELETE | — | saved document | same |
| 18 | `/workspaces/{ws}/run-configurations/{configuration}/launch` | POST | `{mode:"run"\|"debug"}`, `?async=1` | 200 `ServerResponse` (sync) or 202 `LaunchJob` (async) — build+deploy+start with orchestration + unobserved-launch guard + workspace filter | `run_configurations.go:handleRunConfigurationLaunch` |
| 19 | `/launch-jobs/{id}` | GET | — | `LaunchJob{id,state,step,tasks,steps,server,errorCode,error,...}` (poll async launch) | `launch_jobs.go` |
| 20 | `/projects` | GET | `?workspaceId=` or header | `[]domain.Project` (filtered by workspace) | `handlers.go:handleProjects` |
| 21 | `/projects/{id}` | GET | — | `domain.Project` | `handleProjectByID` |
| 22 | `/projects/{id}` | PUT | `domain.Project` | updated project (update-only; writes `.kairo/project.yaml` + catalog with rollback) | `handleProjectByID` |
| 23 | `/projects/{id}` | DELETE | — | `{ok:true}` | `handleProjectByID` |
| 24 | `/projects/detect` | POST | `{rootPath}` | `ProjectDetection` (sourceDirs, webRoot, libDirs, buildScript, encodings, JDK/source/target 1.6, confidence, warnings) | `handleProjectDetect`, `convertToProjectDetection` |
| 25 | `/projects/import` | POST | `ProjectImportConfirmRequest` | created `domain.Project` (create-only, 409 on duplicate, writes project.json + project.yaml, MRU) | `handleProjectImportNew` |
| 26 | `/projects/recent` | GET | — | `[]recentProjectEntry` (MRU top 10, reconciled against catalog) | `handleProjectRecent`, `project_recent.go` |
| 27 | `/toolchains` | GET | — | `[]json.RawMessage` (`protocol.Toolchain`) — sorted oldest JDK first | `handleToolchains` |
| 28 | `/toolchains/import` | POST | `{path,label}` | imported `Toolchain` (detects java/javac, sha256 fingerprint, source levels) | `handleToolchainImport` |
| 29 | `/builds` | GET | `?workspaceId` | `[]BuildResult` (idempotency-filtered, output stripped in list) | `handlers.go:handleBuilds`, `services/build.go` |
| 30 | `/builds` | POST | `BuildRequest{projectId,clean?,intent?,selectedFiles?}` (strict) | `BuildResult` queued (async; project hydrated server-side; operation-registry dedup) | `handleBuilds`, `hydrateBuildRequest` |
| 31 | `/builds/{id}` | GET | — | `BuildResult` (full output/diagnostics) | `handleBuildByID` |
| 32 | `/builds/{id}` | DELETE | — | cancelled `BuildResult` (kills javac process tree) | `handleBuildByID` |
| 33 | `/build/custom` | POST | `build.CustomBuildConfig{projectRoot,command,buildId?,timeoutMs?,...}` | `{buildId,status:"running"}` (async detached command, 30m default budget) | `build_handlers.go:handleCustomBuild` |
| 34 | `/build/custom/{buildId}` | GET | — | `{buildId,status,exitCode?}` | `handleCustomBuildStatus` |
| 35 | `/build/custom/{buildId}/cancel` | POST | — | `{buildId,status:"cancelled"}` | `handleCustomBuildCancel` |
| 36 | `/ant/classpath/analyze` | GET/POST | `?projectRoot&buildFile` or `{projectRoot,buildFile}` | `{success,buildFile,classpath,classpathCount,sourceRoots,outputDir,properties,warnings}` (Ant classpath analysis) | `antclasspath_handlers.go` |
| 37 | `/deployments` | GET | — | `[]DeployResult` | `handlers.go:handleDeployments` |
| 38 | `/deployments` | POST | `DeployRequest{projectId,buildId,scope}` (source/target resolved server-side; sandbox enforced) | `DeployResult` (sync publish) | `handleDeployments`, `services/deploy.go` |
| 39 | `/deployments/{id}` | GET | — | `DeployResult` | `handleDeploymentByID` |
| 40 | `/servers` | GET | `?workspaceId` | `[]ServerResponse` (safe DTO; no local paths) | `handlers.go:handleServers` |
| 41 | `/servers` | POST | `StartServerRequest{projectId,debug?,...}` | `ServerResponse` (allocates ports, starts real Tomcat 6) | `handleServers`, `services/server.go:Start` |
| 42 | `/servers/{id}` | GET | — | `ServerResponse` | `handleServerSub` |
| 43 | `/servers/{id}` | DELETE | body `{force?}` | `ServerResponse` (graceful or force stop) | `handleServerSub` |
| 44 | `/servers/{id}/debug` | POST | — | `ServerResponse` (restart with JDWP on fresh port) | `handleServerSub` |
| 45 | `/servers/{id}/restart` | POST | — | `ServerResponse` (stop + start, generation++) | `handleServerSub` |
| 46 | `/servers/{id}/logs` | GET | `?tail=N` | `[{line,ts,stream?,source?,ordinal}]` (bounded tail of catalina logs + headers state/pid) | `handleServerLogs`, `services/server.go:Logs` |
| 47 | `/servers/{id}/recover` | POST | — | `ServerResponse` (relaunch crashed server) | `handleServerSub` |
| 48 | `/servers/{id}/reload` | POST | — | `{status:"reloaded",action:"restart",previousGeneration,generation,...}` (verified restart, new gen serves) | `hot_deploy_handlers.go:handleServerReload` |
| 49 | `/servers/recoverable` | GET | — | `[]ServerResponse` (crashed, WasRunning) | `handleRecoverableServers` |
| 50 | `/operations/{requestId}` | GET | — | `OperationStatusDTO{requestId,scope,kind,state,statusCode,error,createdAt,completedAt}` | `handlers.go:handleOperationsSub`, `operation_registry.go` |
| 51 | `/search` | POST | search request (query,isRegex,caseSensitive,wholeWord,include,exclude,contextLines,maxResults,previewReplace,projectEncoding) | `search.Result{matches,totalMatches,filesSearched,truncated,elapsedMs,erroredFiles}` | `handlers.go:handleSearch`, `services/search.go` |
| 52 | `/search/files` | POST | `{workspaceId,rootPath?,include?,exclude?,maxFiles?}` | `{files:[{path,name}],total,truncated}` | `handleSearchFiles` |
| 53 | `/search/stream` | WebSocket | first frame = search request JSON | streamed `SearchStreamEvent` batches + final stats (ping/pong cancel) | `search_events.go` |
| 54 | `/encoding/detect` | POST | `{file,sampleBytes?}` | `{file,encoding,confidence,candidates,hasBom,eol}` | `handlers.go:handleEncodingDetect`, `services/encoding.go` |
| 55 | `/encoding/recode` | POST | `{file,from,to,eol?}` | `{ok:true,bytes}` (strict decode + atomic in-place rewrite) | `handleEncodingRecode` |
| 56 | `/encoding/validate` | POST | `{text,encoding}` | `{valid,error?}` (representability check) | `handleEncodingValidate` |
| 57 | `/auth/login` | POST | `{username,password}` | login result + session cookie (throttled: 5/60s, 10/600s lockout) | `handlers.go:handleLogin`, `login_throttle.go`, `services/auth.go` |
| 58 | `/auth/logout` | POST | — | `{ok:true}` | `handleLogout` |
| 59 | `/audit` | GET | — | `[]audit.Event` (reads audit.log.ndjson) | `handleAudit`, `audit/audit.go` |
| 60 | `/events` | WebSocket | subprotocol `kairo-secret-v1` + secret | event stream events `{sequence,type,workspaceId,message?,data?,time}` | `handlers.go:handleEvents`, `transport/events` |
| 61 | `/jdtls` | GET | — | JDT LS status (state/version/jre/jar/sourceLevel/lastError) | `handlers.go:handleJDTLS` |
| 62 | `/jdtls` | POST | — | install report (`Prepare`, singleflight) | `handleJDTLS` |
| 63 | `/jdtls` | DELETE | — | 400 (explicitly unsupported; Theia owns process lifecycle) | `handleJDTLS` |
| 64 | `/jdtls/distribution` | GET | — | `DistributionStatus{installed,version,home,launcherJar,source,message}` | `hot_deploy_handlers.go:handleJDTLSDistribution` |
| 65 | `/jdtls/project` | GET | `?workspaceId=` | JDT project-model status `{workspaceId,exists,projectId,generatedAt}` | `handlers.go:handleJDTProject` |
| 66 | `/jdtls/project` | POST | `{workspaceId,projectId,rootPath,intoProjectRoot?,autoDetectClasspath?,buildFile?,libraries?}` | `GenerateResult` (model paths, src roots, classpath entries, encoding/levels) | `handleJDTProject`, `jdtproject/generator.go` |
| 67 | `/diagnostics/port?port=` | GET | query `port` (1–65535) | `PortDiagnostics{port,occupied,pid?,processName?,suggestion?}` | `port_diagnostics.go` |
| 68 | `/maven/detect` | POST | `{rootPath}` | Maven project detection | `handlers.go:handleMavenDetect`, `maven` |
| 69 | `/maven/dependencies?rootPath=&offline=` | GET | query | dependency tree | `handleMavenDependencies` |
| 70 | `/maven/run` | POST | `maven.RunRequest{rootPath,task,...}` | lifecycle task result (task whitelist enforced) | `handleMavenRun` |
| 71 | `/sql/execute` | POST | `{connectionId,sql,maxRows?,offset?,params?}` | `sqlQueryResponse{columns,rows,rowCount,totalRows,executionTimeMs,truncated,rowsAffected?}` | `sql_handler.go` |
| 72 | `/sql/test-connection` | POST | `{host,port?,sid?,serviceName?,useServiceName?,username,password}` | `{success,oracleVersion?,instanceName?,connectionId}` | `sql_handler.go` |
| 73 | `/debug/adapter/status` | GET | — | `jdkmanager.FullStatus` (JDI bridge host JDK status) | `debug_handlers.go` |
| 74 | `/debug/tunnel/connect` | POST | `DebugTunnelConnectRequest{host,port,sshPort?,user,authType,sshKeyPath?,token?,localPort?}` | `{success,localPort}` (needs `-tags remote`; else clear error) | `debug_tunnel.go` |
| 75 | `/debug/tunnel/disconnect` | POST | — | `{success:true}` | `debug_tunnel.go` |
| 76 | `/debug/jdk/download` | POST | — | `{status, jdkPath?,message?}` (sync 180MB JDK17 fetch, 10m write budget, progress events) | `debug_handlers.go` |
| 77 | `/java/run` | POST | `RunJavaRequest{projectRoot,filePath,className,packageName?,line?,debug?,methodType:"main"\|"test"}` | `RunJavaResponse{ok,exitCode,stdout,stderr,buildDir?,classpath?,debugPort?,runId?,state?,pid?}` | `java_run_handlers.go` |
| 78 | `/java/runs/{runId}` | GET | — | managed JVM session snapshot `{runId,state,pid,debugPort,exitCode,stdout,stderr}` | `java_run_sessions.go` |
| 79 | `/java/runs/{runId}` | DELETE | — | `{ok:true}` (kills managed JVM) | `java_run_sessions.go` |
| 80 | `/java/detect` | POST | `{filePath}` | `JavaClassInfo{packageName,className,methods[{name,line,isMain,isTest,startLine,endLine}]}` | `java_run_handlers.go:handleDetectJava` |
| 81 | `/jvm/compile-incremental` | POST | `{projectId,files:[path\|fileURI]}` | build queued `{state,filesCompiled,id}` | `hot_deploy_handlers.go:handleCompileIncremental` |
| 82 | `/jvm/compile` | POST | `{file\|sourceUri,projectId?,className?}` | `{success,buildId,state,classPath,projectId?}` (waits up to 120s for terminal state) | `handleJvmCompile` |
| 83 | `/jvm/redefine` | POST | `{sourcePath\|sourceUri,classPath,className?,expectedHash?,classLoaderId?,jdwpHost?,jdwpPort?,projectId?,serverId?,target?:DebugTargetBinding}` | `{success,className,refTypeId,byteCount,jdwpHost,jdwpPort}` (JDWP RedefineClasses; explicit target binding required, loopback-only JDWP) | `handleJvmRedefine`, `debug/redefine_live.go` |

Separate binary `kairo-remote` (TLS, `Authorization: Bearer <32+ char token>`, no browser Origin allowed):

| Path | Method | Purpose |
|---|---|---|
| `/api/v1/remote/health` | GET | `{status:"healthy",tls:"1.3"}` |
| `/api/v1/remote/ws` | WS | liveness-only websocket (ping/pong; file ops use bounded HTTP) |
| `/api/v1/workspaces` | GET | virtual roots list `[{id:"wN",name,path:"wN"}]` |
| `/api/v1/files/{stat,list,read,mkdir,write,delete,rename,copy}` | GET/POST/DELETE | file ops against virtual workspace IDs, 50 MiB cap, base64 content |

Unwired-only (build tag `unwired`, not in production): `GET /api/v1/health/ready`, and typed `HandleWorkspaces/Projects/Builds/Deployments/Servers/ServerByID/ServerRestart/ServerLogs/Events/Toolchains` in `api_handler.go`.

### Error code → HTTP status mapping (`server.go:writeError`)
`unauthenticated`→401; `forbidden`/`path_forbidden`→403; `not_found`/`target_not_found`→404; `conflict`/`target_ambiguous`/`stale_target`→409; `rate_limited`→429; `unsupported`→501; other 4xx codes (`invalid_request`, `toolchain_missing`, `runtime_missing`, `unsupported_jdk_target`)→400; all 5xx (`internal`, `io_error`, `process_spawn_failed`, `compile_failed`, `deploy_failed`, `debug_attach_failed`, `cancelled`, `timeout`, `plugin_crashed`)→500.

---

## 4. Feature areas

### 4.1 Process bootstrap, config, lifecycle & restart
- **Behavior**: `cmd/kairo-runtime/main.go` enables UTF-8 console, optionally `--hash-password` (Argon2id), binds config (defaults → YAML → `KAIRO_` env → flags), enforces audit policy (daily rotation, 90-day retention, 8 MiB size valve; **signing intentionally off** because no stable key), fails fast when `KAIRO_DESKTOP=1` without a secret, resolves bundled Tomcat 6 home, builds the container, wires the EventBus, creates `api.Server`, binds only loopback (non-loopback bind is fatal: "remote mode not available"), writes `agent-state.json` atomically with generation monotonicity, and serves HTTP/TLS.
- **Key files/types**: `main.go` constants `defaultWriteTimeout=60s`, `drainTimeout=15s`, `childrenShutdownTimeout=30s`, `restartShutdownTimeout=30s`; `config.Config` (`config/config.go`) with defaults (bind `127.0.0.1`, port `18080`, dataDir `$HOME/.kairo`, session 8h, idle 30m, log ring 5000, scan depth 8/50k files, Tomcat start 60s/stop 15s); `RestartConfig`, `AgentState` (`protocol/types.go`).
- **How it works**: graceful shutdown is a `sync.Once` with phases (HTTP drain → managed-child force-stop → log completion); `ListenWithHandoff` retries bind on EADDRINUSE with 50→250ms backoff for restart port handoff; `/runtime/restart` spawns the replacement process **first** (detached stdio, `KAIRO_GENERATION`++, `KAIRO_PARENT_PID`, `--port` pinned to bound port), then shuts down and `os.Exit(0)`.
- **Porting notes**: Must preserve loopback-only bind, atomic state file with generation checks, spawn-before-shutdown restart ordering, and F4b per-route write budgets (compile 135s, java run 135s, launch 5m, JDK download 10m) to avoid duplicate side effects on retry.

### 4.2 HTTP server, middleware & idempotency
- **Behavior**: `api.Server` uses `http.ServeMux` wrapped by CORS → rate limiter → auth/request-id/recovery middleware. Shared-secret auth: `X-Kairo-Secret` constant-time compare; when `requireAuth` without secret, session token (`X-Kairo-Session` / `Bearer`) + CSRF token (`X-Kairo-CSRF`) on state-changing methods. Safe origins only (loopback http(s), `file`, `vscode-webview`); cross-origin state-changing requests are actively rejected. Per-IP token bucket (default 100/min, disabled at ≤0; XFF ignored from loopback). Login throttle 5 failures/60s→60s lock, 10/600s→600s. `OperationRegistry` (SHA-256 payload fingerprint, 10m TTL, 512 entries) coalesces concurrent/idempotent mutating requests; `LaunchJob` async state machine (30m TTL/timeout, 256 jobs, join per workspace+project) and an unobserved-launch ledger prevent orphan Tomcats.
- **Key files**: `server.go`, `http_helpers.go`, `services.go` (all service interfaces), `operation_registry.go`, `rate_limiter.go`, `login_throttle.go`, `launch_jobs.go`.
- **Porting notes**: Reproduce the envelope shapes, error enum, header names, `newRequestID()` (`req_`+12 random alphanumeric), `X-Kairo-Idempotent-Replay: 1`, and the safe-origin + CSRF rules exactly.

### 4.3 Domain model
- **Behavior/Key types** (`internal/domain/project.go`, `run_configuration.go`, `errors.go`): ID aliases (`WorkspaceID`, `ProjectID`, `BuildID`, `ServerID`); `Project` (root, source/resource/lib dirs, webappDir, outputDir, build tool `ant|javac`, source/target level, encoding, contextPath, toolchainID); `ProjectConfig` (`.kairo/project.yaml` source of truth); `Toolchain`; `BuildPlan`; `RuntimePlan` (javaHome/catalinaHome/catalinaBase/webappDir/deploymentRoot/ports/JVMOptions/Env/Generation, with `DeepCopy`); `DeployPlan`+`DeployEntry`+`OwnerToken` (json:"-"); `ServerRecord` (desired/observed state, generation, ProcessIdentity, RuntimePlan); `BuildRun`, `ServerEvent`, `LogLine`; `ProcessIdentity{PID,Executable,StartTime,CatalinaBase,MarkerToken}`; legal server/build state-transition tables. Run configurations are versioned documents of `TomcatRunConfiguration{projectId, mode, JDKRef, beforeLaunchTasks, build{target,args}, server{httpPort,debugPort,contextPath,env}, deploy{artifact}, suspend, vmOptions}` with strict decoders and `${env:VAR}` references.
- **Porting notes**: Keep sensitive fields out of API DTOs (`ToServerUseCaseResponse` is the single choke point, ADR-0012); domain objects must never be serialized directly.

### 4.4 Build system (JDK 1.6 focus) — javac, Ant, JDT model
- **Behavior**: `POST /builds` is async (`asyncBuildEngine`): validates/hydrates the request from the stored project (root, output, webapp, classpath jars from `lib` + `WEB-INF/lib`, resource roots, encoding, source/target default `1.6`), queues a `build_...` id, persists `finished.json` (500-entry cap, restart reconciliation of orphaned queued/running→failure), and compiles on a background goroutine. After success it copies resource roots, mirrors classes into `WEB-INF/classes`, and runs an **encoding-preservation self-check** (CJK string literals must survive into `.class`, else `encoding_lost` failure). Output/diagnostics are sanitized (workspace/JDK paths → `<workspace>`/`<jdk>`, secret assignments redacted, 512 KB cap).
- **javac provider** (`build/compiler.go`): `Compiler.New(javaHome)` probes `javac -help` (English + Chinese "支持的发行版本") and `javac -version` to get `minSourceLevel`/`javaMajor`; **compat gate (PR01/F01)** rejects requested `-source/-target` below the JDK minimum with `toolchain_incompatible` (no silent elevation). Always passes `-encoding`, `-g`, `-Xlint:all,-options`, `-source/-target`; builds `@argfile` when command line >24 KB or >50 sources on Windows, with JDK-aware charset (JDK≥21 UTF-8, else GBK on Windows) and safe fallback to CLI. Uses `proc.ManagedProcess` for full process-tree cancellation; parses localized javac diagnostics (error/warning/错误/警告, optional `[javac]` prefix) and **validates output class-file major version** ≤ expected for target (1.6→50). Supports incremental source filtering and `CollectSources`/`ResolveClasspath`.
- **Ant provider** (`provider/runtime/ant_provider.go`): parses `build.xml` (project/property/target/depends/import/tasks), validates targets, runs `ant -buildfile ... [clean] [targets|compile]` with `ANT_HOME`/`JAVA_HOME`, 10m timeout, parses `[javac]` diagnostics, and emits a `debug="true"`/debuglevel warning when the build lacks LocalVariableTable.
- **Custom build** (`build/custom_executor.go`): detached 30m-bounded command execution for before-launch tasks.
- **JDT project model** (see §4.10).
- **Porting notes**: Preserve the no-silent-elevation rule, explicit `-encoding`, argfile charset logic, class-major gate, GBK post-compile verification, ant debug-info warning, and the async/poll shape.

### 4.5 Deploy engine & hot deploy
- **Behavior**: Two layers. (a) `services/diskDeployer` (`services/deploy.go`) implements the HTTP `Deployer`: merge/mirror directory sync with atomic per-file writes (`copyOneFile`: temp+chmod+rename), `pruneUnseen` mirror deletion with an explicit stack (avoids WalkDir cached-dir errors), `what:"static"` syncs only web extensions + merges `WEB-INF/classes`, refuses undeclared self-deploy (source==target), and recognizes `ExplodedLiveDir` (live docBase) as a truthful zero-file no-op. (b) `internal/deploy` (`plan.go`, `sync.go`, `atomic.go`, `package.go`) is the hardened primitive: `PlanBuilder.Build` requires a valid `DeploymentOwnerToken`, canonicalizes root, validates relative targets (no absolute/traversal/duplicates/collisions), rejects symlink escapes segment-by-segment, restricts mirror mode to a Kairo-owned exploded webapp root (`.../runtime/servers/<id>/webapps/<ctx>`), protects `.kairo`, and `safeDeployEngine.Execute` copies/deletes with pre/post ancestor rechecks and atomic replace.
- **Hot deploy**: `HotReloadWatcher` (`provider/runtime/hot_reload.go`) polls webapp/source dirs every 2s using size+mtime cheap stamps and SHA-256 content hashes, syncs static files to DeploymentDir (direct-docBase mode = no sync), and calls a compile callback for `.java` changes (`synced`/`compiling`/`restart_required` statuses published as events). `POST /jvm/compile{-incremental}` triggers incremental javac; `POST /jvm/redefine` performs live JDWP `RedefineClasses`. `POST /servers/{id}/reload` does a verified restart (context `reloadable=false` — touching web.xml alone would lie).
- **Porting notes**: Keep owner-token verification at the primitive, sandbox authorization at the HTTP boundary, mirror-root ownership check, atomic temp+rename, and restart-based context reload.

### 4.6 Tomcat 6 lifecycle & process identity
- **Behavior**: `tomcat6.Start(spec)` prepares a per-instance CatalinaBase (`conf/`, `server.xml`, `logging.properties`, `logs/`, `webapps/`, `temp/`, `work/`), builds the command `java -cp bootstrap.jar;tomcat-juli.jar -Dcatalina.home/base ... -Dfile.encoding=<enc> [--add-opens... only on Java 9+] [-agentlib:jdwp ...] org.apache.catalina.startup.Bootstrap start`. `server.xml` uses one HTTP connector bound to `127.0.0.1`, `autoDeploy=false`, `reloadable=false`, explicit `<Context docBase=<WebappDir>>` (shadows no ROOT; avoids classloader crashes on modern JDKs), access-log valve. Readiness: `WaitForContextReady` probes a real static resource of the context (strict 2xx/3xx/401/403), not just `/`. `WaitForPort` for JDWP; `SendShutdown` writes `SHUTDOWN` to the shutdown port. Stop is graceful (shutdown port + grace timeout) then force (process group); a 40-line log tail is attached to readiness errors.
- **Process identity verification** (`internal/proc`): every started process gets a random `MarkerToken` via env `KAIRO_PROCESS_MARKER`, and `ProcessIdentity{PID,Executable,StartTime,CatalinaBase,MarkerToken}`. `identityMatches` (Go level) and OS-level `verifyProcessIdentity` (Windows: OpenProcess + QueryFullProcessImageNameW + GetProcessTimes; Unix: `/proc` stat) must both pass before any signal — defeats PID reuse. Windows: process-wide **Job Object** with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` reaps all children when the agent dies; Unix: new process groups + explicit `Container.Shutdown` force-stop (the sole safeguard). `DecodeOutputBytes` decodes child output UTF-8-first then GBK/Big5/ShiftJIS/Windows-1252/1251.
- **Runtime plan** (`runtimeplan/resolver.go`, `ports.go`): resolves workspace/project/toolchain/runtime, canonicalizes roots, plans the CatalinaBase under `<dataRoot>/runtime/servers/<serverID>`, issues a target-bound `DeploymentOwnerToken`; ports allocated from ranges HTTP 18080–18999, shutdown 18005–18099, debug 18000–18049 with `PortLease` release.
- **Porting notes**: Reproduce explicit Context + autoDeploy=false, Java-version-gated `--add-opens`, strict context readiness, process identity checks before signaling, Job Object/process-group child reaping, and port ranges/lease semantics.

### 4.7 Security: sandbox, auth, audit
- **WorkspaceRoots** (`security/sandbox.go`): canonical roots (symlink/junction-resolved with nearest-existing-ancestor; Windows uses an opened handle via `read_file_path_windows.go`), `AddRootIfAbsent` runtime registration (bootstrap registers managed `webapps` dirs with a 1s reconciler), `AuthorizeRead/Write/ReadAbs/WriteAbs`, `OpenReadFileAbs`, lexical validation rejecting NUL/backslash/volume/UNC/absolute/traversal, read-only paths (bundled), `ErrPathForbidden`/`ErrSymlinkEscape`/`ErrPathReadOnly`. `pathpolicy.DefaultPathPolicy` adds `ResolveWithin`, `ResolveURIOrPath` (Theia `/g:/...` URIs), ID validation.
- **Auth**: loopback + shared secret (`X-Kairo-Secret`) or login sessions (Argon2id password hashing in `services/password.go`, `HashPassword` CLI helper); per-session CSRF; login throttle; rate limiter; safe-origin CORS; WebSocket subprotocol auth; audit events (categories, HMAC optional, daily rotation).
- **DeploymentOwnerToken** (`domain/project.go`): process-wide random `ownerSecret`; token = 32-byte random nonce + HMAC-SHA256(nonce|ws|proj|srv|root); `Valid()` weak check, `Verify(ws,proj,srv,root)` strong constant-time check; tokens die on restart (plans aren't persisted). Used by `ResolveDeploymentTarget` and `PlanBuilder`.
- **Fail-closed remote**: `remote/gateway.go` requires TLS 1.3, a ≥32-char bearer token (constant-time SHA-256 compare), TLS cert/key, explicit roots, rejects browser Origin, virtual `wN` workspace ids (never user-supplied absolute paths), no process-execution API. `kairo-remote/main.go` never opens a public listener.
- **Porting notes**: Keep fail-closed defaults (empty root set rejects all paths), symlink resolution via opened handles on Windows, marker-token + executable + start-time identity checks, and constant-time comparisons.

### 4.8 Encoding (GBK focus)
- **Behavior** (`internal/encoding/encoding.go`): `Detect(sample, defaultEnc, aliases)` returns `(id, confidence, hasBom, eol)`. Priority: BOM → XML `<?xml encoding?>` (0.98) → HTML `<meta charset>`/JSP `<%@ page pageEncoding/contentType %>`/`<jsp:directive.page>` (0.95) → valid UTF-8 (ASCII vs defaultEnc 0.55) → GB18030 validation (0.9) → decodable defaultEnc (0.6) → ISO-8859-1 (0.5). Trims trailing partial rune from sniff windows; canonicalizes aliases (gb2312/cp936→gbk, etc.). `Decode`, `StrictDecode` (rejects invalid UTF-8/GBK/GB18030/UTF-16 surrogates with byte offsets — required for write paths), `Encode` (strict GBK via `simplifiedchinese.GBK` so emoji fail), `ConvertEOL`, `PropertiesDecode/Encode` (ISO-8859-1 + `\uXXXX`, surrogate pairs for supplementary planes). Services layer clamps `sampleBytes` to [1,1MB], sandbox-authorizes read/write, caches per-file detected encoding, recode writes atomically.
- **Porting notes**: Must keep decoders (GB18030 for decode, GBK for encode), declaration parsing, strict write-path decode, BPM presence, EOL detection after UTF-16 decode, and the platform-output decoder in `proc`.

### 4.9 Search
- **Behavior** (`internal/search/search.go`): workspace text search with regex/literal, case/whole-word, include/exclude globs, context lines (hard cap 200, rejected above), max results (0=100k, negative=unlimited/1<<30), per-file 10 MB skip, encoding-aware decoding using project encoding + per-file header sniff, UTF-16 column accounting (`ColumnUtf16` for Monaco), replacement preview (empty replacement is valid). Parallel worker pipeline with `sync.Pool` buffers, deterministic stats, cancellation, symlink escape protection via `OpenFile`/`AuthorizeFile` hooks. `SearchStreamingWithStats` streams 50-match batches with single owned context, deterministic goroutine join, bidirectional cancellation, and stats (total/files/skipped/truncated/cancelled/duration). `ListFilesWithMeta` enumerates limit+1 for exact truncation. Typed errors (`invalidPatternError`) map to wire codes.
- **Porting notes**: Keep caps, encoding sniff + UTF-16 columns, per-file sandbox authorization, streaming cancellation/join semantics, and glob/exclude behavior.

### 4.10 JDT LS distribution & LSP bridge / JDT project model
- **Behavior**: JDT LS is pinned to **1.55.0** (release 2026-01-13, SHA-256 `90627c9f…`), installed offline only. Resolution order: `KAIRO_JDTLS_HOME` → `KAIRO_JDTLS_ARCHIVE` → `<bundled>/jdtls/<archive>` → `KAIRO_JDTLS_ARCHIVE_URL` (intranet/file://) → `ErrNotInstalled` (no public internet). Extract rejects zip/tar-slip; launcher JAR found by reading Equinox launcher plugin coordinate. Host JRE must be **JDK 21+** (`JDTLSRequiredJREMajor=21`) via `KAIRO_JDT_LS_JRE` → `KAIRO_JRE17_HOME` → bundled/jdk21 → JAVA_HOME; project compiler JDK stays legacy. As of Phase 4 the Go agent only serves a **launch descriptor** (`command`, args, `workingDir`, minimal `envAllowlist` — never `os.Environ()`); the Theia backend spawns JDT LS and owns LSP frames/stdio (`jdtls.go` retains deprecated `readHeaders` and Start/Stop for compatibility).
- **JDT project model** (`jdtproject/`): reads `.legacyflow/project.yaml` (or `.kairo/project.yaml`), optionally auto-detects classpath from `build.xml` via `antpath`, adds conventional `lib/`+`WEB-INF/lib` jars, and writes `.project` (`<projectDescription>` root, javanature + javabuilder), `.classpath` (src/output/con/lib entries, full `JRE_CONTAINER/.../JavaSE-<level>`), `KairoJavaConfig.ini`, and `.settings/org.eclipse.jdt.core.prefs`. Compliance clamped to **1.8** for the Eclipse model (JDT search engine fails below on host JDK 21) while `KairoJavaConfig.ini` keeps real 1.6; encoding written to both `compiler.encoding` and `encoding/<projectName>` to prevent GBK→U+FFFD corruption. JDT output dir is `build/jdt-classes`, separate from agent `build/classes`. `IntoProjectRoot` writes into the project root (relative src paths required by Eclipse). Fingerprint cache (payload hash + config/build mtimes) avoids regeneration.
- **Porting notes**: Preserve offline-only archive resolution, pinned SHA, JDK-21 host requirement, minimal env allowlist, project-root model placement, compliance clamp, and per-project encoding keys.

### 4.11 Java run / JUnit / managed debug sessions
- **Behavior** (`java_run_handlers.go`, `java_run_sessions.go`, `junitrunner/`): normalize Theia URIs, sandbox-authorize paths, resolve toolchain by project JDK, compile with `build.Compiler` (honoring project encoding/source/target, `-g` for debug) into `<projectRoot>/build/classes`, build classpath from lib/WebRoot-WEB-INF-lib/build-classes + `rt.jar`, run `java` (120s budget, kills process tree on timeout; distinct "timeout" state, no retry). JUnit4: single `@Test` method via caret-line source parsing → generated `KairoSingleMethod.java` runner using `Request.method` (never whole-class masquerading as method-scoped); else `JUnitCore`. Debug runs use `-agentlib:jdwp=...,suspend=y` and a **managed session** (`runId`, states starting→waiting-for-debugger→running/exited), returning as soon as JDWP listens (15s) rather than blocking on process exit (DBG-01 deadlock fix); logs capped at 512 KB. `/java/detect` parses main/test methods with comment/string stripping and an mtime+TTL cache.
- **Porting notes**: Keep non-retryable timeout, single-method runner honesty, JDWP-ready return for debug, and capped logs.

### 4.12 Debug / hotswap (JDWP)
- **Behavior** (`internal/debug/`): JDWP wire client (handshake, capabilities, event modifiers), breakpoints (line/exception), step control, stack frames, variables, source mapping, multi-VM orchestration. `HotSwapManager` keeps redefinition history + original-bytecode backups for rollback, validates class bytes & schema compatibility, supports `RedefineClasses` (JDWP 1.4+) / `RetransformClasses` (1.6+). `debug.RedefineClassLive` is used by `/jvm/redefine` with explicit target binding (`projectId`/`serverId`/`target`), rejects ambiguous/stale/mismatched targets (`ErrTargetAmbiguous`, `ErrStaleTarget`, `ErrTargetNotFound`), restricts JDWP host to loopback (SSRF guard), and expects `expectedHash`/`classLoaderId`.
- **Porting notes**: Never fall back blindly to "first running server"; verify generation/runtimeInstanceId; loopback-only JDWP.

### 4.13 Persistence / repositories
- **Behavior**: `internal/repository` provides file-backed repos with versioned documents (`VersionedDocument[T]`, schema validation, atomic JSON/YAML writes, `BackupCorruptFile`, `CorruptionError`), `.kairo/project.yaml` (source of truth with defaults + migration from `.legacyflow/project.yaml`), `RunConfigurationRepository` (bounded read, refuses to overwrite corrupt/oversized files, symlink-escape rejection, lossless concurrent creates), build/server history, project catalog. Services layer adds disk workspace/project stores, `finished.json`, `deployments.json`, `servers.json`, `toolchains.json`, `recent-projects.json`, `agent-state.json`. All writes use `atomicfile` (temp+fsync+rename, Windows `MoveFileExW` with sharing-violation retries, dir fsync).
- **Porting notes**: Keep atomic writes, versioned schema validation, corrupt-file non-overwrite, and per-project run-config files.

### 4.14 Events, logging & audit
- **Behavior**: `EventHub` is a per-workspace pub/sub with a monotonic sequence, 1000-event ring history, per-subscriber 256-buffer, `snapshot.required` and `event.gap` events when history can't cover a resume sequence or a slow consumer overflows; `StreamEvents` uses two-value receive to avoid busy-looping on the rejected-subscriber sentinel. WebSocket adapter serves `/events`. `log.Logger` writes structured JSON to stderr with component/request context and a redactor; `RingBuffer` feeds the diagnostic center. `audit.Log` appends NDJSON events with categories, optional HMAC signing, daily rollover + 90-day retention + size valve.
- **Porting notes**: Sequence/gap detection and safe channel handling are load-bearing; preserve event type strings (e.g. `build.*`, `server.*`, `hotreload.status`, `event.gap`).

### 4.15 Maven, SQL, remote tunnel (experimental)
- **Maven** (`internal/maven`): detect pom.xml, dependency tree (offline flag), whitelisted lifecycle tasks via `mvn` subprocess; settings parsing.
- **SQL** (`internal/sql`): pure-Go Oracle (`replace github.com/sijms/go-ora/v2 => ./third_party/go-ora`) with bind escaping, null projection, result budget (pagination + truncation), connection store keyed by `connectionId`; `/sql/*` uses 30s execution / 10s test timeouts, maxRows ≤100k, offset ≤1M.
- **SSH debug tunnel** (`-tags remote`): single active `SSHTunnel` forwarding local port → remote JDWP, ssh-key/token auth, reconnect disabled; default build returns "requires -tags remote".
- **Porting notes**: These are optional/experimental; the remote build tag isolates high-privilege SSH/docker/podman code from the default binary.

---

## 5. Tests that reveal behavior

- **Unit tests** exist for essentially every package (e.g. `domain/deployment_owner_token_test.go` proves nonce uniqueness/tamper rejection/zero-value invalid; `build/compiler_test.go`, `argfile_charset_test.go`, `jvm_compile_contract_test.go`; `encoding/strict_test.go`; `search/*_test.go` for context clamping, symlinks, unlimited, cancellation; `tomcat6/jdk6_compat_test.go`, `addopens_compat_test.go`, `explicit_deployment_flags_test.go`; `security/*_test.go` for sandbox bypass, junction, open-read-file; `proc/output_decode_test.go`, `job_assignment_windows_test.go`; `api/*_test.go` for CSRF, idempotency, login throttle, deploy sandbox, workspace isolation, run configurations).
- **`test/integration/smoke_test.go`** (tag `integration`): requires `KAIRO_LEGACY_SAMPLE`; walks health → workspace open/scan → GBK JSP encoding detect → search → real async javac build with Servlet API jar polling to success. `smoke_tomcat_test.go` additionally uses `KAIRO_TOMCAT6_HOME` for real start/stop.
- **`test/e2e/smoke_test.go`** (tag `e2e`) and `internal/test/integration/api_integration_test.go`: end-to-end API flows.
- **`test/fault/encoding_roundtrip_test.go`** (tag `fault`): GBK/UTF-8-BOM round trips, invalid/unknown encoding rejection, GBK content detection; `sandbox_bypass_test.go` covers path escape attempts.
- **Run commands** are documented in file headers, e.g. `KAIRO_LEGACY_SAMPLE=... go test -tags=integration ./test/integration/...`; note `./script/clippy` (not `cargo clippy`) and GPUI-flavored repo instructions do not apply to Go (use `go test ./...`).

---

## 6. Cross-cutting porting checklist

1. Reimplement the **wire contract** first (`internal/api/protocol/types.go` mirrors the TS protocol) — envelopes, error enum, DTO shapes, event type strings.
2. Wire production dependencies exactly as `services.NewMemoryServices` + `bootstrap.NewContainer` do; do **not** wire the `unwired` layer.
3. Preserve the **legacy-Java guarantees**: no silent `-source` elevation, explicit `-encoding`, GBK argfile charset, class-major gate, GBK post-compile check, JDK-21-gated `--add-opens`, JDK-21 JDT host, 1.8 compliance clamp for Eclipse.
4. Preserve **safety invariants**: loopback-only bind, shared-secret/session auth, CSRF, safe-origin CORS, workspace sandbox with symlink resolution (opened handles on Windows), owner tokens, process identity verification before signaling, Job Object/process-group child reaping, atomic writes, mirror-root ownership.
5. Preserve **idempotency/async** semantics: operation registry fingerprints, port handoff + spawn-before-shutdown restart, launch jobs + unobserved-launch guard, async build/deploy with restart reconciliation.
6. Preserve **encoding/decoding** on all child-process I/O and persisted Tomcat logs (`DecodeOutputBytes`).
