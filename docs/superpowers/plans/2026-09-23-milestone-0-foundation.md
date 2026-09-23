# CAD2Maxwell Milestone 0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a runnable Windows monorepo whose Tauri/React desktop shell reports the status of a local typed FastAPI backend.

**Architecture:** A pnpm workspace owns the React/Vite frontend and thin Tauri host, while a uv-managed Python package owns the local API. The UI consumes a versioned HTTP DTO through a small client boundary; PowerShell scripts orchestrate setup, development, tests, and packaging without introducing geometry code.

**Tech Stack:** Tauri 2, React 19, TypeScript 5 strict mode, Vite, Zustand, Vitest/Testing Library, Rust stable, Python 3.12, FastAPI, Pydantic 2, uv, Ruff, mypy, pytest.

**Spec:** `CAD2Maxwell_PROJECT_SPEC.md` (Milestone 0 and section 76)

## Global Constraints

- Windows is the MVP platform; processing remains local and the API binds only to `127.0.0.1`.
- Python 3.12+ is the geometry authority; Rust remains a thin desktop/process bridge; React contains no geometry logic.
- Cross-process data uses typed, serializable DTOs and versioned `/api/v1` endpoints.
- TypeScript uses strict mode and no undocumented `any`; Python uses Pydantic, Ruff, mypy, and pytest.
- Do not add STEP import, OpenCascade, sectioning, validation, or DXF export in this milestone.
- Prefer focused source files below 300 lines and do not create generic dumping-ground modules.

## Review Focus

- Backend unavailable at UI startup: the shell must render a clear offline state instead of throwing or remaining in a loading state.
- Backend returns malformed JSON or a non-2xx response: the client must map it to a typed unavailable result.
- Browser/Tauri environment lacks the configured API URL: the client must use the documented localhost default.
- A developer lacks uv, pnpm, Cargo, or Python 3.12: setup/dev scripts must stop with an actionable prerequisite message.
- Tests run from a path containing spaces: scripts must use path-safe PowerShell argument handling.

---

### Task 1: Python Backend Package and Health Contract

**Files:**
- Create: `backend/pyproject.toml`
- Create: `backend/src/cad2maxwell_backend/__init__.py`
- Create: `backend/src/cad2maxwell_backend/config.py`
- Create: `backend/src/cad2maxwell_backend/models/health.py`
- Create: `backend/src/cad2maxwell_backend/api.py`
- Create: `backend/tests/test_health.py`

**Interfaces:**
- Consumes: environment variable `CAD2MAXWELL_SESSION_TOKEN` and loopback host configuration.
- Produces: `GET /api/v1/health -> HealthResponse(status: Literal["ok"], service: str, api_version: str)` and `create_app() -> FastAPI`.

- [ ] **Step 1: Write failing API tests**

```python
from fastapi.testclient import TestClient
from cad2maxwell_backend.api import create_app

def test_health_returns_versioned_typed_payload() -> None:
    response = TestClient(create_app()).get("/api/v1/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok", "service": "cad2maxwell-backend", "api_version": "v1"}

def test_external_host_is_not_configured() -> None:
    from cad2maxwell_backend.config import Settings
    assert Settings().host == "127.0.0.1"
```

- [ ] **Step 2: Run `uv run --project backend pytest backend/tests -v` and verify collection fails because the package does not exist.**
- [ ] **Step 3: Add the Pydantic response model, loopback-only settings, application factory, CORS restricted to Tauri/local development origins, and `/api/v1/health`.**
- [ ] **Step 4: Run the focused pytest command and verify both tests pass.**
- [ ] **Step 5: Run `uv run --project backend ruff check backend` and `uv run --project backend mypy backend/src` with zero errors.**

### Task 2: React Engineering Workspace and Backend Status

**Files:**
- Create: `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`
- Create: `frontend/package.json`, `frontend/tsconfig.json`, `frontend/vite.config.ts`, `frontend/eslint.config.js`, `frontend/index.html`
- Create: `frontend/src/main.tsx`, `frontend/src/app/App.tsx`, `frontend/src/app/App.css`
- Create: `frontend/src/features/backend-status/contracts.ts`
- Create: `frontend/src/features/backend-status/client.ts`
- Create: `frontend/src/features/backend-status/BackendStatus.tsx`
- Create: `frontend/src/test/setup.ts`, `frontend/src/app/App.test.tsx`, `frontend/src/features/backend-status/client.test.ts`

**Interfaces:**
- Consumes: Task 1 `HealthResponse` JSON at `/api/v1/health`.
- Produces: `fetchBackendHealth(fetcher?: typeof fetch): Promise<BackendHealth>` where `BackendHealth` is a discriminated `online | offline` union; an engineering shell with toolbar, model tree, viewport, properties panel, and status bar.

