$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$env:UV_CACHE_DIR = Join-Path $root '.uv-cache'
$testRunDirectory = Join-Path $root ('.test-tmp/quality-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRunDirectory | Out-Null
Push-Location $root
try {
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'lint')
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'typecheck')
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'test')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'ruff', 'check', 'backend')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'mypy', 'backend/src')
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--project', 'backend', 'pytest', 'backend/tests', '-v',
        '--basetemp', (Join-Path $testRunDirectory 'pytest'), '-o', ('cache_dir=' + (Join-Path $testRunDirectory 'pytest-cache')))
    Invoke-NativeCommand -Command 'cargo' -Arguments @('fmt', '--manifest-path', 'frontend/src-tauri/Cargo.toml', '--', '--check')
    Invoke-NativeCommand -Command 'cargo' -Arguments @('clippy', '--manifest-path', 'frontend/src-tauri/Cargo.toml', '--', '-D', 'warnings')
    Invoke-NativeCommand -Command 'cargo' -Arguments @('test', '--manifest-path', 'frontend/src-tauri/Cargo.toml')
    $scriptTests = Invoke-Pester -Script (Join-Path $PSScriptRoot 'tests/Scripts.Tests.ps1') -PassThru
    if ($scriptTests.FailedCount -gt 0) { throw 'PowerShell launcher tests failed.' }
}
finally {
    Pop-Location
}
