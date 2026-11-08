$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$backend = Join-Path $root 'backend'
$token = New-SessionToken
$env:CAD2MAXWELL_SESSION_TOKEN = $token

$backendProcess = $null
try {
    $python = Get-BackendPythonPath -BackendPath $backend
    if (-not (Test-Path -LiteralPath $python -PathType Leaf)) {
        throw 'Backend Python environment is missing. Run .\scripts\setup.ps1 first.'
    }
    Assert-BackendPortAvailable -Port 8000
    $backendArguments = Get-BackendProcessArguments
    $backendProcess = Start-Process -FilePath $python -ArgumentList $backendArguments `
        -WorkingDirectory $root -WindowStyle Hidden -PassThru

    Wait-BackendReady -BackendProcess $backendProcess -Token $token

    # The app sees the token and uses this service instead of spawning a bundled one.
    Invoke-NativeCommand -Command 'cargo' -Arguments @('run', '--manifest-path', 'frontend/rune-app/Cargo.toml')
}
finally {
    if ($null -ne $backendProcess -and -not $backendProcess.HasExited) {
        Stop-Process -Id $backendProcess.Id -Force
        $backendProcess.WaitForExit()
    }
    Remove-Item Env:CAD2MAXWELL_SESSION_TOKEN -ErrorAction SilentlyContinue
}
