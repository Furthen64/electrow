param(
    [string] $Address = '127.0.0.1',
    [int] $Port = 8000,
    [switch] $NoBuild,
    [switch] $Open
)

$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot

if (-not $NoBuild) {
    & "$root\winbuild.ps1"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Push-Location $root
try {
    $backendArgs = @('-m', 'backend', '--host', $Address, '--port', $Port)
    if ($Address -notin @('127.0.0.1', 'localhost', '::1')) { $backendArgs += '--allow-lan' }
    if ($Open) { $backendArgs += '--open' }
    python @backendArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
