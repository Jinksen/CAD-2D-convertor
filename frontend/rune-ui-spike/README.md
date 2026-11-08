# Rune UI evaluation spike

This is a standalone, non-production Rune app used to test the native UI framework before migrating CAD2Maxwell. It is intentionally not wired into the current React/Tauri build.

```powershell
cargo run --manifest-path frontend/rune-ui-spike/Cargo.toml
cargo test --manifest-path frontend/rune-ui-spike/Cargo.toml
cargo run --manifest-path frontend/rune-ui-spike/Cargo.toml -- --snap .test-tmp/rune-ui-spike.png
```

Rune is pinned to a specific upstream revision in `Cargo.toml`. Read [the migration plan](../../docs/rune-migration.md) before extending this scaffold; the 2D and especially 3D CAD viewport feasibility must be established before choosing a production renderer.
