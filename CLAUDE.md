# CLAUDE.md — CAD2Maxwell

Use `CAD2Maxwell_PROJECT_SPEC.md` as the product source of truth.

## Core priorities

1. geometric correctness,
2. traceability from source CAD to 2D region,
3. deterministic export,
4. engineering-quality UX,
5. maintainable architecture.

## Critical constraints

Never:
- replace exact CAD sectioning with triangle-mesh slicing,
- discard source names/colors without explicit reason,
- use diacritics in Maxwell export names,
- put OpenCascade objects into frontend state,
- silently close nontrivial gaps,
- union separate source components by default,
- approximate analytic arcs with polylines unless required by an explicit export option.

## Preferred architecture

```text
React/TypeScript UI
        ↓
Tauri/Rust shell
        ↓
Local Python API
        ↓
OpenCascade exact geometry
```

## Implementation behavior

Before editing:
- inspect existing patterns,
- preserve architecture,
- prefer small coherent changes.

After editing:
- run relevant tests,
- report failures honestly,
- add regression tests for geometry defects.

## Project style

This is a CAE/CAD engineering application.

Prefer:
- tables,
- panels,
- tree views,
- property inspectors,
- diagnostics,
- compact toolbars.

Avoid:
- oversized cards,
- decorative dashboards,
- excessive animations,
- vague status messages.

## Long-term compatibility

Keep the core usable without Autodesk Inventor and without ANSYS installed.

Integrations belong behind adapters.

Potential integrations:
- Inventor COM/C# Add-In,
- PyAEDT,
- Motor-CAD.

Do not make them hard dependencies of the geometry core.
