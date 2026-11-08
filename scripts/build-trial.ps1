$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$destination = Join-Path $root 'artifacts/CAD2Maxwell'
Invoke-NativeCommand -Command 'cargo' -Arguments @('build', '--release', '--locked', '--manifest-path', 'frontend/rune-app/Cargo.toml')
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'frontend/rune-app/target/release/cad2maxwell-desktop.exe') `
    -Destination (Join-Path $destination 'CAD2Maxwell.exe') -Force
Write-Host 'Trial build ready. Double-click START-APP.cmd in the repository folder.' -ForegroundColor Green
