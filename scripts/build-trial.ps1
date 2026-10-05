$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$destination = Join-Path $root 'artifacts/CAD2Maxwell'
Push-Location (Join-Path $root 'frontend')
try {
    Invoke-NativeCommand -Command './node_modules/.bin/tauri.CMD' -Arguments @('build', '--no-bundle')
}
finally {
    Pop-Location
}
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'frontend/src-tauri/target/release/cad2maxwell-desktop.exe') `
    -Destination (Join-Path $destination 'CAD2Maxwell.exe') -Force
Write-Host 'Trial build ready. Double-click START-APP.cmd in the repository folder.' -ForegroundColor Green
