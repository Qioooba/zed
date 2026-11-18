# Kairo for Zed — 用户体验设计

> 目标用户：维护 2008 年 JSP/Servlet 项目的工程师，机器配置低、内网、JDK 6、
> GBK 文件。设计原则来自 Kairo `ui-spec.md`：4 态视图、一个主操作、分页日志、
> 状态栏一眼可见、不撒谎（fail-closed 必须显式呈现）。

## 1. 信息架构

Zed 侧只增加一个停靠面板 **Kairo**（默认底部，可拖到右侧），不增加顶层菜单，
避免与 Zed 原生菜单冲突。所有操作同时进入命令面板（F1 / Ctrl+Shift+P），
命令以 `kairo::` 命名空间前缀，例如 `kairo::Compile`。

理由：审核发现 Zed 的 `assets/keymaps/default-*.json` 中 `ctrl-alt-b/d/s/p`
等组合均已被占用，因此默认键位改用未被占用的 `ctrl-alt-shift-*` 组合：

| 操作 | 键位 | 说明 |
|------|------|------|
| Compile | `Ctrl+Alt+Shift+J` | 单独编译 |
| Deploy | `Ctrl+Alt+Shift+M` | 部署最近一次成功构建 |
| Hot Deploy | `Ctrl+Alt+Shift+U` | 按文件类型决定静态同步或增量编译 |
| Start Server | `Ctrl+Alt+Shift+V` | 启动真实 Tomcat 6 |
| Debug | `Ctrl+Alt+Shift+Y` | 先过 Java 6 闸门，再起桥并附加 |
| 面板开关 | `Ctrl+Alt+Shift+K` | 显示/隐藏 Kairo 面板 |

用户可在 Zed 键位文件中自由重绑定；Kairo 不修改上游 keymap 文件。

## 2. 面板分区（自上而下）

1. **状态行** — "Kairo agent connected at http://127.0.0.1:port" / "Build X is success (7 files)"，
   始终一句人话，不用颜色单独表达状态。
2. **编码与热部署行** — `Encoding: gbk | Hot reload: staticSync / compileOnly → restart_required`。
   这是数据安全和诚实状态的核心，必须常驻。
3. **错误行（仅失败时出现）** — `ERROR: toolchain_incompatible: a real JDK 6 is required for
   source level 1.6; importing a modern JDK is not a substitute`。绝不把失败折叠成空状态。
4. **操作按钮组** — Compile / Clean Build / Build & Deploy / Deploy / Hot Deploy /
   Start Server / Debug / Stop / Refresh。主操作（Compile、Start Server）靠前，
   破坏性操作（Stop）靠后，按钮文案用动词短语而非图标。
5. **四列状态区** — Projects（当前项目用 `*` 标记）/ Toolchains（含 SHA 指纹状态）/
   Toolchain Slots（LS JRE、Compiler JDK、Tomcat JDK、source/target、encoding）/
   Servers（状态、generation、端口）。
6. **日志区** — 最多 200 行，倒序显示，永不无限滚动（对应 4GB 机器的硬约束）。
7. **检测到的布局（有结果时）** — 扫描出的项目路径、构建系统、警告。

## 3. 关键交互流程

### 首次打开 legacy 项目
1. 用户按 `kairo::ScanProject`（或点击 Refresh）→ agent 自动以临时端口、仅回环地址启动；
2. 扫描 `src/`、`WebRoot/`、`lib/`、`build.xml`、`web.xml`、编码声明；
3. 若只有一个项目，自动设为 active，并把设置中的 source/target/encoding 写回项目配置；
4. 状态行提示"Detected 1 legacy project layout"。用户无需导入向导即可开始（向导可后续补）。

### 一键编译-部署
`Build & Deploy` 按钮 → 提交构建 → **轮询到终态**（500ms 间隔，最多 120s）→
成功才触发部署；失败则把 agent 的真实错误码写进错误行。杜绝"永远 queued"的假绿。

### 调试
`Debug` → 调 `/debug/adapter/status` 做 Java 6/Tomcat 6/JDWP 闸门检查 →
失败立即在错误行说明原因；通过后启动带 JDWP 的 Tomcat，由 kairo_core 启动
`kairo-jdi-bridge.jar` 并等待其 TCP 端口就绪，然后 Zed 的 `java` DAP 适配器附加。
调试端口用 agent 实际返回的 `ports.debug`，不再硬编码 8000。

### GBK 文件保存
Zed 缓冲区是 UTF-8，`Ctrl+S` 不能保证字节保真。因此提供显式
`kairo::SaveWithEncoding`：取当前编辑器文件的绝对路径 → `/encoding/detect` →
`/encoding/recode`（严格解码、可表示性检查、临时文件 + fsync + 原子改名）→
保存后更新面板编码行。面板编码行常驻提醒用户使用此命令。

## 4. 状态语义（不许撒谎）

| 状态 | 含义 | UI 文案 |
|------|------|---------|
| `success` + `staticSync` | JSP/CSS 已即时生效 | `staticSync` |
| `compileOnly` | class 已同步，但 Tomcat `reloadable=false` | `compileOnly → restart_required` |
| `classHotSwap` 无调试器 | 未真正热替换 | `restart_required` |
| `contextReload` | 已做可验证重启 | `contextReload` |
| `toolchain_incompatible` | 无真实 JDK 6 | 错误行 + 明确指引 |

## 5. 真实项目验证记录

见 `VERIFICATION.md`：真实 Zulu JDK 6（`javac 1.6.0-119`）编译成功，
生成 class 文件 `major=50`（Java 6 字节码）；真实 Tomcat 6 提供 JSP 200；
修改 JSP 后 3 秒生效且未重启，证明热部署路径真实可用。
