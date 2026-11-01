# UX and performance pass — spec

Date: 2026-11-01 · Branch: `feature/ux-performance`

## Goal

Make CAD2Maxwell feel like a responsive engineering tool: no idle GPU/CPU burn, no frozen backend
during import, smooth 3D preview of real assemblies, CAD-grade 2D navigation, keyboard workflow and
honest progress feedback.

## Non-goals

- No change to exact sectioning, classification, validation or DXF export geometry.
- No new runtime dependencies (three.js, zustand, FastAPI, OCP already cover everything here).
- No server-side cancellation of OpenCascade operations (client-side abort only).

## Constraints (from CLAUDE.md / AGENTS.md)

- The 3D preview mesh is display-only; it must never feed sectioning or export.
- OpenCascade objects stay in the backend; the frontend receives typed DTOs only.
- Every behaviour change gets a test; geometry-adjacent changes get a regression test.
- Quality gate: `scripts/test.ps1` (eslint, tsc, vitest, ruff, mypy, pytest, cargo).

## Tickets

Work packages (WP) own disjoint files so they can run in parallel.

| ID | Ticket | WP | Issue |
|----|--------|----|-------|
| T1 | 3D render loop runs continuously, also when hidden | A | #12 |
| T2 | STEP upload blocks the backend event loop | A | #13 |
| T3 | Preview mesh: shared nodes, smooth normals, compact transfer, cache | A | #14 |
| T4 | Preview triangle limit fails instead of coarsening | A | #15 |
| T5 | 3D feature edges, standard views and fit | A | #16 |
| T6 | 2D viewport: pan, zoom at cursor, non-passive wheel, viewport keys | B | #17 |
| T7 | Avoid whole-store re-renders and O(n·m) lookups | A/B/C | #18 |
| T8 | Debounced automatic section recompute | C | #19 |
| T9 | Global keyboard shortcuts | C | #20 |
| T10 | Progress feedback and client-side cancel for long operations | C | #21 |
| T11 | Lazy-load the 3D preview chunk | C | #22 |
| T12 | Pin frontend dependency versions | D | #23 |
| T13 | pytest works without `--basetemp` | D | #24 |

### T1 — 3D render loop runs continuously
`MeshCanvas.tsx` calls `requestAnimationFrame` every frame forever, and the 3D panel stays mounted
(`hidden`) when the 2D tab is active.
Acceptance: render only on demand (controls `change`, damping settle, resize, selection/plane change);
no frames are rendered while the panel is hidden or the canvas is idle. Test: unit test of the
render-scheduling helper.

### T2 — STEP upload blocks the event loop
`api.py` `import_step_file` is `async` and calls synchronous `service.import_path`.
Acceptance: the import runs in a worker thread (`starlette.concurrency.run_in_threadpool`);
`/api/v1/health` answers while an import is running. Test: API test that health responds during a
slow (stubbed) import.

### T3 — Preview mesh quality and transfer
`preview_mesh.py` duplicates three vertices per triangle with flat normals (faceted curved surfaces)
in a pure-Python loop, sent as nested JSON float arrays, regenerated on every request.
Acceptance: reuse OCC triangulation nodes per face, smooth per-node normals (OCC normals when
available), transfer positions/normals/indices as base64 little-endian `Float32`/`Uint32` buffers
(frontend contract and type guard updated together), preview cached per `import_id` in the session.
Test: box preview still matches bounds and component ids; cylinder preview has non-flat normals;
payload decoding test on the frontend.

### T4 — Preview triangle limit
50 000 triangles is a hard error ("Try a smaller STEP file").
Acceptance: when the budget is exceeded, retry with a coarser deflection (bounded number of steps);
only fail if the coarsest step still exceeds the budget. The response says when a coarsened preview
is shown and the UI displays it as a note. Test: small budget forces coarsening, not failure.

### T5 — 3D feature edges, standard views, fit
Acceptance: B-Rep edges drawn as dark lines over the shaded bodies (backend sends edge polylines
from the display triangulation, display-only); toolbar with Fit, Iso, Top, Front, Right; selection
highlight remains. Test: backend returns edges for a box (12 edges); view helper unit tests.

### T6 — 2D viewport navigation
Acceptance: wheel zooms around the cursor (`zoomView` anchor), the wheel listener is non-passive so
the page never scrolls, drag with middle or left button on empty space pans, the SVG is focusable
and supports F (fit), +/− (zoom), arrow keys (pan). Test: `viewCamera` unit tests for anchor zoom and
pan; component test for keys.

### T7 — Re-renders and lookups
`useWorkspaceStore()` without selector is used in `App.tsx`, `StepImportWorkspace.tsx`,
`PreviewView.tsx`, `SectionViewport.tsx`; `SectionViewport` calls `components.find` per path.
Acceptance: each WP converts the files it owns to per-field selectors (`useShallow` where several
fields are read); `SectionViewport` uses one `Map` by component id.

### T8 — Automatic section recompute
Changing the offset or normal clears the result and requires Compute again.
Acceptance: once a section has been computed for the current model/plane, edits recompute it after
a 300 ms debounce; stale responses are discarded; Enter in the offset field computes immediately.
Test: hook/component test with fake timers.

### T9 — Global keyboard shortcuts
Acceptance: Ctrl+O open STEP, Ctrl+Enter compute, Ctrl+E export (when exportable), 1 / 2 switch 2D /
3D, Esc clears selection. Ignored while typing in inputs. Shortcuts shown in button tooltips.
Test: component test.

### T10 — Long-running operation feedback
Acceptance: import and section show elapsed time (e.g. "Importing… 12 s") and a Cancel button that
aborts the fetch (`AbortController`) and restores the idle state with a clear message.
Test: client test that abort rejects with a cancel error; UI test for the Cancel button.

### T11 — Lazy-load 3D preview
`dist/assets/index-*.js` is 797 kB, mostly three.js.
Acceptance: `PreviewView` is loaded with `React.lazy` + `Suspense`; the initial chunk no longer
contains three.js.

### T12 — Pin frontend dependencies
`frontend/package.json` uses `"latest"`; `vite` and `@vitejs/plugin-react` are runtime dependencies.
Acceptance: caret ranges matching the versions in `pnpm-lock.yaml`; build tools moved to
`devDependencies`; lockfile unchanged apart from the specifier lines.

### T13 — pytest temp directory
Plain `pytest` fails with `PermissionError` in the system temp dir on Windows.
Acceptance: `pyproject.toml` sets a default `--basetemp` under the git-ignored `.test-tmp/`;
`scripts/test.ps1` keeps working (its explicit `--basetemp` overrides).

## File ownership

| WP | Owns |
|----|------|
| A | `backend/**/preview*`, `backend/**/api.py`, `frontend/src/features/preview/**` |
| B | `frontend/src/features/section/SectionViewport.tsx`, `viewCamera.ts`, `SectionView.tsx` |
| C | `StepImportWorkspace.tsx`, `App.tsx`, `Ribbon.tsx`, `useSectionRun.ts`, `SectionSettings.tsx`, feature `client.ts` files except preview |
| D | `frontend/package.json`, `backend/pyproject.toml` |

`App.css` is shared; append new rules in a block commented with the ticket ID.
