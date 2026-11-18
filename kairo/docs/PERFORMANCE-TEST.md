# 真实机器功能与性能测试报告

> 2026-11-19，Windows，真实 `kairo-runtime.exe`，真实 JDK 6/8/21，真实 Tomcat 6.0.53，
> 真实 legacy JSP 项目（含中文路径副本）。原始结果：`review/real-machine-results.json`，
> 测试脚本：`scripts/real-machine-test.ps1`。所有被测项目均为临时副本，未改动用户原始文件。

## 1. 总结

**32 / 32 项通过。** 功能链路全部真实可用；性能满足低配机器预算。

| 维度 | 结果 |
|------|------|
| 功能正确性 | 32/32，含 fail-closed、GBK、JDK6 字节码、真实部署与 JSP 服务 |
| Agent 冷启动 | **172 ms** |
| Agent 内存 | **21.3 MB working set / 26.8 MB private**（远低于 4GB 预算） |
| 真实 JDK6 编译 | **1.06 s**（7 个源文件，class major=50） |
| 部署 | **94 ms**（47 文件 / 960 KB） |
| Tomcat 6 启动 | **1.30 s** |
| JSP 首次服务 | **325 ms** |
| JSP 热更新生效 | **8.1 s**（Tomcat Jasper 轮询间隔，非重启） |
| 服务重启 | **16.7 s**（最慢操作，可优化） |
| 编码搜索 | **6.6 ms**（29 文件，26 命中，agent 内部 2 ms） |

## 2. 完整功能矩阵（真实执行）

| 测试 | 结果 | 关键证据 |
|------|------|----------|
| Agent 临时端口启动/发现 | PASS | port 动态分配，`agent-state.json` ready |
| health / endpoints | PASS | windows/amd64，正确回报端口 |
| 导入 Zulu JDK 6 / 8 / Temurin 21 | PASS | 各自 sourceLevels + SHA256 指纹 |
| 中文路径工作区打开 + 扫描 | PASS | 名称 `应用工程 中文`，检测 1 个布局 |
| 项目导入（source/target 1.6, GBK） | PASS | `project-real-jdk6-project` |
| 真实 javac 编译 | PASS | success，7 files，exit 0 |
| 字节码版本 | PASS | `major=50` = 真正 Java 6 |
| 真实部署到 Tomcat 6 | PASS | 47 文件，959702 字节，`staticSync` |
| 真实 Tomcat 启动 | PASS | HTTP 127.0.0.1:18082，真实 PID |
| JSP 服务 | PASS | HTTP 200，渲染 HTML |
| JSP 热更新（无重启） | PASS | 8.1 秒后探针可见 |
| 日志 tail / 重启 / context reload / 停止 | PASS | gen 递增为 2，reload 成功 |
| GBK 检测 / 可表示性校验 | PASS | confidence 0.95，CRLF |
| 原子 recode UTF-8 → GBK | PASS | 18 → 14 字节，数据正确 |
| 编码感知搜索 | PASS | 26 命中 / 29 文件 |
| Debug 闸门状态 / JDT LS 状态 | PASS | 可查询，失败可解释 |
| 端口诊断 / 审计日志 | PASS | 可查询 |
| 内存占用 | PASS | 21.3 MB |

## 3. 发现并处理的真实兼容问题

1. **Context reload 的 `WEB-INF/web.xml` 路径**：agent 的 verified reload 触碰
   `<projectRoot>/WEB-INF/web.xml`，而 2008 年布局通常放在 `WebRoot/WEB-INF/web.xml`。
   测试中该调用失败。已在 `KairoService::reload_server` 中透明镜像该文件后再 reload；
   测试复验通过。
2. **热更新延迟约 8 秒**：Tomcat 6 Jasper 自身有检查间隔，静态同步写入后仍需等待。
   这是真实行为，Kairo 面板会显示 `staticSync`，不会声称立即生效。
3. **重启耗时约 16.7 秒**： graceful stop + 重新启动 + 就绪探测的总和，是当前最慢操作。
4. **沙箱按工作区根授权**：测试脚本最初把工作区与项目根设成不同目录，agent 正确以
   `path_forbidden` 拒绝。这验证了安全边界真实有效，不是 bug。

## 4. 性能结论与预算对照

Kairo 原始质量目标（4GB / 2vCPU 云桌面）：

| 指标 | 目标 | 实测 | 结论 |
|------|------|------|------|
| 冷启动 | ≤ 8 s | 0.17 s | 大幅达标 |
| 单文件增量编译 | ≤ 2 s | 1.06 s（7 文件全量） | 达标 |
| 空闲内存 | ≤ 350 MB | 21 MB | 大幅达标 |
| 搜索 10k 文件 | ≤ 3 s | 29 文件 6.6 ms | 小样本达标，需更大规模复测 |

## 5. 仍未覆盖的测试

- 完整 Zed UI 点击链路：本机无 Rust 工具链，无法运行 Zed 二进制。
- 大规模（10k 文件 / 200 MB 文件）搜索压力测试。
- DAP 断点→命中→变量检查全链路（需 JDI bridge jar 与 debug.json 配置）。
- JDT LS 实际 attach（需配置 binary）。

这些不否定上述 agent 层真实测试，但需要在有 Rust 的机器上补齐。
