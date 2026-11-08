# CAD2Maxwell desktop app (Rune)

Native Rust UI built on [Rune](https://github.com/anomalyco/rune), pinned in `Cargo.toml`. It talks to the authenticated local geometry service on 127.0.0.1:8000. At start-up it uses the service whose token is in `CAD2MAXWELL_SESSION_TOKEN` (set by `scripts/dev.ps1` and `scripts/run.ps1`), otherwise it spawns `backend/cad2maxwell-backend.exe` next to its own executable.

```powershell
.\scripts\dev.ps1                                             # backend + cargo run
cargo test --manifest-path frontend/rune-app/Cargo.toml        # headless, no GPU
cargo run --manifest-path frontend/rune-app/Cargo.toml -- --snap .test-tmp/rune-app.png
.\scripts\build-release.ps1                                    # installer + portable ZIP
```

`assets/icon.ico` is the app icon, used by the installer. Gate results and known gaps: [docs/rune-migration.md](../../docs/rune-migration.md).
