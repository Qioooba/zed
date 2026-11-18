# 专项测试：热部署 / Debug / JSP·Java 高亮

> 目标：这四项做到可用、可验证、不撒谎。全部在本机真实工具链上执行。

## 1. Debug — 真实 DAP/JDWP 全链路 PASS

脚本：`scripts/dap-e2e-test.py`，可重复运行。

真实流程：agent 启动带 JDWP 的 Tomcat 6（JDWP 端口 18003）→ Python 作为 DAP
客户端通过 **stdin/stdout** 驱动 `kairo-jdi-bridge.jar` → `initialize` → `attach`
→ `setBreakpoints`（`HelloServlet.java:26`，延迟断点在类 Prepared 时绑定）
→ `configurationDone` → HTTP 触发 `/hello?name=Kairo` → 命中断点 →
`stackTrace`（顶帧 `doGet`）→ `scopes` → `variables`（`name = Kairo`）→
`continue` → `disconnect`。

| 步骤 | 结果 |
|------|------|
| initialize | PASS |
| attach (host 127.0.0.1, port 18003) | PASS |
| deferred breakpoint bound on ClassPrepare | PASS |
| stopped event / top frame `doGet` | PASS |
| local variable `name` | `"Kairo"` |
| continue / disconnect | PASS |

**纠正的真实错误**：JDI 桥使用 Content-Length 帧的 **DAP-over-stdio**，原先
Zed 侧错误地按 TCP 附加。现已改为 Zed DAP 适配器直接 `java -jar bridge <host>
<port>` 并持有 stdio 管道。桥路径与真实 JDWP 端口通过 worktree 内
`.zed/kairo-integration.json` 自动传给扩展，不再需要用户手配。

## 2. 热部署 — 三种真实模式 PASS

脚本：`scripts/hot-deploy-test.py`。

| 模式 | 实测 | 说明 |
|------|------|------|
| `staticSync` | JSP 改动 **2.3 s** 后在浏览器可见，未重启 Tomcat | agent watcher + Tomcat Jasper 轮询 |
| `compileOnly` | 真实完整构建 **1.06 s** 成功 | agent 的 `/jvm/compile*` 在 Windows 上存在路径策略 bug（已实测确认），故走可靠的 `/builds`，并如实标注 `restart_required` |
| `contextReload` | `POST /servers/{id}/reload` 返回 `reloaded`，服务器保持 running，约 3 s | 需先 touch `<root>/WEB-INF/web.xml`；Tomcat context reload 在同一 JVM 内，generation 不一定变化，这是真实语义 |

**纠正的真实错误**：
- legacy 布局的 `web.xml` 在 `WebRoot/WEB-INF/`，agent 的 reload 期望项目根
  `WEB-INF/web.xml` → Zed 侧现在透明镜像。
- reload 前必须 touch web.xml，否则只是 no-op。
- `/jvm/compile*` 拒绝 Windows 原生路径（agent 内部 bug），改用已验证的真实构建。

## 3. JSP / Java 高亮 — 查询全部通过语法节点校验

脚本：`scripts/validate-grammar-queries.py`（从 GitHub 拉取固定 commit 的
`node-types.json`，校验 `.scm` 中每个节点名/字面量确实存在）。

```
jsp/highlights.scm: 26 named nodes, 4 literals checked
jsp/injections.scm: 2 named nodes
java/highlights.scm: 22 named nodes, 37 literals checked
RESULT: PASS
```

改进内容：
- JSP：覆盖 directive / scriptlet / EL / tag / attribute / doc_type / 错误标签 /
  `{{ }}` 定界符，并把 `(jsp)`、`(interpolation)` 注入 Java，使 scriptlet 与
  EL 获得 Java 高亮，贴近 Kairo 的 region scanner。
- Java：覆盖注释、各类字面量、类型/方法/字段/参数、注解、import/package、
  全部关键字与基本类型节点（`boolean_type`/`void_type` 等）。

## 4. 仍需注意

- 本机没有 Rust 工具链，WASM 扩展未实际加载；查询校验是静态 + 真实 grammar 节点表双重验证。
- HotSwap（`/jvm/redefine`）依赖 JDWP 目标绑定，未纳入本轮专项测试；当前 Java 改动如实报告 `restart_required`，不会谎称已热替换。
