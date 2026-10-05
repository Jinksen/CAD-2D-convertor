param([switch]$Check)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-RepositoryRoot
$executable = Join-Path $root 'artifacts/CAD2Maxwell/CAD2Maxwell.exe'
$python = Get-BackendPythonPath -BackendPath (Join-Path $root 'backend')
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw 'The local trial build is missing. Run .\scripts\build-trial.ps1 first.'
}
if (-not (Test-Path -LiteralPath $python -PathType Leaf)) {
    throw 'The backend environment is missing. Run .\scripts\setup.ps1 first.'
}

Assert-BackendPortAvailable -Port 8000
$logDirectory = Join-Path $root 'artifacts/logs'
New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
$logPrefix = Join-Path $logDirectory (Get-Date -Format 'yyyyMMdd-HHmmss-fff')
$previousToken = $env:CAD2MAXWELL_SESSION_TOKEN
$backendProcess = $null
$desktopProcess = $null
try {
    $token = New-SessionToken
    $env:CAD2MAXWELL_SESSION_TOKEN = $token
    $backendProcess = Start-Process -FilePath $python -ArgumentList (Get-BackendProcessArguments) `
        -WorkingDirectory $root -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput "$logPrefix-backend.log" -RedirectStandardError "$logPrefix-backend-error.log"
    Wait-BackendReady -BackendProcess $backendProcess -Token $token
    if ($Check) {
        Write-Host 'Trial executable exists and authenticated geometry service is ready.' -ForegroundColor Green
        return
    }
    Write-Host 'CAD2Maxwell is starting. Close the desktop window to stop its local geometry service.'
    $desktopProcess = Start-Process -FilePath $executable -WorkingDirectory $root -PassThru
    while (-not $desktopProcess.HasExited) {
        if ($backendProcess.HasExited) {
            throw "The geometry service stopped. See $logPrefix-backend-error.log and restart the app."
        }
        Start-Sleep -Milliseconds 500
    }
    if ($desktopProcess.ExitCode -ne 0) {
        throw "The desktop exited with code $($desktopProcess.ExitCode)."
    }
}
catch {
    Write-Host "Startup logs: $logDirectory" -ForegroundColor Yellow
    throw
}
finally {
    if ($null -ne $desktopProcess -and -not $desktopProcess.HasExited) {
        Stop-Process -Id $desktopProcess.Id -Force
        $desktopProcess.WaitForExit()
    }
    if ($null -ne $backendProcess -and -not $backendProcess.HasExited) {
        Stop-Process -Id $backendProcess.Id -Force
        $backendProcess.WaitForExit()
    }
    $env:CAD2MAXWELL_SESSION_TOKEN = $previousToken
}
