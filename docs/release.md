# Release checklist

1. Versions match: `frontend/rune-app/Cargo.toml` (+ `Cargo.lock`), `backend/pyproject.toml`, `backend/uv.lock`, `backend/src/cad2maxwell_backend/{__init__,api}.py`. Update `CHANGELOG.md` and `docs/release-notes/vX.Y.Z.md`.
2. `.\scripts\test.ps1` passes.
3. `.\scripts\build-release.ps1` (needs Inno Setup 6). Output in `artifacts/release/`: installer, portable ZIP, `SHA256SUMS.txt`.
4. Verify checksums: `Get-FileHash <file> -Algorithm SHA256` against `SHA256SUMS.txt`.
5. Smoke test on a clean Windows machine (no Python/Rust): install, open `demo-motor.step`, compute a section, export DXF, check the service log, uninstall. Repeat with the ZIP.
6. Optional: code-sign the exe and installer (see the marked step in `build-release.ps1`).
7. Tag `vX.Y.Z` and create a GitHub release with the three assets and the release notes.
