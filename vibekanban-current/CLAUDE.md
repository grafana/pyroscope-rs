# pyroscope-rs-design-docs

Design documentation for building a new profiler (pyroscope-rs). This repo contains the main design document and research on existing profilers.

## Repo Structure

- `design.md` — main design document for the new profiler
- `other/` — analysis of existing profilers (async-profiler, gperftools, pprof-rs, v8, Go 1.26 runtime profiler)
- `other/candidates.md` — list of profiler candidates considered during research

## Issue Tracking

Use the Vibe Kanban MCP server for all issue tracking. Do **not** use markdown files or git for task management.

Rules:
- **Only create issues** — never create workspaces
- **Never change issue status** — status is managed by the user, not agents
- If an issue depends on another and must not be started until the blocker is done, use `create_issue_relationship` with type `blocking` to link them

Key tools:
- `list_issues` — view current issues
- `create_issue` — create a new issue
- `update_issue` — update an issue's title or description (not status)
- `get_issue` — get details on a specific issue
- `create_issue_relationship` — link issues (e.g. blocker → blocked)
