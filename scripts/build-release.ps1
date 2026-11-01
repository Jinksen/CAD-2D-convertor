# Builds the Windows installer and the portable ZIP into artifacts/release/.
# Both contain the frozen geometry service, so users need no Python, Node or Rust.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$version = (Get-Content -Raw (Join-Path $root 'frontend/src-tauri/tauri.conf.json') | ConvertFrom-Json).version
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

# 2. Build the desktop shell and the NSIS installer with the frozen service as a resource.
# Remapping keeps this machine's user and folder names out of the published binaries.
$previousFlags = $env:RUSTFLAGS
$env:RUSTFLAGS = "--remap-path-prefix=$root=. --remap-path-prefix=$env:USERPROFILE=~"
Push-Location (Join-Path $root 'frontend')
try {
    Invoke-NativeCommand -Command './node_modules/.bin/tauri.CMD' -Arguments @(
        'build', '--config', 'src-tauri/tauri.release.conf.json')
}
finally {
    Pop-Location
    $env:RUSTFLAGS = $previousFlags
}
$release = Join-Path $root 'frontend/src-tauri/target/release'
$installer = Get-ChildItem -LiteralPath (Join-Path $release 'bundle/nsis') -Filter "*_${version}_x64-setup.exe" |
    Select-Object -First 1
if ($null -eq $installer) { throw 'The NSIS installer was not produced.' }
Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $out "CAD2Maxwell_${version}_x64-setup.exe")

# 3. Portable ZIP: the same layout the installer creates (executable next to backend/).
$portable = Join-Path $work 'CAD2Maxwell'
New-Item -ItemType Directory -Path $portable -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $release 'cad2maxwell-desktop.exe') -Destination (Join-Path $portable 'CAD2Maxwell.exe')
Copy-Item -LiteralPath $backendDist -Destination (Join-Path $portable 'backend') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination $portable
Copy-Item -LiteralPath (Join-Path $root 'docs/demo/demo-motor.step') -Destination $portable
# Windows tar writes standard forward-slash entries; Compress-Archive in PowerShell 5.1 does not.
Invoke-NativeCommand -Command (Join-Path $env:SystemRoot 'System32/tar.exe') -Arguments @(
    '-a', '-c', '-f', (Join-Path $out "CAD2Maxwell_${version}_x64_portable.zip"), '-C', $work, (Split-Path -Leaf $portable))

# 4. Checksums so downloads can be verified.
$sums = @(Get-ChildItem -LiteralPath $out -File | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object {
    "$((Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant())  $($_.Name)"
})
Set-Content -Encoding ascii -LiteralPath (Join-Path $out 'SHA256SUMS.txt') -Value $sums
Get-ChildItem -LiteralPath $out | Format-Table Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
