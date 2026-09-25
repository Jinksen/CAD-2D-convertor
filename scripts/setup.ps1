$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$env:UV_CACHE_DIR = Join-Path $root '.uv-cache'

Push-Location $root
try {
    Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'install', '--frozen-lockfile')
    Invoke-NativeCommand -Command 'uv' -Arguments @('sync', '--project', (Join-Path $root 'backend'))
}
finally {
    Pop-Location
}

Write-Host 'CAD2Maxwell development dependencies are ready.' -ForegroundColor Green
