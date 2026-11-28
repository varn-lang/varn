param(
    [string]$Vn = (Join-Path $PSScriptRoot '..\target\quick\vn.exe'),
    [int]$Stress = 256
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$vn = if ([IO.Path]::IsPathRooted($Vn)) { (Resolve-Path $Vn).Path } else { (Resolve-Path (Join-Path $repo $Vn)).Path }

function Invoke-Mode($arguments, $variables) {
    $saved = @{}
    foreach ($key in $variables.Keys) {
        $saved[$key] = [Environment]::GetEnvironmentVariable($key)
        Set-Item -Path "Env:$key" -Value $variables[$key]
    }
    try {
        $lines = & $vn @arguments 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        foreach ($key in $saved.Keys) {
            if ($null -eq $saved[$key]) {
                Remove-Item "Env:$key" -ErrorAction SilentlyContinue
            } else {
                Set-Item -Path "Env:$key" -Value $saved[$key]
            }
        }
    }
    [pscustomobject]@{ Lines = @($lines); Code = $code }
}

function Get-Verdicts($lines) {
    $lines | ForEach-Object { if ($_ -match '^\s+(ok|FAILED)\s+(\S+)') { "$($Matches[1]) $($Matches[2])" } } | Sort-Object
}

function Test-Case($name, $arguments, $canonical) {
    $plain = Invoke-Mode $arguments @{}
    $stressVars = @{ VN_GC_STRESS = "$Stress"; VN_GC_VERIFY = '1' }
    $stress = Invoke-Mode $arguments $stressVars
    $left = if ($canonical) { Get-Verdicts $plain.Lines } else { $plain.Lines }
    $right = if ($canonical) { Get-Verdicts $stress.Lines } else { $stress.Lines }
    $diff = Compare-Object @($left) @($right) -SyncWindow 0
    if ($plain.Code -ne $stress.Code) {
        Write-Host "[FAIL] $name : exit codes differ ($($plain.Code) vs $($stress.Code))" -ForegroundColor Red
        return $false
    }
    if ($null -ne $diff) {
        Write-Host "[FAIL] $name : $($diff.Count) diverging line(s)" -ForegroundColor Red
        $diff | Select-Object -First 20 | Format-Table -AutoSize
        return $false
    }
    Write-Host "[PASS] $name : $($left.Count) line(s) identical" -ForegroundColor Green
    $true
}

$ok = $true
$ok = (Test-Case 'corpus' @('test', 'tests/*.test.vn') $true) -and $ok
$ok = (Test-Case 'object graphs' @('run', 'tests/gc/graphs.vn') $false) -and $ok
if (-not $ok) {
    Write-Host "GC differential verification failed." -ForegroundColor Red
    exit 1
}
Write-Host "GC differential verification passed." -ForegroundColor Green
