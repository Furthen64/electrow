$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot

& "$root\checkreqs.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location $root
try {
    cargo build --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    python -m compileall -q backend tests
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    if (Get-Command node -ErrorAction SilentlyContinue) {
        node --check web/app.js
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    else {
        Write-Host 'node not found; skipping JavaScript syntax check'
    }
}
finally {
    Pop-Location
}

Write-Host 'Build OK.'
