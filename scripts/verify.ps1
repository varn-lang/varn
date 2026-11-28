<#
.SYNOPSIS
    Script de pre-verificación local de Varn para validar cambios antes de commit/push.

.DESCRIPTION
    Ejecuta el conjunto completo de validaciones requeridas por el proyecto:
    1. Formateo de código (cargo fmt)
    2. Linter (cargo clippy)
    3. Auditoría de gobernanza de tamaño de archivos
    4. Compilación del binario de producción (cargo build --release --bin vn,
       con sccache automático si está instalado)
    5. Suite de integracion con `vn test "tests/*.test.vn"`: el camino de
       produccion (@embedded + JIT) con -Fast, la matriz completa sin
       banderas (Q1-Q4 + estres del GC en paralelo con Start-Job, cada uno
       con su propio VN_HOME/VN_CACHE_DIR; cada test corre ademas aislado
       en su propio directorio temporal via sandbox fs, asi que el orden
       de ejecucion no importa), mas el cuadrante 5 que verifica la cache
       JIT tibia de Q1 byte a byte (VN_JIT_CACHE=verify)
    6. Benchmark de estabilidad (opcional con -Fast)

.PARAMETER Fast
    Puerta previa al push: corre solo el camino que se distribuye (std @embedded
    con JIT) y omite el benchmark. La matriz de 4 cuadrantes completa la corre el
    CI en cada push; sin banderas se corre aquí completa, útil al tocar JIT, su
    caché o el cargador de std.

.PARAMETER SkipLint
    Omite cargo fmt y cargo clippy.

.PARAMETER CleanCache
    Limpia la caché de Varn (`vn cache clean`) antes de cada cuadrante.
    En la matriz en paralelo las cachés ya son directorios frescos por
    trabajo; el cuadrante 5 nunca se limpia porque debe reutilizar la
    caché tibia de Q1.

.PARAMETER Quick
    Ciclo de iteración: compila con el perfil `quick` (sin LTO, codegen-units=16),
    omite lint y benchmark, y corre solo el cuadrante 1 (dev-checkout + JIT).
    No sustituye a la matriz completa antes de commitear.

.PARAMETER Min
    Ejecucion minima: perfil `quick`, sin lint, sin auditoria de tamano, sin
    goldens (debug goldens siguen huerfanos), y solo el cuadrante 1
    (`vn test "tests/*.test.vn"` con dev-checkout + JIT). Implica -Quick.
    El corpus tests/errors SI corre en -Min: es estatico y barato.
    Es lo mas barato que sigue ejecutando la suite de integracion.

.PARAMETER Jobs
    Máximo de cuadrantes en paralelo en la matriz completa. 0 = automático
    (número de cuadrantes, acotado por $env:NUMBER_OF_PROCESSORS).

.PARAMETER NoParallel
    Fuerza la matriz completa en serie (comportamiento anterior). Útil para
    depurar un cuadrante o en máquinas con poca RAM.

.EXAMPLE
    .\scripts\verify.ps1
    .\scripts\verify.ps1 -Min
    .\scripts\verify.ps1 -Fast
    .\scripts\verify.ps1 -Quick
#>

param (
    [switch]$Fast,
    [switch]$SkipLint,
    [switch]$CleanCache,
    [switch]$Quick,
    [switch]$Min,
    [int]$Jobs = 0,
    [switch]$NoParallel
)

if ($Min) {
    $Quick = $true
}
if ($Quick) {
    $SkipLint = $true
    $Fast = $true
}
$BuildProfile = if ($Quick) { "quick" } else { "release" }

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

# sccache reutiliza objetos entre ramas y perfiles: en la segunda compilación
# del mismo árbol casi todo es cache hit. Solo se activa si está instalado
# (cargo install sccache); si no, el build sigue igual que antes.
if (-not $env:RUSTC_WRAPPER) {
    if (Get-Command sccache -ErrorAction SilentlyContinue) {
        $env:RUSTC_WRAPPER = "sccache"
    }
}

