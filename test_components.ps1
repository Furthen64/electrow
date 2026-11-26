$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot

& "$root\winbuild.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location $root
try {
    $scenarioDir = Join-Path $root 'tests\ComponentTests'
    $scenarios = Get-ChildItem -Path $scenarioDir -Filter '*.json' | Sort-Object Name
    if ($scenarios.Count -eq 0) {
        Write-Host 'No component scenarios found in tests\ComponentTests\'
        exit 1
    }

    $failed = @()
    foreach ($scenario in $scenarios) {
        python tests/check_component.py $scenario.FullName
        if ($LASTEXITCODE -ne 0) {
            $failed += $scenario.Name
        }
    }

    if ($failed.Count -gt 0) {
        Write-Host "FAILED scenarios: $($failed -join ', ')"
        exit 1
    }
    Write-Host "Component tests OK ($($scenarios.Count) scenarios)."
}
finally {
    Pop-Location
}
