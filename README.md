# CAD2Maxwell

CAD2Maxwell is a Windows desktop engineering preprocessor that converts exact 3D CAD assemblies into validated 2D geometry suitable for ANSYS Maxwell 2D.

## Planned stack

- Tauri
- React
- TypeScript
- Python 3.12
- FastAPI
- OpenCascade/OCP
- ezdxf
- Three.js / React Three Fiber

## Start here

Read:

1. `CAD2Maxwell_PROJECT_SPEC.md`
2. `AGENTS.md`
3. `CLAUDE.md`

The specification contains the architecture, UX, geometry pipeline, milestones and Codex implementation tasks.

## Core principle

This is not a generic STEP-to-DXF converter.

The product must preserve traceability between source CAD components and resulting 2D electromagnetic simulation regions.
