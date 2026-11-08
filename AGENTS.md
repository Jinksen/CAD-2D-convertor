# AGENTS.md — CAD2Maxwell

## Mission

Build CAD2Maxwell as a robust engineering preprocessor for converting exact 3D CAD geometry into validated ANSYS Maxwell 2D geometry.

Read `CAD2Maxwell_PROJECT_SPEC.md` before substantial changes.

## UI rewrite direction

The project is migrating its desktop UI to the native Rust [Rune](https://github.com/anomalyco/rune) framework. Read [docs/rune-migration.md](docs/rune-migration.md) before UI or desktop-shell work; it defines the migration plan, compatibility gates, pinned Rune revision, and the one-worktree/one-PR policy for the rewrite.

Treat the React/Vite/Tauri UI as the current production implementation, not the target architecture. Keep it working until the Rune replacement passes the documented parity and packaging gates. Do not start a broad rewrite or remove the current UI as part of an unrelated task.

For the full rewrite, use exactly one dedicated worktree and branch, keep the migration commits together there, and open only one PR after the complete app works and passes its gates. Do not create intermediate PRs or additional rewrite worktrees.

The migration changes the desktop UI and shell, not the geometry authority: Python/OpenCascade remains the only source of exact CAD geometry, sectioning, validation, and export. Preserve the typed local-service boundary and never move geometry algorithms into the UI.

## Architectural rules

- Rune/Rust = target desktop UI; Tauri + React + TypeScript = current production UI during migration.
- Python/OpenCascade = geometry authority.
- Rust = thin desktop/service bridge.
- Geometry logic must not live in the UI.
- UI must not depend on OpenCascade internals.
- Use typed DTOs across process boundaries.
- Preserve source component identity and metadata.
- Exact B-Rep sectioning is mandatory; mesh slicing is not an acceptable substitute.
- Export names for Maxwell must be deterministic and ASCII-safe.
- Do not silently repair significant geometry.

## Work style

For each task:
1. inspect relevant code,
2. write a compact plan,
3. implement a coherent vertical slice,
4. run focused tests,
5. run broader tests,
6. summarize changes and limitations.

Avoid giant rewrites.

## Working inside PyCharm (MCP)

When the `pycharm` MCP server is connected, prefer its IDE-aware tools over plain shell/text tools:

| Need | Use |
|------|-----|
| Project layout | `list_directory_tree`, `get_project_modules`, `get_project_dependencies` |
| Interpreter before running Python | `get_python_environment` (backend venv is `backend/.venv`, uv) |
| Find code | `search_symbol` for identifiers, `search_regex` / `search_text` for text |
| Understand a symbol | `get_symbol_info` (quick docs, declaration) |
| Who calls what | `analyze_calls` (call hierarchy) instead of grepping for usages |
| Rename | `rename_refactoring` (updates all references), never search-and-replace |
| After every edit | `lint_files` / `get_file_problems` on touched files, then `reformat_file` |
| Build / type errors | `build_project` |
| Run tests and apps | `get_run_configurations` + `execute_run_configuration`; otherwise `execute_terminal_command` |
| Repo state | `git_status`, `get_repositories` |
| Show the user a file | `open_file_in_editor` |

Notes:
- Run pytest with `--basetemp .test-tmp/<name>` (as `scripts/test.ps1` does); the system temp dir fails with `PermissionError` on this machine.
- PyCharm warns "OCP not listed in project requirements" — OCP comes from the `cadquery-ocp-novtk` wheel; ignore that warning.
- IDE inspections do not replace `scripts/test.ps1` (ruff, mypy, eslint, tsc, vitest, cargo clippy).

## Source-file guidelines

- Prefer <300 lines.
- Reconsider design around 500 lines.
- Keep functions focused.
- Separate domain logic from IO and UI.
- Do not create generic dumping-ground modules.

## Python

- Python 3.12+
- typed code
- Pydantic DTOs
- Ruff
- pyright/mypy
- pytest
- central tolerance configuration

## TypeScript

- strict mode
- no `any` without documented reason
- feature-oriented folders
- Zustand for local workspace state
- keep rendering and backend contracts separated

## Rust

Keep Rust thin:
- native dialogs,
- desktop lifecycle,
- local Python process supervision,
- packaging/security.

Do not port the geometry engine to Rust unless explicitly requested.

## Testing

Every geometry bug should become a regression test.

Do not hard-code production behavior to one specific motor model.

## UX

Design for engineers:
- compact,
- information-dense,
- keyboard-friendly,
- clear selection,
- clear diagnostics,
- actionable errors,
- no consumer-dashboard visual clutter.

## Safety for user data

- Local processing by default.
- No cloud upload.
- No telemetry by default.
- Do not execute code embedded in imported files.

## Definition of done

A feature is not done until:
- typed,
- tested,
- errors handled,
- relevant docs updated,
- no known architectural violation.
