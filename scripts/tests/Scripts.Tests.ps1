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
}
