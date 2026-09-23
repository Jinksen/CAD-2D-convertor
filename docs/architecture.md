# Architecture

CAD2Maxwell uses three deliberately narrow layers:

```text
React/TypeScript UI -> Tauri/Rust host -> loopback FastAPI/Python geometry service
```

React owns rendering, selection, and editable workspace state. It consumes versioned DTOs and must not contain geometry algorithms or depend on OpenCascade types. Rust owns desktop lifecycle, native integration, and future Python process supervision; it stays thin. Python owns exact CAD topology, sectioning, validation, and export.

The Milestone 0 backend exposes `GET /api/v1/health`. It binds to `127.0.0.1`, and browser origins are restricted to the Tauri and local Vite origins. Future process startup will provide a random per-session token.

Heavy geometry work will never run on the UI thread. Exact B-Rep objects remain backend-only; serializable Pydantic DTOs cross the process boundary.