function Write-StepHeader($title) {
    Write-Host ""
    Write-Host ("=" * 70) -ForegroundColor Cyan
    Write-Host "  >> $title" -ForegroundColor Cyan
    Write-Host ("=" * 70) -ForegroundColor Cyan
}

function Write-Success($msg) {
    Write-Host " [PASS] $msg" -ForegroundColor Green
}

function Write-Failure($msg) {
    Write-Host " [FAIL] $msg" -ForegroundColor Red
}

function Write-WarningMsg($msg) {
    Write-Host " [WARN] $msg" -ForegroundColor Yellow
}

$startTime = [System.Diagnostics.Stopwatch]::StartNew()
$failedSteps = @()

# 1. Formateo
if (-not $SkipLint) {
    Write-StepHeader "1/6: Verificacion de Formateo (cargo fmt --check)"
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-WarningMsg "El codigo tiene diferencias de formateo con cargo fmt. Puedes ejecutar 'cargo fmt --all' para estandarizar."
    } else {
        Write-Success "Formato de codigo correcto."
    }

    # 2. Linter Clippy
    Write-StepHeader "2/6: Analisis Estatico con Clippy (cargo clippy)"
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Failure "Clippy encontro avisos o errores."
        $failedSteps += "cargo clippy"
    } else {
        Write-Success "Analisis de Clippy completado exitosamente."
    }
} else {
    Write-WarningMsg "Saltando pasos de formateo y clippy (-SkipLint activo)."
}

# 3. Auditoria de Tamano de Archivos (techo AGENTS §6: 400; >1000 falla)
if ($Min) {
    Write-WarningMsg "Saltando auditoria de tamano de archivos (-Min activo)."
} else {
Write-StepHeader "3/6: Auditoria de Gobernanza de Tamano de Archivos (Anti-God Files)"
$largeFiles = Get-ChildItem -Path "crates" -Recurse -Filter "*.rs" |
    ForEach-Object {
        $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines
        [PSCustomObject]@{
            Path   = $_.FullName.Replace("$RootDir\", "")
            Lines  = $lines
            Status = if ($lines -gt 1000) { "ERROR (>1000 lineas)" } elseif ($lines -gt 400) { "ADVERTENCIA (>400 lineas)" } else { "OK" }
        }
    } | Where-Object { $_.Lines -gt 400 } | Sort-Object Lines -Descending

if ($largeFiles) {
    $largeFiles | Format-Table -AutoSize
    $critical = $largeFiles | Where-Object { $_.Lines -gt 1000 }
    if ($critical) {
        Write-Failure "Se detectaron archivos que superan el limite estricto de 1000 lineas (Regla Anti-God File)."
        $failedSteps += "File Size Governance (>1000 lines)"
    } else {
        Write-WarningMsg "Archivos entre 400 y 1000 lineas detectados. Techo AGENTS §6: 400 por dominio."
    }
} else {
    Write-Success "Todos los archivos de crates cumplen con la gobernanza de tamano (<400 lineas)."
}
}

# 4. Compilacion en Modo Release
Write-StepHeader "4/6: Compilacion (cargo build --profile $BuildProfile --bin vn)"
cargo build --profile $BuildProfile --bin vn
if ($LASTEXITCODE -ne 0) {
    Write-Failure "Error al compilar el binario 'vn' (perfil $BuildProfile)."
    $failedSteps += "cargo build --profile $BuildProfile"
    exit 1
}
Write-Success "Binario 'vn' compilado exitosamente (perfil $BuildProfile)."
if ($env:RUSTC_WRAPPER -eq "sccache") {
    Write-Success "Compilacion acelerada con sccache."
} else {
    Write-Host " (nota) sccache no detectado: 'cargo install sccache' acelera 3-10x las recompilaciones." -ForegroundColor DarkGray
}

$vnBin = Join-Path $RootDir "target\$BuildProfile\vn.exe"
if (-not (Test-Path $vnBin)) {
    $vnBin = Join-Path $RootDir "target\$BuildProfile\vn"
}

$stdBundle = Join-Path $RootDir "dist\std.vnb"
if (-not $Quick) {
    Write-StepHeader "4c/6: Bundle std para cuadrantes @embedded (vn std-bundle)"
    & $vnBin std-bundle --std-dir "std" --out $stdBundle
    if ($LASTEXITCODE -ne 0) {
        Write-Failure "No se pudo compilar el bundle std."
        $failedSteps += "vn std-bundle"
    } else {
        Write-Success "Bundle std listo."
    }
}

Write-StepHeader "4d/6: Corpus de errores (tests/errors)"
$corpusOut = & (Join-Path $PSScriptRoot "error-corpus.ps1") -VnBin $vnBin 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    Write-Failure "Corpus de errores en rojo."
    Write-Host $corpusOut -ForegroundColor Red
    $failedSteps += "Corpus de errores (tests/errors)"
} else {
    Write-Success "Corpus de errores: todas las fixtures conformes."
}

