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
    $backendArguments = Get-BackendProcessArguments -BackendPath $backend
    $backendProcess = Start-Process -FilePath 'uv' -ArgumentList $backendArguments `
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
        Invoke-NativeCommand -Command 'corepack' -Arguments @('pnpm', 'exec', 'tauri', 'dev')
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
