# 09 — 1080p 前端页面 / UI 截图审核

审核对象：`kairo/ui-prototype/kairo-panel-1080p.html`（按 `KairoPanel::render`
的真实分区、按钮、状态和文案 1:1 还原的 1920×1080 原型）。

截图：

- `kairo/ui-prototype/screenshots/kairo-panel-1080p.png` — 正常运行态
- `kairo/ui-prototype/screenshots/kairo-panel-1080p-error.png` — fail-closed 错误态

方法：浏览器实际渲染后截图，逐区域从"维护 2008 年 JSP 项目的工程师"视角检查。

## 1. 总体评价

- 信息密度合适：编辑器占主区域，Kairo 面板固定在底部 372px，不遮挡代码。
- 四个关键事实（active 项目、工具链/指纹、编码、Tomcat 状态）都在一屏内。
- 主操作按钮（Compile / Build & Deploy / Start Server）用蓝色强调，破坏性 Stop
  用红色文字，符合"一个主操作"原则。
- 失败态红条非常醒目，且给出"a real JDK 6 is required"的可执行指引，不是空状态。

## 2. 截图发现的问题与修复

| # | 问题 | 用户影响 | 修复 |
|---|------|----------|------|
| 1 | 面板缺少 Scan / Select Project / Open App / Save with Encoding 按钮，只能从命令面板进入 | 新用户不知道如何开始；GBK 安全操作难发现 | 已在 `KairoPanel` 增加四个具名按钮，Scan 使用当前 worktree 根路径触发 `bootstrap` |
| 2 | 编码在 meta 行和状态栏各出现一次，文案不一致 | 轻微困惑 | 统一为 `gbk · CRLF`，状态栏作为只读摘要 |
| 3 | 日志区为单行不换行、非等宽文本 | 200 行上限正确，但可读性差 | 已加 `overflow_y_scroll()` + `font_monospace()` |
| 4 | Stop 后仍每 2 秒轮询已停止服务器 | 无意义刷新，低配机器浪费 CPU | `stop_server` 中清空 `poll_task` 与 `hot_reload_mode` |
| 5 | `ctrl-alt-shift-i/o` 与 Linux/macOS 默认键位冲突 | Mac 旅行场景按键被劫持 | 改为 `ctrl-alt-shift-u`（Hot Deploy）和 `ctrl-alt-shift-y`（Debug） |
| 6 | 早期返回错误（无 agent / 无项目 / 无成功构建）只进日志 | 用户点了按钮没反应 | `KairoService::unavailable()` 把所有前置失败写入 `last_error`，面板红条显示 |
| 7 | 首次使用无项目：只有 Scan 能启动 agent，且有异步竞态；scan 不导入项目 | 新用户完全无法开始 | 新增 `bootstrap()`：一次性串行完成 启动 agent → 打开工作区 → 扫描 → 导入工具链 → 导入项目 → 激活项目 |
| 8 | `SelectProject`/`SearchEncoded`/`ReconnectAgent` 是空操作 | 按钮撒谎 | SelectProject 改为轮换项目；ReconnectAgent 真正重连；删除无查询输入的 SearchEncoded 假动作 |
| 9 | HotDeploy 的 Java 分支无调用者 | 热部署只剩普通 Deploy | 动作读取当前编辑器文件，`.java` 走 `/jvm/compile`，其余走静态同步 |

## 3. 仍需在真实 Zed 窗口验证的点

- 面板实际字体/间距与原型的一致性（原型使用 Segoe UI，Zed 使用系统 UI 字体）。
- 点击按钮后的红条高度变化时，日志区是否仍保持可见（原型中可见）。
- DAP 调试需要用户先写 `debug.json`；端口自动注入尚未实现。

## 4. 结论

1080p 原型通过信息架构与可读性审核；审核中发现的 9 项问题均已在代码中修复或
明确记录为待验证项。首次使用链路（最重要缺陷）已由 `bootstrap()` 串行化解决。
