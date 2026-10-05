$ErrorActionPreference = 'Stop'

Describe 'Developer script helpers' {
    BeforeEach {
        . (Join-Path $PSScriptRoot '..\common.ps1')
    }

    It 'names a missing prerequisite in the error' {
        $message = $null
        try { Assert-CommandExists -Name 'cad2maxwell-definitely-missing-tool' }
        catch { $message = $_.Exception.Message }
        $message | Should Match 'cad2maxwell-definitely-missing-tool'
    }

    It 'resolves the repository root independently of the current directory' {
        $expected = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
        Push-Location $TestDrive
        try {
            Get-RepositoryRoot | Should Be $expected
        }
        finally {
            Pop-Location
        }
    }

    It 'creates a 256-bit hexadecimal session token' {
        New-SessionToken | Should Match '^[0-9A-F]{64}$'
    }

    It 'starts the installed backend Python process directly' {
        $python = Get-BackendPythonPath -BackendPath 'C:\CAD Projects\Motor Tool\backend'
        $arguments = Get-BackendProcessArguments

        $python | Should Be 'C:\CAD Projects\Motor Tool\backend\.venv\Scripts\python.exe'
        $arguments | Should Match '^-m uvicorn cad2maxwell_backend.api:app --host 127\.0\.0\.1 --port 8000$'
    }

    It 'finds the newest installed MSVC linker directory' {
        $installation = Join-Path $TestDrive 'BuildTools'
        foreach ($version in @('14.40.10000', '14.44.35207')) {
            $directory = Join-Path $installation "VC/Tools/MSVC/$version/bin/Hostx64/x64"
            New-Item -ItemType Directory -Path $directory -Force | Out-Null
            New-Item -ItemType File -Path (Join-Path $directory 'link.exe') | Out-Null
        }

        Get-MSVCLinkerDirectory -InstallationPath $installation |
            Should Be (Join-Path $installation 'VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64')
    }

    It 'rejects a port already held by another backend process' {
        $listener = New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Loopback, 0)
        $listener.Start()
        try {
            $port = $listener.LocalEndpoint.Port
            { Assert-BackendPortAvailable -Port $port } | Should Throw 'already in use'
        }
        finally {
            $listener.Stop()
        }
    }

    It 'authenticates readiness and checks the expected service identity' {
        Mock Invoke-RestMethod { return @{ status = 'ok'; service = 'cad2maxwell-backend'; api_version = 'v1' } }
        Wait-BackendReady -BackendProcess ([pscustomobject]@{ HasExited = $false }) -Token 'test-session' -Attempts 1
        Assert-MockCalled Invoke-RestMethod -Times 1 -Exactly -ParameterFilter {
            $Headers['X-Session-Token'] -eq 'test-session'
        }
    }

    It 'does not mistake another service for the geometry backend' {
        Mock Invoke-RestMethod { return @{ status = 'ok'; service = 'another-service'; api_version = 'v1' } }
        Mock Start-Sleep {}
        { Wait-BackendReady -BackendProcess ([pscustomobject]@{ HasExited = $false }) -Token 'test-session' -Attempts 1 } |
            Should Throw 'did not become ready'
    }

    It 'reports an early backend exit instead of opening an unusable window' {
        { Wait-BackendReady -BackendProcess ([pscustomobject]@{ HasExited = $true; ExitCode = 7 }) -Token 'test-session' } |
            Should Throw 'code 7'
    }
}
