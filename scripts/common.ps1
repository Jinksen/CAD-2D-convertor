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

function Assert-DeveloperPrerequisites {
    foreach ($command in @('corepack', 'uv', 'cargo', 'rustc')) {
        Assert-CommandExists -Name $command
    }

    if ($null -eq (Get-Command 'link.exe' -ErrorAction SilentlyContinue)) {
        throw "Required MSVC linker 'link.exe' was not found. Install Visual Studio 2022 Build Tools with the Desktop development with C++ workload, then use a Developer PowerShell."
    }
}
