$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
Push-Location $root
try {
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'lint')
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'typecheck')
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'test')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'ruff', 'check', 'backend')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'mypy', 'backend/src')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'pytest', 'backend/tests', '-v')
    Invoke-NativeCommand -Command 'cargo' -Arguments @('fmt', '--manifest-path', 'frontend/src-tauri/Cargo.toml', '--', '--check')
    Invoke-NativeCommand -Command 'cargo' -Arguments @('clippy', '--manifest-path', 'frontend/src-tauri/Cargo.toml', '--', '-D', 'warnings')
    Invoke-NativeCommand -Command 'cargo' -Arguments @('test', '--manifest-path', 'frontend/src-tauri/Cargo.toml')
}
finally {
    Pop-Location
}
