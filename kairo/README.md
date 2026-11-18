# Kairo-for-Zed Integration

This directory is the home of the effort to bring the capabilities of the
**Kairo IDE** (`G:\spaces\kairo-ide`) into the **Zed** editor (`E:\github\zed`).

Kairo IDE is a focused IDE for legacy Java Web projects
(JDK 1.6 / Tomcat 6 / Servlet / JSP / GBK encoding). This integration aims to
give Zed the same real, clickable compile / launch / deploy / debug /
syntax-highlighting / hot-deploy workflow, while keeping the diff against
upstream Zed small enough to rebase on future Zed releases.

- Tracking branch: `kairo/integration` (created from `main`).
- All new files live under `kairo/` plus a small, explicit set of registration
  edits in upstream files (documented in `docs/SYNC-STRATEGY.md`).
- Analysis reports from the parallel agents live in `docs/analysis/`.

Start here:

- `docs/PROGRESS.md` — living status / task board / decisions.
- `docs/analysis/` — deep analysis of Kairo IDE and of Zed's extension points.
- `docs/ARCHITECTURE.md` — target architecture (evolving).
- `docs/SYNC-STRATEGY.md` — how to keep rebasing on upstream Zed.
