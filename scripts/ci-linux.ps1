<#
.SYNOPSIS
    Reproduce en un Linux limpio (Docker) lo que el CI ejecuta en ubuntu-latest.

.DESCRIPTION
    Con el mismo Rust que fija rust-toolchain.toml:
      1. cargo fmt --check y cargo clippy -D warnings    (job "Code Quality & Linting")
      2. cargo build --profile quick --bin vn
      3. las puertas estaticas de verify.ps1: corpus de errores, vn fmt --check,
         bails del JIT y capas del workspace
      4. los 4 cuadrantes de tests/*.test.vn (dev-checkout/@embedded x JIT/No-JIT)
      5. la cache JIT tibia verificada (VN_JIT_CACHE=verify)
      6. el estres del GC con verificador de invariantes (VN_GC_STRESS=256, VN_GC_VERIFY=1)
    Lo que solo se rompe en Linux (cfg, rutas, FFI SysV, sockets, lints) no aparece en
    verify.ps1. Correr este script antes de hacer push cuando el cambio toque codigo
    dependiente de plataforma.

    Requiere Docker Desktop en ejecucion. Los volumenes con nombre cachean target/ y el
    registro de cargo, asi que solo la primera ejecucion compila todo.
#>
$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

docker info *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker no esta en ejecucion: arranca Docker Desktop y vuelve a intentarlo." -ForegroundColor Red
    exit 2
}

$toolchain = (Select-String -Path (Join-Path $repo "rust-toolchain.toml") -Pattern 'channel\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value

$steps = @'
set -e
apt-get update -qq >/dev/null && apt-get install -y -qq fonts-dejavu-core fontconfig >/dev/null && fc-cache -f
rustup component add rustfmt clippy >/dev/null 2>&1 || true
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --profile quick --bin vn
BIN=/target/quick/vn
mkdir -p /opt/pwsh && curl -fsSL https://github.com/PowerShell/PowerShell/releases/download/v7.4.6/powershell-7.4.6-linux-x64.tar.gz | tar -xz -C /opt/pwsh
PWSH=/opt/pwsh/pwsh
chmod +x $PWSH
$PWSH -NoProfile -File scripts/error-corpus.ps1 -VnBin $BIN
$BIN fmt --check
$PWSH -NoProfile -File scripts/jit-bails.ps1 -VnBin $BIN
$PWSH -NoProfile -File scripts/layers.ps1
$BIN std-bundle --std-dir std --out /tmp/std.vnb
echo "== Quadrant 1 (dev-checkout + JIT)";    timeout 900 $BIN test "tests/*.test.vn"
echo "== Quadrant 2 (dev-checkout + No-JIT)"; VN_NO_JIT=1 timeout 900 $BIN test "tests/*.test.vn"
echo "== Quadrant 3 (@embedded + JIT)";       VN_STD=@embedded VN_STD_BUNDLE=/tmp/std.vnb timeout 900 $BIN test "tests/*.test.vn"
echo "== Quadrant 4 (@embedded + No-JIT)";    VN_STD=@embedded VN_STD_BUNDLE=/tmp/std.vnb VN_NO_JIT=1 timeout 900 $BIN test "tests/*.test.vn"
echo "== Quadrant 5 (JIT cache verify)";    VN_JIT_CACHE=verify timeout 900 $BIN test "tests/*.test.vn"
echo "== GC (stress 256 + verifier)"; VN_GC_STRESS=256 VN_GC_VERIFY=1 timeout 1800 $BIN test "tests/*.test.vn"
echo CI_LINUX_OK
'@ -replace "`r`n", "`n"

# El script viaja como archivo: pasarlo como argumento de `docker run` en Windows
# pierde las comillas dobles y bash recibe un script truncado que "pasa" sin ejecutar nada.
$tmp = Join-Path $repo "target/tmp"
New-Item -ItemType Directory -Force $tmp | Out-Null
[System.IO.File]::WriteAllText((Join-Path $tmp "ci-linux.sh"), $steps)

docker run --rm `
    -v "${repo}:/work" `
    -v vn-ci-target:/target `
    -v vn-ci-cargo:/usr/local/cargo/registry `
    -e CARGO_TARGET_DIR=/target `
    -e VN_STD=/work/std `
    -w /work "rust:$toolchain" bash /work/target/tmp/ci-linux.sh
exit $LASTEXITCODE