Write-StepHeader "4d/6: Lint del codigo Varn (vn lint)"
$lintOut = & $vnBin lint --deny-warnings 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    Write-Failure "El linter encontro problemas en el codigo Varn."
    Write-Host $lintOut -ForegroundColor Red
    $failedSteps += "Lint del codigo Varn"
} else {
    Write-Success "Lint del codigo Varn: sin avisos."
}

Write-StepHeader "4d/6: Formato del codigo Varn (vn fmt --check)"
$fmtOut = & $vnBin fmt --check 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    Write-Failure "Hay codigo Varn sin formatear (o que el formateador no puede procesar)."
    Write-Host $fmtOut -ForegroundColor Red
    $failedSteps += "Formato del codigo Varn"
} else {
    Write-Success "Formato del codigo Varn: todo conforme a vn fmt."
}

if (-not $Quick) {
    Write-StepHeader "4e/6: Bails del JIT (funciones que el JIT deja al interprete)"
    $bailsOut = & (Join-Path $PSScriptRoot "jit-bails.ps1") -VnBin $vnBin 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        Write-Failure "El JIT rechazo funciones que debia compilar."
        Write-Host $bailsOut -ForegroundColor Red
        $failedSteps += "Bails del JIT"
    } else {
        Write-Success "Bails del JIT: solo los previstos (scripts/jit-bails.allow)."
    }

    Write-StepHeader "4f/6: Capas del workspace (dependencias entre crates)"
    $layersOut = & (Join-Path $PSScriptRoot "layers.ps1") 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        Write-Failure "Hay dependencias fuera del contrato de capas."
        Write-Host $layersOut -ForegroundColor Red
        $failedSteps += "Capas del workspace"
    } else {
        Write-Success "Capas del workspace: solo las dependencias previstas (scripts/layers.allow)."
    }
}

# Helper para ejecutar un cuadrante
function Run-Quadrant($name, $envVars, $argsList) {
    Write-Host "`n--- Ejecutando Cuadrante: $name ---" -ForegroundColor Yellow
    
    # Guardar estado anterior de variables
    $prevVars = @{}
    foreach ($k in $envVars.Keys) {
        $prevVars[$k] = [System.Environment]::GetEnvironmentVariable($k, "Process")
        [System.Environment]::SetEnvironmentVariable($k, $envVars[$k], "Process")
    }

    if ($CleanCache) {
        & $vnBin cache clean | Out-Null
    }

    $proc = Start-Process -FilePath $vnBin -ArgumentList $argsList -NoNewWindow -PassThru -Wait
    $code = $proc.ExitCode

    # Restaurar variables de entorno
    foreach ($k in $envVars.Keys) {
        [System.Environment]::SetEnvironmentVariable($k, $prevVars[$k], "Process")
    }

    if ($code -ne 0) {
        Write-Failure "Cuadrante fallido: $name (Exit Code: $code)"
        return $false
    } else {
        Write-Success "Cuadrante superado: $name"
        return $true
    }
}

$VnTestCmd = "test"
$VnTestPattern = Join-Path $RootDir "tests\*.test.vn"

