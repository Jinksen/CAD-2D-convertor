# Rune desktop UI migration

## Decision and status

CAD2Maxwell's target desktop UI is a native Rust application built with [Rune](https://github.com/anomalyco/rune). The current React/Vite/Tauri application remains the production implementation until the replacement passes the acceptance gates below. This is a UI and desktop-shell migration, not a geometry-engine rewrite.

The Rune repository describes itself as a native GPU-rendered UI framework and provides native windows, accessibility, file-drop handlers, native file dialogs behind a feature, and headless rendering/input tests. It also exposes vector `Shape`/`Path` rendering. It is pre-release (`0.0.1`), its renderer is new to this project, and the repository asks consumers to pin a `dev` revision. The evaluation spike pins the revision current on 2026-11-08:

```text
a7f276c3630f85a06a0230295da4d2bd172d8d3b
```

References:

- [Rune repository and framework overview](https://github.com/anomalyco/rune)
- [Rune core API and capabilities](https://github.com/anomalyco/rune/blob/a7f276c3630f85a06a0230295da4d2bd172d8d3b/crates/rune/README.md)
- [Rune hello example and headless interaction test](https://github.com/anomalyco/rune/blob/a7f276c3630f85a06a0230295da4d2bd172d8d3b/examples/hello/src/main.rs)
- [Rune shape rendering example](https://github.com/anomalyco/rune/blob/a7f276c3630f85a06a0230295da4d2bd172d8d3b/examples/shapes/src/main.rs)
- [Rune native file-drop example](https://github.com/anomalyco/rune/blob/a7f276c3630f85a06a0230295da4d2bd172d8d3b/examples/drop-files/src/main.rs)

The revision is an initial reproducibility pin, not a promise to follow `dev` automatically. Update it only through a deliberate dependency review and rerun the spike and gates.

## Target boundaries

```text
Rune native UI
    -> Rust desktop/service adapter
        -> authenticated loopback FastAPI service
            -> Python/OpenCascade exact geometry
```

- Python/OpenCascade remains the only owner of STEP/XCAF, exact B-Rep topology, sectioning, validation, and DXF/manifest generation.
- Keep the service's loopback binding, per-session authentication, typed DTOs, error handling, and local-only data flow.
- Rust may own the native window, keyboard/selection state, file dialogs, and service/process lifecycle. Keep it a thin adapter; do not port domain geometry to Rust.
- The UI may prepare display-only representations from DTOs. It must not mutate/export those representations as if they were exact CAD geometry.
- Remove React, Vite, Three.js, and Tauri only after the Rune replacement is the default production app and all parity gates pass.

## Why a staged migration

Rune is a UI framework, not a drop-in replacement for React or Tauri. Replacing the webview shell changes event handling, dialogs, startup/packaging, rendering, tests, and existing Rust integration. Rune's 2D `Shape` examples do not establish that it can render CAD-scale analytic sections efficiently, nor do they establish a production 3D mesh viewer or a public way to embed an independently rendered `wgpu` surface. Those are early feasibility questions, not details to defer until the end.

## Rewrite collaboration and delivery

Perform the complete rewrite in exactly one dedicated worktree on one branch. Keep all migration commits together on that branch, including the feasibility work and implementation. Do not split phases into separate worktrees, stacked branches, or intermediate PRs. Open one PR only when the Rune app is complete, functional, and passes the acceptance gates; merge only that final PR. If a feasibility gate blocks the rewrite, report the blocker in that same worktree rather than opening a partial migration PR.

## Migration phases

### 0. Evaluation spike

`frontend/rune-ui-spike` is deliberately separate from the current app and pins Rune to the revision above. It is a small runnable native workspace scaffold with a CAD-style panel layout and a headless element-tree test. Keep it a spike; do not wire it into release packaging or create a second production app yet.

Extend the spike before choosing the final renderer or shell integration. Record results for:

1. Windows build and launch, window resizing, keyboard input, high-DPI behavior, and headless snapshot/test workflows.
2. 2D rendering and input using captured section DTO fixtures at small, typical, and large entity counts (include lines, arcs, circles, holes, colors, selection, diagnostics, pan, zoom, and fit).
3. 3D preview of representative backend meshes, camera controls, component picking/highlighting, clipping/section-plane display, and memory/frame-time behavior. Prove a supported integration path; do not assume Rune's internal `wgpu` use exposes a surface for custom rendering.
4. Native open/save dialogs and STEP file drop; then call the existing authenticated local API and show actionable loading/backend errors.
5. Accessibility tree/native UI Automation, themes, window shutdown, packaged startup, and offline behavior.

The spike passes only when the target meets current user-visible needs with a maintainable, testable renderer and service integration. If Rune lacks a viable 3D path or fails representative 2D performance/interaction tests, report the blocker and propose a narrow adapter/renderer integration before any broad rewrite.

### 1. Lock contracts and behavior

Capture existing request/response DTOs, representative fixture outputs, user-visible workflows, keyboard shortcuts, error states, and packaging behavior. Reuse the Python API unchanged wherever practical. Add Rust request/response types and explicit decoding/validation at the service boundary; do not let untyped JSON leak into view state.

### 2. Build the Rune app shell and a vertical slice

Implement one native Rune executable with the agreed process lifecycle and adapter. Port a complete small workflow end-to-end (open a STEP file, import via the existing service, list/select bodies, compute a section, render it, surface diagnostics, and export). Test against the same backend fixtures and compare exported DXF/manifest semantics with the current app.

### 3. Migrate remaining workflows by feature

Migrate model tree and selection, 3D preview, section settings/history, 2D viewport, diagnostics, project open/save, export, shortcuts, themes, and error/recovery states. Keep each feature behind the same DTO/service boundary; do not duplicate geometry or invent a new backend protocol without a documented need.

### 4. Switch production and retire the old UI

Make Rune the default dev, test, build, install, and release path only after the parity gates pass. Update CI and user/developer documentation in the same change. Remove the old React/Tauri stack after the replacement is the tested default; avoid indefinite dual production paths.

## Acceptance gates

- The existing Python geometry and export suites pass unchanged (or any intentional API change is separately justified and tested).
- The Rune app can perform all documented user workflows against the authenticated local service without network access.
- Geometry parity is demonstrated on fixture STEP models: same selected source components, section parameters, diagnostics, and deterministic export contents/manifests. Any output difference is investigated, not waived because the renderer looks similar.
- Viewport stress tests cover realistic and large models with measured frame time, memory, input responsiveness, and zoom-dependent display tolerance. Rendering never changes exact export results.
- Keyboard, accessibility, native dialogs, error states, app lifecycle, and Windows packaging have automated or documented integration checks.
- Developer setup, CI, packaging, and docs no longer require the retired UI stack once it is removed.
