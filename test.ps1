$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot

& "$root\build.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location $root
try {
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    python tests/smoke.py
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

Write-Host 'Tests OK.'
