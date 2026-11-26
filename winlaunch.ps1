param(
    [string] $Address = '127.0.0.1',
    [int] $Port = 8000,
    [switch] $NoBuild
)

$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot

if (-not $NoBuild) {
    & "$root\winbuild.ps1"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Push-Location $root
try {
    Write-Host "Electrow UI at http://$Address`:$Port/ (Ctrl+C to stop)"
    python -m backend --host $Address --port $Port
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
