<#
.SYNOPSIS
  Fails when a workspace crate depends on a crate its layer does not allow.

.DESCRIPTION
  scripts/layers.allow lists, for every workspace crate, the complete set of
  workspace crates it may depend on (directly or transitively, with the
  features that crate enables on its own). The script runs `cargo tree` for
  each crate and reports every dependency that is not in its list.

  A new dependency is therefore a deliberate edit of layers.allow, visible in
  review. The contract it protects: the analysis crates (lexer, parser,
  binder, sem, checker, tir, emit, compiler, modules, pipeline, lsp) never
  depend on the runtime crates (vm, jit, gpu, ui, debug).

.PARAMETER Update
  Rewrites layers.allow from the current graph instead of checking it.
#>
[CmdletBinding()]
param(
    [switch]$Update
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$allowFile = Join-Path $PSScriptRoot 'layers.allow'

Push-Location $root
try {
    $crates = Get-ChildItem (Join-Path $root 'crates') -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName 'Cargo.toml') } |
        ForEach-Object { $_.Name } | Sort-Object

    $actual = [ordered]@{}
    foreach ($crate in $crates) {
        $tree = & cargo tree -p $crate -e normal --prefix none 2>$null
        if ($LASTEXITCODE -ne 0) { throw "cargo tree fallo para $crate" }
        $actual[$crate] = @(
            $tree | ForEach-Object { ($_ -split '\s+')[0] } |
                Where-Object { $_ -like 'vn-*' -and $_ -ne $crate } |
                Sort-Object -Unique
        )
    }
}
finally {
    Pop-Location
}

if ($Update) {
    $lines = @(
        '# Workspace crate -> workspace crates it may depend on (transitively).',
        '# Checked by scripts/layers.ps1; regenerate with -Update after a deliberate change.'
    )
    foreach ($crate in $actual.Keys) { $lines += "${crate}: $($actual[$crate] -join ' ')".TrimEnd() }
    [System.IO.File]::WriteAllText($allowFile, ($lines -join "`n") + "`n", (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "layers.allow actualizado ($($actual.Count) crates)."
    exit 0
}

$allowed = @{}
foreach ($line in Get-Content $allowFile) {
    if (-not $line -or $line.StartsWith('#')) { continue }
    $name, $rest = $line -split ':', 2
    $allowed[$name.Trim()] = @($rest -split '\s+' | Where-Object { $_ })
}

$violations = @()
foreach ($crate in $actual.Keys) {
    if (-not $allowed.ContainsKey($crate)) {
        $violations += "${crate}: no figura en layers.allow"
        continue
    }
    foreach ($dep in $actual[$crate]) {
        if ($allowed[$crate] -notcontains $dep) { $violations += "${crate} no puede depender de ${dep}" }
    }
}
foreach ($crate in $allowed.Keys) {
    if (-not $actual.Contains($crate)) { $violations += "${crate}: figura en layers.allow pero ya no existe" }
}

Write-Host "Capas: $($actual.Count) crates revisadas."
if ($violations.Count -gt 0) {
    Write-Host "Dependencias fuera del contrato de capas:" -ForegroundColor Red
    $violations | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    exit 1
}
exit 0
