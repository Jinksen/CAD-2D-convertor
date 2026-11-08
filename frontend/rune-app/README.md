# CAD2Maxwell native desktop app (Rune)

Native Rune replacement for the React/Tauri UI (migration phases 1-2: service client, app state, shell and an import-section-export vertical slice). It is not yet wired into release packaging; the React/Tauri app remains the production UI until the gates in docs/rune-migration.md pass. The app starts the bundled geometry service (or uses `CAD2MAXWELL_SESSION_TOKEN` when a script already started it) and talks to it on 127.0.0.1:8000.

```powershell
cargo run --manifest-path frontend/rune-app/Cargo.toml
cargo test --manifest-path frontend/rune-app/Cargo.toml
cargo run --manifest-path frontend/rune-app/Cargo.toml -- --snap .test-tmp/rune-app.png
```

Rune is pinned to a specific upstream revision in `Cargo.toml`. Read [the migration plan](../../docs/rune-migration.md) before extending this scaffold; the 2D and especially 3D CAD viewport feasibility must be established before choosing a production renderer.