function Get-MaxParallelism($count) {
    if ($NoParallel) { return 1 }
    if ($Jobs -gt 0) { return [Math]::Max(1, [Math]::Min($Jobs, $count)) }
    $cpu = 4
    try { $cpu = [Math]::Max(1, [int]$env:NUMBER_OF_PROCESSORS) } catch { $cpu = 4 }
    return [Math]::Min($cpu, $count)
}

# Ejecuta los cuadrantes en paralelo con Start-Job. Cada trabajo recibe su
# propio VN_HOME/VN_CACHE_DIR para no contener en la cache en disco; los
# archivos temporales que escribe cada test ya van aislados por la sandbox
# de `vn test`, asi que el CWD del trabajo no necesita aislarse por test.
# Devuelve @{ Failed = @(...); ScratchUsed = $bool }.
# Con -NoParallel o -Jobs 1 cae al camino en serie anterior.
function Run-QuadrantsParallel($specs, $scratchRoot) {
    $failedNames = @()
    $maxPar = Get-MaxParallelism $specs.Count
    if ($maxPar -le 1) {
        foreach ($s in $specs) {
            $ok = Run-Quadrant $s.Name $s.Env $s.Args
            if (-not $ok) { $failedNames += $s.Name }
        }
        return [PSCustomObject]@{ Failed = $failedNames; ScratchUsed = $false }
    }
    New-Item -ItemType Directory -Force -Path $scratchRoot | Out-Null

    $worker = {
        param($vnBin, $testCmd, $testPattern, $envList, $workDir, $homeDir, $cacheDir)
        $envVars = @{}
        foreach ($kv in $envList) {
            $eq = $kv.IndexOf("=")
            if ($eq -gt 0) { $envVars[$kv.Substring(0, $eq)] = $kv.Substring($eq + 1) }
        }
        foreach ($k in @("VN_STD", "VN_STD_BUNDLE", "VN_NO_JIT", "VN_JIT_CACHE", "VN_GC_STRESS", "VN_GC_VERIFY", "VN_HOME", "VN_CACHE_DIR")) {
            if (-not $envVars.ContainsKey($k)) {
                [System.Environment]::SetEnvironmentVariable($k, $null, "Process")
            }
        }
        foreach ($k in $envVars.Keys) {
            [System.Environment]::SetEnvironmentVariable($k, $envVars[$k], "Process")
        }
        [System.Environment]::SetEnvironmentVariable("VN_HOME", $homeDir, "Process")
        [System.Environment]::SetEnvironmentVariable("VN_CACHE_DIR", $cacheDir, "Process")
        New-Item -ItemType Directory -Force -Path $workDir | Out-Null
        New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
        Set-Location $workDir
        $out = & $vnBin $testCmd $testPattern 2>&1 | Out-String
        [PSCustomObject]@{ ExitCode = $LASTEXITCODE; Output = $out }
    }

    Write-Host "Lanzando $($specs.Count) cuadrantes en paralelo (max $maxPar a la vez, scratch: $scratchRoot)..." -ForegroundColor Yellow
    $next = 0
    $tag = 0
    while ($next -lt $specs.Count) {
        $batch = @()
        $n = [Math]::Min($maxPar, ($specs.Count - $next))
        for ($i = 0; $i -lt $n; $i++) {
            $s = $specs[$next]; $next++; $tag++
            $envList = @($s.Env.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" })
            $workDir = Join-Path $scratchRoot "q$tag-work"
            $homeDir = Join-Path $scratchRoot "q$tag-home"
            $cacheDir = Join-Path $homeDir "cache"
            $job = Start-Job -ScriptBlock $worker -ArgumentList @($vnBin, $VnTestCmd, $VnTestPattern, $envList, $workDir, $homeDir, $cacheDir) -Name "verify-q$tag"
            $batch += [PSCustomObject]@{ Job = $job; Spec = $s; Tag = $tag }
            Write-Host "  [job $($job.Id)] $($s.Name)" -ForegroundColor DarkCyan
        }
        Wait-Job -Job ($batch | ForEach-Object { $_.Job }) -Timeout 1200 | Out-Null
        foreach ($b in $batch) {
            $logPath = Join-Path $scratchRoot "q$($b.Tag).log"
            if ($b.Job.State -eq "Running") {
                Stop-Job -Job $b.Job | Out-Null
                Write-Failure "Cuadrante agotado por tiempo (20 min): $($b.Spec.Name)"
                $failedNames += $b.Spec.Name
            } else {
                $r = Receive-Job -Job $b.Job -ErrorAction SilentlyContinue
                if ($r -is [array]) { $r = $r[-1] }
                $text = if ($r -and $r.Output) { [string]$r.Output } else { "" }
                $reason = $b.Job.ChildJobs[0].JobStateInfo.Reason
                if ($reason) { $text += "`n[Job $($b.Job.State)]: $reason" }
                $text | Out-File -FilePath $logPath -Encoding utf8
                $code = if ($r -and $null -ne $r.ExitCode) { [int]$r.ExitCode } else { 1 }
                if ($code -ne 0) {
                    Write-Failure "Cuadrante fallido: $($b.Spec.Name) (Exit Code: $code) -- log: $logPath"
                    Write-Host $text
                    $failedNames += $b.Spec.Name
                } else {
                    Write-Success "Cuadrante superado: $($b.Spec.Name) (log: $logPath)"
                }
            }
            Remove-Job -Job $b.Job -Force
        }
    }
    return [PSCustomObject]@{ Failed = $failedNames; ScratchUsed = $true }
}

# 5. Suite de integracion
if ($Fast -and -not $Quick) {
    Write-StepHeader "5/6: Camino de produccion (vn test tests/*.test.vn con @embedded + JIT)"
    $q3 = Run-Quadrant "[@embedded std] + [JIT Habilitado]" @{ "VN_STD" = "@embedded"; "VN_STD_BUNDLE" = $stdBundle } @($VnTestCmd, $VnTestPattern)
    if (-not $q3) { $failedSteps += "Camino de produccion (@embedded + JIT)" }
    Write-WarningMsg "Cuadrantes 1, 2, 4 y 5 los corre el CI; correr sin banderas al tocar JIT, su cache o el cargador de std."
} else {
Write-StepHeader "5/6: Matriz Obligatoria de 4 Cuadrantes (vn test tests/*.test.vn)"

# Q1-Q4 + estres del GC: independientes entre si, corren en paralelo con
# cachés y CWD aislados. Q5 (verificacion de la caché JIT) corre después,
# reutilizando la caché tibia de Q1 para que el modo verify compare hits
# reales en vez de una caché fría.
$matrixSpecs = @(
    @{ Name = "1/4: [dev-checkout] + [JIT Habilitado]"; Env = @{}; Args = @($VnTestCmd, $VnTestPattern) },
    @{ Name = "2/4: [dev-checkout] + [Interprete Pure (VN_NO_JIT=1)]"; Env = @{ "VN_NO_JIT" = "1" }; Args = @($VnTestCmd, $VnTestPattern) },
    @{ Name = "3/4: [@embedded std] + [JIT Habilitado]"; Env = @{ "VN_STD" = "@embedded"; "VN_STD_BUNDLE" = $stdBundle }; Args = @($VnTestCmd, $VnTestPattern) },
    @{ Name = "4/4: [@embedded std] + [Interprete Pure (VN_NO_JIT=1)]"; Env = @{ "VN_STD" = "@embedded"; "VN_STD_BUNDLE" = $stdBundle; "VN_NO_JIT" = "1" }; Args = @($VnTestCmd, $VnTestPattern) },
    @{ Name = "GC: [estres GC 256] + [verificador de invariantes]"; Env = @{ "VN_GC_STRESS" = "256"; "VN_GC_VERIFY" = "1" }; Args = @($VnTestCmd, $VnTestPattern) }
)

if ($Quick) {
    $q1 = Run-Quadrant "1/4: [dev-checkout] + [JIT Habilitado]" @{} @($VnTestCmd, $VnTestPattern)
    if (-not $q1) { $failedSteps += "Cuadrante 1 (dev-checkout + JIT)" }
    Write-WarningMsg "Saltando cuadrantes 2-4 (-Quick activo): correr la matriz completa antes de commitear."
} else {
    $scratchRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vn-verify-" + (Get-Date -Format "yyyyMMdd-HHmmss") + "-$PID")
    $matrixResult = Run-QuadrantsParallel $matrixSpecs $scratchRoot
    foreach ($f in $matrixResult.Failed) { $failedSteps += $f }

    $q5Env = @{ "VN_JIT_CACHE" = "verify" }
    if ($matrixResult.ScratchUsed) {
        $q5Env["VN_CACHE_DIR"] = Join-Path $scratchRoot "q1-home\cache"
    }
    # Q5 debe correr sobre la caché tibia de Q1: nunca limpiarla aquí.
    $cc = $CleanCache
    $CleanCache = $false
    $q5 = Run-Quadrant "5: [cache JIT] + [VN_JIT_CACHE=verify]" $q5Env @($VnTestCmd, $VnTestPattern)
    $CleanCache = $cc
    if (-not $q5) { $failedSteps += "Cuadrante 5 (cache JIT verify)" }

    if ($failedSteps.Count -eq 0) {
        Remove-Item -Recurse -Force -Path $scratchRoot -ErrorAction SilentlyContinue
    } else {
        Write-WarningMsg "Scratch de verificacion conservado en: $scratchRoot"
    }
}
}

# 6. Benchmark de Estabilidad
if (-not $Fast) {
    Write-StepHeader "6/6: Benchmark de Estabilidad (vn bench tests/benchmarks/vn/bench_fib.vn -v)"
    $proc = Start-Process -FilePath $vnBin -ArgumentList @("bench", "tests/benchmarks/vn/bench_fib.vn", "-v") -NoNewWindow -PassThru -Wait
    if ($proc.ExitCode -ne 0) {
        Write-Failure "El benchmark de estabilidad reporto errores."
        $failedSteps += "Benchmark de Estabilidad"
    } else {
        Write-Success "Benchmark de estabilidad completado satisfactoriamente."
    }

    Write-StepHeader "6b/6: Tabla comparativa del README (xtask compare: Varn vs Bun vs Node)"
    try {
        & (Join-Path $PSScriptRoot "update-readme-benchmarks.ps1")
        Write-Success "README.md actualizado con las metricas actuales."
    } catch {
        Write-WarningMsg "No se pudo actualizar la tabla del README: $($_.Exception.Message)"
    }
} else {
    Write-WarningMsg "Saltando paso de benchmark (-Fast activo)."
}

$startTime.Stop()
$elapsed = [Math]::Round($startTime.Elapsed.TotalSeconds, 2)

# Resumen Final
Write-Host ""
Write-Host ("=" * 70) -ForegroundColor Cyan
Write-Host "  RESUMEN DE PRE-VERIFICACION LOCAL (Tiempo total: ${elapsed}s)" -ForegroundColor Cyan
Write-Host ("=" * 70) -ForegroundColor Cyan

if ($failedSteps.Count -eq 0) {
    $scope = if ($Min) { "vn test tests/*.test.vn (cuadrante 1)" } elseif ($Quick) { "el cuadrante 1" } elseif ($Fast) { "el camino de produccion (@embedded + JIT)" } else { "la matriz de 4 cuadrantes" }
    Write-Host "`n [TODO CORRECTO] Todas las verificaciones y $scope pasaron exitosamente." -ForegroundColor Green
    Write-Host " El codigo esta listo para produccion y es seguro hacer commit / push.`n" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n [FALLOS DETECTADOS] Los siguientes pasos fallaron:" -ForegroundColor Red
    foreach ($step in $failedSteps) {
        Write-Host "  - $step" -ForegroundColor Red
    }
    Write-Host "`n Por favor corrige los errores anteriores antes de enviar a produccion.`n" -ForegroundColor Red
    exit 1
}
