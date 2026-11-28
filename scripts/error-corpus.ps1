<#
.SYNOPSIS
    Corpus de errores estaticos (tests/errors/*.vn).

.DESCRIPTION
    Cabecera `// expect:`, `vn check` (y `vn run` si el check pasa para fixtures
    de fallo), tokens ordenados, `help: ` presente, posiciones validas y exit
    code acorde. Sale con 1 si alguna fixture no es conforme.
#>
param (
    [Parameter(Mandatory = $true)][string]$VnBin
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $RootDir

$errDir = Join-Path $RootDir "tests/errors"
$fixtures = Get-ChildItem -Path $errDir -Filter *.vn | Sort-Object Name
Write-Host "Corpus de errores: $($fixtures.Count) fixtures"
$cacheDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vn-error-corpus-" + $PID)
New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
$prevCache = [System.Environment]::GetEnvironmentVariable("VN_CACHE_DIR", "Process")
$prevNoColor = [System.Environment]::GetEnvironmentVariable("NO_COLOR", "Process")
$prevOutEnc = [Console]::OutputEncoding
[System.Environment]::SetEnvironmentVariable("VN_CACHE_DIR", $cacheDir, "Process")
[System.Environment]::SetEnvironmentVariable("NO_COLOR", "1", "Process")
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$failures = @()
try {
    foreach ($f in $fixtures) {
        $rel = "tests/errors/" + $f.Name
        $head = @()
        foreach ($line in [System.IO.File]::ReadLines($f.FullName)) {
            $t = $line.Trim()
            if ($t.StartsWith("// expect: ")) { $head += $t.Substring("// expect: ".Length) }
            elseif ($head.Count -gt 0) { break }
        }
        $tokens = @(); $suggestions = @(); $fragments = @(); $badHeader = $false
        foreach ($h in $head) {
            if ($h.StartsWith("suggestion: ")) { $suggestions += $h.Substring("suggestion: ".Length); continue }
            $m = [regex]::Match($h, '^(error|warning)\[(.+)\]$')
            if ($m.Success -and ($m.Groups[2].Value.StartsWith("VN"))) {
                $tokens += "$($m.Groups[1].Value)[$($m.Groups[2].Value)]"; continue
            }
            $m2 = [regex]::Match($h, '^error\[(.+)\]$')
            if ($m2.Success -and -not ($m2.Groups[1].Value.StartsWith("VN"))) {
                $fragments += $m2.Groups[1].Value; continue
            }
            $badHeader = $true
        }
        if ($badHeader -or (($tokens.Count -eq 0) -and ($fragments.Count -eq 0)) -or (($tokens.Count -gt 0) -and ($fragments.Count -gt 0))) {
            $failures += "${rel}: cabecera invalida"; continue
        }
        $runOut = & $VnBin check $rel 2>&1 | Out-String
        $runCode = $LASTEXITCODE
        if ($fragments.Count -eq 0) {
            $emitted = @()
            foreach ($line in $runOut -split "`r?`n") {
                $t = $line.Trim()
                $m = [regex]::Match($t, '^(error|warning)\[VN[^\]]*\]')
                if ($m.Success) { $emitted += $m.Value }
            }
            $emitted = @($emitted | Sort-Object)
            $declared = @($tokens | Sort-Object)
            if (($emitted -join "`n") -ne ($declared -join "`n")) {
                $failures += "${rel}: declarados [$($declared -join ', ')] emitidos [$($emitted -join ', ')]"; continue
            }
            $miss = $false
            foreach ($s in $suggestions) {
                if (-not $runOut.Contains('help: ' + $s)) { $failures += "${rel}: falta sugerencia [$s]"; $miss = $true; break }
            }
            if ($miss) { continue }
            $badLoc = $false
            foreach ($line in $runOut -split "`r?`n") {
                $t = $line.Trim()
                if ($t.StartsWith([char]0x250C + "─ ")) {
                    $loc = $t.Substring(3)
                    $m = [regex]::Match($loc, ':(\d+):(\d+)\s*$')
                    if (-not $m.Success -or $m.Groups[1].Value -eq "0" -or $m.Groups[2].Value -eq "0") {
                        $failures += "${rel}: posicion invalida [$t]"; $badLoc = $true; break
                    }
                }
            }
            if ($badLoc) { continue }
            if (($declared | Where-Object { $_ -like "error*" }).Count -gt 0 -and $runCode -eq 0) {
                $failures += "${rel}: hay errores declarados pero `vn check` salio con 0"
            }
        } else {
            if ($runCode -eq 0) {
                $runOut = & $VnBin run $rel 2>&1 | Out-String
                $runCode = $LASTEXITCODE
            }
            if ($runCode -eq 0) { $failures += "${rel}: esperaba fallo y salio con 0"; continue }
            foreach ($frag in $fragments) {
                if (-not $runOut.Contains($frag)) { $failures += "${rel}: falta fragmento [$frag]"; break }
            }
        }
    }
} finally {
    [System.Environment]::SetEnvironmentVariable("VN_CACHE_DIR", $prevCache, "Process")
    [System.Environment]::SetEnvironmentVariable("NO_COLOR", $prevNoColor, "Process")
    [Console]::OutputEncoding = $prevOutEnc
    Remove-Item -Recurse -Force -Path $cacheDir -ErrorAction SilentlyContinue
}

if ($failures.Count -gt 0) {
    $failures | Select-Object -First 15 | ForEach-Object { Write-Host "  - $_" }
    Write-Host "Corpus de errores: $($failures.Count) fixtures en rojo."
    exit 1
}
Write-Host "Corpus de errores: todas las fixtures conformes."
exit 0
