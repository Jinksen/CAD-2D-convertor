# AGENTS.md — CAD2Maxwell

## Mission

Build CAD2Maxwell as a robust engineering preprocessor for converting exact 3D CAD geometry into validated ANSYS Maxwell 2D geometry.

Read `CAD2Maxwell_PROJECT_SPEC.md` before substantial changes.

## Architectural rules

- Tauri + React + TypeScript = desktop UI.
- Python/OpenCascade = geometry authority.
- Rust = thin desktop/process bridge.
- Geometry logic must not live in React.
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
- Tauri lifecycle,
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
