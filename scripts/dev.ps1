$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

Assert-DeveloperPrerequisites
$root = Get-RepositoryRoot
$backend = Join-Path $root 'backend'
$frontend = Join-Path $root 'frontend'
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

    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($backendProcess.HasExited) {
            throw "Backend exited during startup with code $($backendProcess.ExitCode)."
        }
        try {
            $response = Invoke-RestMethod -Uri 'http://127.0.0.1:8000/api/v1/health' -TimeoutSec 1
            if ($response.status -eq 'ok') { $ready = $true; break }
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if (-not $ready) { throw 'Backend did not become ready within 10 seconds.' }

    Push-Location $frontend
    try {
        Invoke-NativeCommand -Command (Join-Path $frontend 'node_modules/.bin/tauri.CMD') -Arguments @('dev')
    }
    finally {
        Pop-Location
    }
}
finally {
    if ($null -ne $backendProcess -and -not $backendProcess.HasExited) {
        Stop-Process -Id $backendProcess.Id -Force
        $backendProcess.WaitForExit()
    }
    Remove-Item Env:CAD2MAXWELL_SESSION_TOKEN -ErrorAction SilentlyContinue
}