- [ ] **Step 1: Scaffold configuration only, install pinned dependencies with pnpm, then write tests asserting all five workspace regions render and online/offline health states are accessible by role/text.**
- [ ] **Step 2: Write client tests using a local fake `fetch` function for valid success, rejected request, malformed body, and non-2xx response.**
- [ ] **Step 3: Run `pnpm --filter @cad2maxwell/frontend test -- --run` and verify failures identify the missing UI/client modules.**
- [ ] **Step 4: Implement strict DTO parsing, the status component, Zustand workspace state, and the compact responsive shell; use semantic landmarks and visible keyboard focus.**
- [ ] **Step 5: Re-run focused tests and verify they pass, then run `pnpm lint`, `pnpm typecheck`, and `pnpm test` successfully.**

### Task 3: Thin Tauri Desktop Host

**Files:**
- Create: `frontend/src-tauri/Cargo.toml`, `frontend/src-tauri/build.rs`, `frontend/src-tauri/tauri.conf.json`
- Create: `frontend/src-tauri/capabilities/default.json`
- Create: `frontend/src-tauri/src/lib.rs`, `frontend/src-tauri/src/main.rs`
- Create: `frontend/src-tauri/icons/*` using Tauri-generated application icons.

**Interfaces:**
- Consumes: Task 2 Vite development server and production assets.
- Produces: a Windows desktop host with least-privilege capability configuration and no geometry/business logic.

- [ ] **Step 1: Add a Rust unit test asserting the application identity constant equals `com.cad2maxwell.desktop`, then run `cargo test --manifest-path frontend/src-tauri/Cargo.toml` and verify it fails before the host module exists.**
- [ ] **Step 2: Add the minimal Tauri 2 host, build script, config, capabilities, and generated icons.**
- [ ] **Step 3: Run `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings` for the Tauri manifest and verify all pass.**

### Task 4: Path-Safe PowerShell Developer Workflows

**Files:**
- Create: `scripts/common.ps1`, `scripts/setup.ps1`, `scripts/dev.ps1`, `scripts/test.ps1`, `scripts/package.ps1`
- Create: `scripts/tests/Scripts.Tests.ps1`

**Interfaces:**
- Consumes: backend uv project, frontend pnpm scripts, and Tauri Cargo manifest from Tasks 1-3.
- Produces: `setup.ps1` prerequisite/bootstrap flow; `dev.ps1` supervised backend plus Tauri development flow; `test.ps1` full quality gate; `package.ps1` verified Tauri build.

- [ ] **Step 1: Write Pester tests that dot-source `common.ps1` and verify prerequisite errors name the missing executable and repository paths resolve correctly when the project path contains spaces.**
- [ ] **Step 2: Run `Invoke-Pester scripts/tests/Scripts.Tests.ps1` and verify failure because the helpers are missing.**
- [ ] **Step 3: Implement native argument arrays, `$PSScriptRoot`-relative paths, prerequisite checks, child-process cleanup in `finally`, backend readiness polling, and nonzero exit propagation.**
- [ ] **Step 4: Re-run Pester and verify it passes; invoke `scripts/test.ps1` and verify it executes frontend, backend, and Rust checks.**

### Task 5: CI and Developer Documentation

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.gitignore`, `.editorconfig`
- Modify: `README.md`
- Create: `docs/architecture.md`, `docs/geometry.md`, `docs/dxf-export.md`, `docs/ux.md`

**Interfaces:**
- Consumes: all Task 1-4 commands.
- Produces: repeatable Windows CI and documented setup/run/test/package workflow.

- [ ] **Step 1: Add a documentation smoke test to `backend/tests/test_repository_docs.py` asserting the README contains prerequisites, setup, development, test, package, status, supported files, limitations, and roadmap headings; run it and verify failure against the current README.**
- [ ] **Step 2: Rewrite the README and focused docs to explain Milestone 0, architecture boundaries, exact-B-Rep future direction, UX shell, commands, and current limitations without claiming geometry support.**
- [ ] **Step 3: Add Windows CI that installs Python 3.12, uv, current Node LTS, pnpm, and Rust stable, then runs `scripts/test.ps1`.**
- [ ] **Step 4: Run the documentation test and the complete `scripts/test.ps1` quality gate; verify all checks pass with clean output.**
- [ ] **Step 5: Run a manual smoke test: start `scripts/dev.ps1`, confirm `/api/v1/health` returns the contract, confirm the desktop shell reports Online, then stop it and confirm child processes exit.**

## Completion Criteria

- `pnpm install` succeeds from the repository root.
- `./scripts/dev.ps1` starts the loopback backend and Tauri desktop app in one workflow.
- The application visibly contains the toolbar, model tree placeholder, viewport placeholder, properties placeholder, status bar, and backend state.
- `./scripts/test.ps1` passes frontend lint/typecheck/tests, backend lint/typecheck/tests, and Rust fmt/clippy/tests.
- No geometry dependency or behavior has been introduced.
