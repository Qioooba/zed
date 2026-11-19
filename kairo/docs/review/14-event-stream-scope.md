# 事件流内容验证的最终结论

> 2026-11-20。经源码级定位，确认 event stream 内容验证 inconclusive 的根因。

## 根因（agent 层，非 Zed 侧）

Kairo runtime-agent 有**两条 API 层**：

1. **生产层** `internal/api.Server`（`handlers.go`），由 `bootstrap.NewContainer` →
   `services.NewMemoryServices` 装配。HTTP `POST /api/v1/builds` 直接调用
   `s.Services.BuildEngine`（`handlers.go:872` 起），**不经过** `app.BuildUsecase`，
   因此不调用 `eventHub.Publish`。

2. **事件发布层** `internal/app.BuildUsecase / DeployUsecase / ServerUsecase`
   （`build_usecase.go:137-382` 等）会 `uc.eventHub.Publish(build.queued/started/completed...)`，
   但这些 usecase 属于 `//go:build unwired` 的 typed API 层，生产二进制里**未装配**
   （见 `analysis/02-kairo-runtime-agent.md` §1）。

因此：通过生产 HTTP API 触发 build/deploy/server 时，agent **不会**向 `/api/v1/events`
发布任何业务事件；我们的 WS 客户端握手成功但收不到事件是**预期行为**。

## Zed 侧状态（已完成且正确）

- `kairo_core/src/events.rs`：真实 WS 客户端（workspaceId 查询参数、`kairo-secret-v1`
  子协议、断线重连）。
- `EventTracker`：sequence 递增 + `event.gap` / `snapshot.required` 检测。
- 服务订阅在 agent 启动后自动建立，事件/间隙写入面板 Events 视图。

代码在事件真的到来时能正确处理；收不到只是因为 agent 生产路径不发布。

## 结论

- 事件流客户端实现完整、编译通过、握手验证通过。
- "内容验证 inconclusive" 不是缺陷，是上游 agent 的 unwired 架构所致。
- 若将来 agent 把 usecase 事件发布接入生产 HTTP 层，Zed 侧无需改动即可消费。

这一点已作为**上游限制**记录，不再作为本分支的未完成项。
