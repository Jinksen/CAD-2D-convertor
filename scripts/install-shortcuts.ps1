param([switch]$Remove)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-RepositoryRoot
$launcher = Join-Path $root 'START-APP.cmd'
$icon = Join-Path $root 'artifacts/CAD2Maxwell/CAD2Maxwell.exe'

foreach ($path in Get-LauncherShortcutPaths) {
    if ($Remove) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Remove-Item -LiteralPath $path
            Write-Host "Removed $path"
        }
        continue
    }
    New-LauncherShortcut -ShortcutPath $path -LauncherPath $launcher -IconPath $icon
    Write-Host "Created $path"
}
if (-not $Remove) {
    Write-Host 'Start CAD2Maxwell from the Desktop or the Start menu.' -ForegroundColor Green
}
