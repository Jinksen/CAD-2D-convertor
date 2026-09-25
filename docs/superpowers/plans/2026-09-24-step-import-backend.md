# STEP Import Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import local STEP/STP files into process-owned exact B-Rep sessions and return deterministic, typed component summaries.

**Architecture:** FastAPI delegates path validation and response mapping to an import service. An OCP/XCAF adapter owns document traversal and shape extraction; only serializable Pydantic DTOs leave the backend. A process-local store retains located millimetre shapes for later geometry features.

**Tech Stack:** Python 3.12+, FastAPI, Pydantic, OCP/OpenCascade, uv, pytest, Ruff, mypy.

**Spec:** `docs/superpowers/specs/2026-09-23-step-import-backend-design.md`

## Global Constraints

- The import source is an absolute local `.step` or `.stp` path; multipart content and CAD-embedded code are out of scope.
- Exact B-Rep shapes remain in Python; mesh slicing and silent geometry repair are forbidden.
- Source-provided name, color, hierarchy, and unit metadata must be distinguished from generated values.
- Session shapes and reported bounds use millimetres; unknown or contradictory units require a diagnostic.
- Stable component IDs depend on content hash and source occurrence/solid identity, never file location.
- Export names contain only `[A-Za-z0-9_-]` and use deterministic duplicate suffixes.
- Expected input errors map to 422, 404, 403, or 400; unexpected failures return 500 without tracebacks.
- This slice excludes preview meshes, frontend changes, and sectioning.

## Review Focus

1. Repeated occurrences of one product shape: assert distinct located bodies and IDs, with the occurrence hierarchy retained (Task 3).
2. STEP inches or metres: assert millimetre bounds and recorded source scale; never silently label unknown units as mm (Task 3).
3. Duplicate names with diacritics: assert stable ASCII names and collision suffixes across repeat imports (Task 1 and Task 4).
4. Invalid, missing, or unreadable paths: assert exact status mapping and no importer invocation (Task 4).
5. A body with void or non-finite bounds: assert a diagnostic and omission of that body, with no non-finite JSON (Task 3).

---

### Task 1: Contracts and deterministic identity

**Files:**
- Create: `backend/src/cad2maxwell_backend/models/imports.py`
- Create: `backend/src/cad2maxwell_backend/naming.py`
- Create: `backend/tests/test_import_models.py`
- Create: `backend/tests/test_naming.py`

**Interfaces:**
- Produces `BoundingBox(min_xyz: tuple[float, float, float], max_xyz: tuple[float, float, float])` with finite, ordered coordinates.
- Produces `ImportRequest(path: str)`, `ImportDiagnostic(code, severity, message, component_id=None)`, `ImportComponent`, and `ImportResponse` Pydantic DTOs. Use `Literal["info", "warning", "error"]` for severity and `tuple[int, int, int] | None` for RGB.
- Produces `stable_component_id(source_sha256: str, occurrence_key: str, solid_ordinal: int) -> str` and `assign_export_names(display_names: list[str]) -> list[str]`.

- [ ] **Step 1: Write failing contract tests.** Cover reversed/non-finite box coordinates, RGB outside 0–255, and response JSON containing only DTO fields. Include this case:

  ```python
  def test_bounds_reject_nonfinite() -> None:
      with pytest.raises(ValidationError):
          BoundingBox(min_xyz=(0, 0, float("nan")), max_xyz=(1, 1, 1))
  ```

- [ ] **Step 2: Write failing naming tests.** Assert `assign_export_names(["Cívka", "Cívka", "***", "Cívka"]) == ["Civka", "Civka_02", "Component", "Civka_03"]`. Assert equal ID inputs yield equal IDs and a changed ordinal or occurrence key changes the ID. Add a collision case where an original name already ends in `_02`.
- [ ] **Step 3: Run `uv run --project backend pytest backend/tests/test_import_models.py backend/tests/test_naming.py -q`; confirm failures.**
- [ ] **Step 4: Implement DTO validators and naming.** Normalize Unicode with NFKD, strip combining marks, replace disallowed runs with `_`, trim separators, default empty results to `Component`, and reserve every final name before suffixing. Hash a length-prefixed UTF-8 tuple of SHA, occurrence key, and ordinal with SHA-256 so separator characters cannot create ID collisions. Set Pydantic `allow_inf_nan=False` and validate box ordering.
- [ ] **Step 5: Rerun the focused tests, Ruff, and mypy; commit only these files.**

