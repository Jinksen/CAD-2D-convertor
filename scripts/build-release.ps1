# Builds the Windows installer and the portable ZIP into artifacts/release/.
# Both contain the frozen geometry service, so users need no Python or Rust.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$version = Get-AppVersion -RepositoryRoot $root
$work = Join-Path $root 'artifacts/release-build'
$out = Join-Path $root 'artifacts/release'
$backendDist = Join-Path $root 'backend/dist/cad2maxwell-backend'
Remove-Item -LiteralPath $work, $out, $backendDist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $work, $out -Force | Out-Null

# 1. Freeze the geometry service (one folder, no console window).
Push-Location (Join-Path $root 'backend')
try {
    Invoke-NativeCommand -Command 'uv' -Arguments @('run', '--frozen', '--group', 'package', 'pyinstaller',
        'packaging/entry.py', '--name', 'cad2maxwell-backend', '--onedir', '--windowed', '--noconfirm', '--clean',
        '--paths', 'src', '--collect-all', 'OCP', '--collect-submodules', 'uvicorn',
        '--distpath', 'dist', '--workpath', (Join-Path $work 'pyinstaller'), '--specpath', $work)
}
finally {
    Pop-Location
}

# 2. Build the native Rune desktop app.
# Remapping keeps this machine's user and folder names out of the published binaries.
$previousFlags = $env:RUSTFLAGS
$env:RUSTFLAGS = "--remap-path-prefix=$root=. --remap-path-prefix=$env:USERPROFILE=~"
try {
    Invoke-NativeCommand -Command 'cargo' -Arguments @('build', '--release', '--locked',
        '--manifest-path', (Join-Path $root 'frontend/rune-app/Cargo.toml'))
}
finally {
    $env:RUSTFLAGS = $previousFlags
}

# 3. Stage the shared layout (executable next to backend/): the installer and the portable ZIP both use it.
$portable = Join-Path $work 'CAD2Maxwell'
New-Item -ItemType Directory -Path $portable -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'frontend/rune-app/target/release/cad2maxwell-desktop.exe') -Destination (Join-Path $portable 'CAD2Maxwell.exe')
Copy-Item -LiteralPath $backendDist -Destination (Join-Path $portable 'backend') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination $portable
Copy-Item -LiteralPath (Join-Path $root 'docs/demo/demo-motor.step') -Destination $portable

# 4. Per-user installer with Inno Setup (preinstalled on GitHub windows-latest; otherwise install Inno Setup 6).
$iscc = Get-InnoSetupCompiler
Invoke-NativeCommand -Command $iscc -Arguments @("/DAppVersion=$version", "/DStageDir=$portable",
    "/DOutputDir=$out", "/DIconFile=$(Join-Path $root 'frontend/rune-app/assets/icon.ico')",
    "/DLicenseFile=$(Join-Path $root 'LICENSE')", (Join-Path $root 'installer/cad2maxwell.iss'))

# 5. Portable ZIP.
# Windows tar writes standard forward-slash entries; Compress-Archive in PowerShell 5.1 does not.
Invoke-NativeCommand -Command (Join-Path $env:SystemRoot 'System32/tar.exe') -Arguments @(
    '-a', '-c', '-f', (Join-Path $out "CAD2Maxwell_${version}_x64_portable.zip"), '-C', $work, (Split-Path -Leaf $portable))

# Optional, not run: code signing. To sign, call signtool.exe on $portable\CAD2Maxwell.exe and on the installer
# (add SignTool= to the .iss for the installer). Builds are unsigned today; no certificates live in this repo.

# 6. Checksums so downloads can be verified.
$sums = @(Get-ChildItem -LiteralPath $out -File | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object {
    "$((Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant())  $($_.Name)"
})
Set-Content -Encoding ascii -LiteralPath (Join-Path $out 'SHA256SUMS.txt') -Value $sums
Get-ChildItem -LiteralPath $out | Format-Table Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
