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
}
