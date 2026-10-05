$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

& (Join-Path $PSScriptRoot 'test.ps1')
& (Join-Path $PSScriptRoot 'build-trial.ps1')
