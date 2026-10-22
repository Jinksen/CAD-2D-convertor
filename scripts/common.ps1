Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-RepositoryRoot {
    return (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
}

function Assert-CommandExists {
    param([Parameter(Mandatory)][string]$Name)

    if ($null -eq (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required executable '$Name' was not found on PATH. See README prerequisites."
    }
}

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory)][string]$Command,
        [Parameter(Mandatory)][string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $Command $($Arguments -join ' ')"
    }
}

function New-SessionToken {
    $bytes = New-Object byte[] 32
    $generator = [Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $generator.GetBytes($bytes)
    }
    finally {
        $generator.Dispose()
    }
    return ([BitConverter]::ToString($bytes)).Replace('-', '')
}

function Get-BackendPythonPath {
    param([Parameter(Mandatory)][string]$BackendPath)

    return (Join-Path $BackendPath '.venv/Scripts/python.exe')
}

function Get-BackendProcessArguments {
    return '-m uvicorn cad2maxwell_backend.api:app --host 127.0.0.1 --port 8000'
}

function Assert-BackendPortAvailable {
    param([Parameter(Mandatory)][int]$Port)

    $client = New-Object System.Net.Sockets.TcpClient
    try {
        try {
            $client.Connect('127.0.0.1', $Port)
        }
        catch [System.Net.Sockets.SocketException] {
            return
        }
        throw "Backend port $Port is already in use. Close the other CAD2Maxwell backend first."
    }
    finally {
        $client.Dispose()
    }
}

function Wait-BackendReady {
    param(
        [Parameter(Mandatory)]$BackendProcess,
        [Parameter(Mandatory)][string]$Token,
        [int]$Attempts = 120
    )

    for ($attempt = 0; $attempt -lt $Attempts; $attempt++) {
        if ($BackendProcess.HasExited) {
            throw "Backend exited during startup with code $($BackendProcess.ExitCode)."
        }
        try {
            $response = Invoke-RestMethod -Uri 'http://127.0.0.1:8000/api/v1/health' `
                -Headers @{ 'X-Session-Token' = $Token } -TimeoutSec 1
            if ($response.status -eq 'ok' -and $response.service -eq 'cad2maxwell-backend' `
                -and $response.api_version -eq 'v1') { return }
        }
        catch {
            # Retry while this launcher-owned process is starting.
        }
        Start-Sleep -Milliseconds 250
    }
    throw 'Backend did not become ready. Check artifacts/logs for the startup error.'
}

function Get-MSVCLinkerDirectory {
    param([Parameter(Mandatory)][string]$InstallationPath)

    $toolsRoot = Join-Path $InstallationPath 'VC/Tools/MSVC'
    if (-not (Test-Path -LiteralPath $toolsRoot -PathType Container)) { return $null }

    foreach ($version in (Get-ChildItem -LiteralPath $toolsRoot -Directory | Sort-Object Name -Descending)) {
        $directory = Join-Path $version.FullName 'bin/Hostx64/x64'
        if (Test-Path -LiteralPath (Join-Path $directory 'link.exe') -PathType Leaf) {
            return $directory
        }
    }
    return $null
}

function Enable-MSVCLinker {
    if ($null -ne (Get-Command 'link.exe' -ErrorAction SilentlyContinue)) { return }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
        $installation = & $vswhere -latest -products '*' `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($installation)) {
            $directory = Get-MSVCLinkerDirectory -InstallationPath $installation
            if ($null -ne $directory) {
                $env:PATH = "$directory;$env:PATH"
                return
            }
        }
    }

    throw "Required MSVC linker 'link.exe' was not found. Install Visual Studio 2022 Build Tools with the Desktop development with C++ workload."
}

function Assert-DeveloperPrerequisites {
    foreach ($command in @('corepack', 'uv', 'cargo', 'rustc')) {
        Assert-CommandExists -Name $command
    }

    Enable-MSVCLinker
}

function Get-LauncherShortcutPaths {
    return @(
        (Join-Path ([Environment]::GetFolderPath('Desktop')) 'CAD2Maxwell.lnk'),
        (Join-Path ([Environment]::GetFolderPath('Programs')) 'CAD2Maxwell.lnk')
    )
}

function New-LauncherShortcut {
    param(
        [Parameter(Mandatory)][string]$ShortcutPath,
        [Parameter(Mandatory)][string]$LauncherPath,
        [string]$IconPath
    )

    if (-not (Test-Path -LiteralPath $LauncherPath -PathType Leaf)) {
        throw "Launcher '$LauncherPath' was not found."
    }
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $LauncherPath
    $shortcut.WorkingDirectory = Split-Path -Parent $LauncherPath
    $shortcut.Description = 'CAD2Maxwell: STEP to 2D DXF section'
    # 7 = minimized: the launcher console stays out of the way while the app runs.
    $shortcut.WindowStyle = 7
    if ($IconPath -and (Test-Path -LiteralPath $IconPath -PathType Leaf)) {
        $shortcut.IconLocation = "$IconPath,0"
    }
    $shortcut.Save()
}