### Task 2: Internal records and exact-shape session ownership

**Files:**
- Create: `backend/src/cad2maxwell_backend/domain/import_result.py`
- Create: `backend/src/cad2maxwell_backend/services/import_sessions.py`
- Create: `backend/tests/test_import_sessions.py`

**Interfaces:**
- `ImportedBody` carries an OCP `TopoDS_Shape`, `occurrence_key`, `solid_ordinal`, optional source metadata, millimetre bounds, and metadata provenance. `ImportedModel` carries ordered bodies, source unit/scale, aggregate bounds, and diagnostics.
- `ImportSessions.put(source_sha256: str, bodies: tuple[ImportedBody, ...]) -> str`, `get(import_id: str) -> tuple[ImportedBody, ...] | None`, and `remove(import_id: str) -> bool`. Import IDs are random process-local tokens; component IDs remain deterministic.

- [ ] **Step 1: Write failing session tests.** Use a minimal test shape from `BRepPrimAPI_MakeBox` and assert put/get preserves object identity, unknown lookup returns `None`, remove invalidates, and two put calls receive different IDs.
- [ ] **Step 2: Run `uv run --project backend pytest backend/tests/test_import_sessions.py -q`; confirm failures.**
- [ ] **Step 3: Implement focused dataclasses and a lock-protected dictionary.** Keep OCP imports in the domain/geometry boundary, never in Pydantic models. Generate IDs with `secrets.token_urlsafe(24)`; store immutable tuples. Do not add persistence or automatic eviction in this slice.
- [ ] **Step 4: Rerun focused tests, Ruff, and mypy; commit only these files.**

### Task 3: XCAF STEP extraction and bounds

**Files:**
- Modify: `backend/pyproject.toml`
- Modify: `backend/uv.lock`
- Create: `backend/src/cad2maxwell_backend/geometry/bounds.py`
- Create: `backend/src/cad2maxwell_backend/geometry/step_importer.py`
- Create: `backend/tests/step_fixtures.py`
- Create: `backend/tests/test_step_importer.py`

**Interfaces:**
- `read_step(path: Path) -> ImportedModel` returns only located, exact solids with finite bounds in millimetres. It raises `StepImportError` for rejected content or no usable solids. The service does not need to know XCAF types.
- `shape_bounds_mm(shape: TopoDS_Shape) -> BoundingBox | None` rejects void and non-finite boxes.

- [ ] **Step 1: Add OCP with a bounded compatible version range and update the uv lock.** Run `uv add --project backend 'cadquery-ocp>=7.9.3.1.1,<7.10'`. PyPI lists a CPython 3.12 Windows wheel for 7.9.3.1.1; record the resolved version in the lockfile. If the package/API differs, keep the same public interfaces and document the binding choice.
- [ ] **Step 2: Probe the installed binding with a small read-only introspection command.** Confirm available `STEPCAFControl_Reader`, `XCAFDoc_DocumentTool`, shape/color tools, STEP unit APIs, `BRepBndLib`, and location methods before coding against them. Inspect `.pyi` signatures where available; do not guess method overloads.
- [ ] **Step 3: Write failing fixture-based tests.** Generate small STEP files during pytest using the same OCP binding. Cases: a named colored box, two product occurrences with distinct translations, duplicate/non-ASCII names, an unnamed solid, inch or metre source units, malformed text, and a skipped non-solid. Assert exact body count, occurrence paths, translated millimetre boxes, color/name provenance, unit scale, stable ordering, and explicit diagnostics for unavailable metadata. A fixture's expected dimensions must come from how it was created, not the user's STEP file.
- [ ] **Step 4: Run `uv run --project backend pytest backend/tests/test_step_importer.py -q`; confirm failures.**
- [ ] **Step 5: Implement XCAF transfer and recursive traversal.** Enable name/color/layer/material modes; inspect free shapes, assemblies, components, and referred products. Build occurrence keys from stable XCAF label entries plus sibling/solid ordinals; apply occurrence transforms to exact solids before retaining them. Prefer occurrence metadata over product metadata. Report reader status and skipped unsupported topology as diagnostics; do not heal shapes.
- [ ] **Step 6: Resolve units before accepting bounds.** Configure the XCAF document's target length unit to millimetres and verify on a non-mm fixture that resulting exact shapes and bounds are millimetres. Record the file's declared source unit and scale. When units cannot be determined reliably, emit an explicit diagnostic and avoid claiming a measured source scale; do not silently treat the file as millimetres.
- [ ] **Step 7: Implement bounds through OpenCascade's bounding facilities.** Reject void/non-finite boxes per body, calculate aggregate bounds from accepted bodies, and fail with `StepImportError` if none remain. Add a focused test for the invalid-box path using a controlled stub if no STEP fixture can produce it.
- [ ] **Step 8: Rerun importer tests, Ruff, and mypy; commit importer, fixtures, dependency, and lockfile.**

