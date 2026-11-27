<#
.SYNOPSIS
  Fails when the JIT rejects a function that it should compile.

.DESCRIPTION
  Runs `vn debug -p bails` over the integration suite and the benchmarks. The
  only accepted bails are functions that use dynamic operations with no
  portable SSA form (`from_ssa unavailable: no portable form for ...`); any
  other reason means the JIT silently fell back to the interpreter.
  Called by scripts/verify.ps1; can also be run by hand.

.PARAMETER VnBin
  Path to the vn executable (default: target/release/vn).
#>
[CmdletBinding()]
param(
    [string]$VnBin
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $VnBin) {
    $exe = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'vn.exe' } else { 'vn' }
    $VnBin = Join-Path $root "target/release/$exe"
}

$allowed = 'from_ssa unavailable: no portable form for'
$expected = @(
    Get-Content (Join-Path $PSScriptRoot 'jit-bails.allow') |
        Where-Object { $_ -and -not $_.StartsWith('#') }
)
$files = @(
    Get-ChildItem -Path (Join-Path $root 'tests') -Filter '*.test.vn' -File
    Get-ChildItem -Path (Join-Path $root 'tests/benchmarks/vn') -Filter 'bench_*.vn' -File
) | Sort-Object FullName

$previousEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Push-Location $root
try {
    $offenders = $files | ForEach-Object -ThrottleLimit 8 -Parallel {
        $env:NO_COLOR = '1'
        $rows = & $using:VnBin debug -p bails -A $_.FullName 2>&1 | Out-String
        $inTable = $false
        foreach ($line in ($rows -split "`r?`n")) {
            if ($line.Contains('bloqueada')) { $inTable = $true; continue }
            if ($line.Contains('end: bails')) { $inTable = $false }
            if (-not $inTable) { continue }
            $isRow = $line.Contains([string][char]0x2502) -and -not $line.TrimStart().StartsWith('funci')
            if (-not $isRow -or $line.Contains($using:allowed)) { continue }
            $function = ($line -split [string][char]0x2502)[0].Trim()
            if ($using:expected -contains "$($_.Name)|$function") { continue }
            [pscustomobject]@{ File = $_.Name; Row = ($line -replace '\s+', ' ').Trim() }
        }
    }
}
finally {
    Pop-Location
    [Console]::OutputEncoding = $previousEncoding
}

$offenders = @($offenders)
Write-Host "JIT bails: $($files.Count) archivos revisados."
if ($offenders.Count -gt 0) {
    Write-Host "El JIT rechazo funciones que debia compilar:" -ForegroundColor Red
    $offenders | Select-Object -First 20 | ForEach-Object { Write-Host "  $($_.File): $($_.Row)" -ForegroundColor Red }
    exit 1
}
exit 0
