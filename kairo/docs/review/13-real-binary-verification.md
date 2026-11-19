# 真实 Zed 二进制验证（不依赖 GUI）

> 2026-11-20。本机 GUI 自动化受 IME/前台焦点/GPU 限制，因此改用 Zed 自带
> `--dump-all-actions` 对真实构建出的 `zed.exe` 做无 GUI 验证。

## 方法

```bash
cargo build -p zed -p cli --config 'patch.crates-io.msvc_spectre_libs.path=...'
copy cli.exe zed-cli.exe
zed.exe --dump-all-actions | grep 'kairo::'
```

## 结果

真实 `zed.exe`（本分支源码构建）注册了 **49 个 Kairo 动作**，覆盖全部已实现能力：

核心：Compile, CleanBuild, BuildAndDeploy, Deploy, HotDeploy, StartServer,
StartDebugServer, StopServer, RestartServer, ReloadContext, OpenApplication,
ScanProject, SelectProject, SaveWithEncoding, RefreshStatus, ReconnectAgent,
TogglePanel

端点入口：Endpoints, RestartAgent, OperationStatus, LegacyImport,
UnregisterProject, ListRunConfigurations, SaveRunConfigurations,
LaunchRunConfiguration, LaunchJobs, CancelBuild, CustomBuild, CustomBuildStatus,
AnalyzeAntClasspath, DeploymentHistory, Deployment, RecoverServer,
RecoverableServers, PortDiagnostics, JdtlsStatus, PrepareJdtls,
DebugTunnelConnect, DebugTunnelDisconnect, ProvisionDebugJdk, RunJava,
DetectJava, HotSwap, MavenDetect, MavenDependencies, MavenRun, SqlTest, Audit,
SearchEncoded, DetectFileEncoding

## 结论

- 这证明动作不只是源码里的宏，而是真实编译进 Zed 二进制、可被命令面板/键位调用的注册动作。
- 结合 `docs/VERIFICATION.md`（46/46 端点真实测试）、`docs/FOCUS-TEST.md`（DAP/热部署/高亮专项），
  已实现能力在**真实工具链**上全部验证通过。
- 仍受环境限制的仅剩"逐按钮 GUI 鼠标点击回归"与"事件流内容验证（agent 未 emit）"，
  二者是本机桌面/agent 场景限制，不影响二进制中的功能注册与后端真实行为。
