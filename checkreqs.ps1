$ErrorActionPreference = 'SilentlyContinue'

$script:failed = 0

function Write-CheckResult {
    param(
        [bool] $Ok,
        [string] $Label,
        [string] $Detail
    )

    $status = if ($Ok) { 'OK  ' } else { 'FAIL' }
    Write-Host "[$status] $Label - $Detail"
    if (-not $Ok) { $script:failed++ }
}

function Get-VersionOutput {
    param(
        [string] $Command,
        [string[]] $Arguments = @()
    )

    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) { return $null }
    return (& $Command @Arguments 2>&1 | Out-String).Trim()
}

$cargoVersion = Get-VersionOutput -Command cargo -Arguments '--version'
if ($null -eq $cargoVersion) {
    Write-CheckResult $false 'cargo' 'not found on PATH'
}
elseif ($cargoVersion -match '^cargo\s+(\d+\.\d+\.\d+)') {
    Write-CheckResult $true 'cargo' $cargoVersion
}
else {
    Write-CheckResult $false 'cargo' "unexpected version output: $cargoVersion"
}

$rustcVersion = Get-VersionOutput -Command rustc -Arguments '--version'
if ($null -eq $rustcVersion) {
    Write-CheckResult $false 'rustc' 'not found on PATH'
}
elseif ($rustcVersion -match '^rustc\s+(\d+\.\d+\.\d+)') {
    Write-CheckResult $true 'rustc' $rustcVersion
}
else {
    Write-CheckResult $false 'rustc' "unexpected version output: $rustcVersion"
}

$pythonVersion = Get-VersionOutput -Command python -Arguments '--version'
$pythonCommand = 'python'
if ($null -eq $pythonVersion) {
    $pythonVersion = Get-VersionOutput -Command py -Arguments @('-3', '--version')
    $pythonCommand = 'py -3'
}
if ($null -eq $pythonVersion) {
    Write-CheckResult $false 'Python' 'python or py launcher not found on PATH (need 3.12+)'
}
elseif ($pythonVersion -match 'Python\s+(\d+)\.(\d+)(?:\.(\d+))?') {
    $major = [int]$Matches[1]
    $minor = [int]$Matches[2]
    $ok = $major -gt 3 -or ($major -eq 3 -and $minor -ge 12)
    Write-CheckResult $ok 'Python' "$pythonVersion (via $pythonCommand; need 3.12+)"
}
else {
    Write-CheckResult $false 'Python' "unexpected version output: $pythonVersion"
}

Write-Host ''
if ($script:failed -gt 0) {
    Write-Host "$script:failed requirement check(s) failed."
    exit 1
}
Write-Host 'All requirements present.'
exit 0