### Task 4: Import service and versioned HTTP route

**Files:**
- Create: `backend/src/cad2maxwell_backend/services/import_service.py`
- Modify: `backend/src/cad2maxwell_backend/api.py`
- Create: `backend/tests/test_import_service.py`
- Create: `backend/tests/test_import_api.py`

**Interfaces:**
- `ImportService(importer: Callable[[Path], ImportedModel], sessions: ImportSessions).import_path(raw_path: str) -> ImportResponse`.
- `POST /api/v1/imports/step` accepts `ImportRequest` and returns `ImportResponse`.
- Typed service exceptions distinguish invalid path, absent file, access denial, rejected STEP, and unexpected importer failure.

- [ ] **Step 1: Write failing service tests with an injected importer.** Use temporary files and assert path validation occurs before import, SHA-256 is content-based, repeated imports have stable component IDs and export names, response order matches source order, and session lookup returns exact shapes. Cover Windows absolute paths using `PureWindowsPath` where the test host cannot create them.
- [ ] **Step 2: Write failing API tests.** Assert 200 DTO shape, 422 for relative/suffix/directory paths, 404 missing file, 403 unreadable file (mock the read/open boundary so the test works under elevated users), 400 malformed/no solids, and 500 with a generic response for an injected unexpected error. Verify no traceback or CAD content appears in errors. Ensure CORS permits `POST` only from allowed origins and existing health tests still pass.
- [ ] **Step 3: Run `uv run --project backend pytest backend/tests/test_import_service.py backend/tests/test_import_api.py -q`; confirm failures.**
- [ ] **Step 4: Implement validation, hashing, mapping, and route wiring.** Resolve paths strictly; differentiate missing files from non-files; stream SHA-256 reads; reject unsupported suffixes case-insensitively. Map imported metadata without inventing source values; reserve deterministic export names. Create the session only after a successful import. Instantiate one service/store per FastAPI app so `import_id` remains addressable while the app lives. Keep request/response types in `models/imports.py`; never serialize OCP objects.
- [ ] **Step 5: Add explicit error handlers and tests for response stability.** Return a stable code plus a concise message for expected errors. Log unexpected exceptions server-side and return a generic 500 body. Preserve health route behavior.
- [ ] **Step 6: Rerun focused tests, Ruff, mypy, and all backend tests; commit service and API files.**

### Task 5: Documentation and complete quality gate

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/geometry.md`
- Modify: `backend/tests/test_repository_docs.py` only if its coverage must change to reflect implemented import behavior.

- [ ] **Step 1: Update docs.** Describe the endpoint with an example request/response, path and error behavior, process-local import IDs, source metadata provenance, millimetre conversion, and missing preview/UI work. Explain why XCAF occurrence traversal and exact located shapes are retained. Keep the README status and roadmap accurate.
- [ ] **Step 2: Run focused backend checks:** `uv run --project backend ruff check backend`; `uv run --project backend mypy backend/src`; `uv run --project backend pytest backend/tests -v`. Fix only failures caused by this slice.
- [ ] **Step 3: Run the repository gate:** `./scripts/test.ps1`. If an unavailable local tool blocks it, record the exact command/output and run every available constituent check.
- [ ] **Step 4: Optionally import the local engineering STEP sample as a manual smoke test if it is readable; report observed counts without hard-coding them. Do not add the sample to git.**
- [ ] **Step 5: Review `git diff` for accidental IDE/sample/verification files, commit only task files, and summarize implemented behavior, test evidence, and limits.**

## Self-Review

- The five review-focus cases are assigned to concrete tests above.
- The route, DTO, session, importer, error, and documentation work in the written design are covered. Mesh and frontend work remain explicitly outside this slice.
- Before implementation, resolve OCP method names by inspecting the installed binding; the plan fixes the public interfaces and behavior while allowing binding-specific calls to match actual signatures.
