$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

& (Join-Path $PSScriptRoot 'test.ps1')
$root = Get-RepositoryRoot
Push-Location (Join-Path $root 'frontend')
try {
    Invoke-NativeCommand -Command (Join-Path $root 'frontend/node_modules/.bin/tauri.CMD') -Arguments @('build')
}
finally {
    Pop-Location
}
