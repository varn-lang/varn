#!/usr/bin/env bash
# Script de pre-verificación local de Varn para entornos Unix (Linux / macOS)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

FAST=0
SKIP_LINT=0
CLEAN_CACHE=0

for arg in "$@"; do
    case $arg in
        --fast|-f)
            FAST=1
            shift
            ;;
        --skip-lint)
            SKIP_LINT=1
            shift
            ;;
        --clean-cache)
            CLEAN_CACHE=1
            shift
            ;;
    esac
done

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

FAILED_STEPS=()

header() {
    echo ""
    echo -e "${CYAN}======================================================================${NC}"
    echo -e "${CYAN}  >> $1${NC}"
    echo -e "${CYAN}======================================================================${NC}"
}

pass() {
    echo -e "${GREEN} [PASS] $1${NC}"
}

fail() {
    echo -e "${RED} [FAIL] $1${NC}"
}

warn() {
    echo -e "${YELLOW} [WARN] $1${NC}"
}

START_TIME=$(date +%s)

# 1. Formato
if [ $SKIP_LINT -eq 0 ]; then
    header "1/6: Verificación de Formateo (cargo fmt --check)"
    if cargo fmt --all -- --check; then
        pass "Formato de código correcto."
    else
        fail "El código tiene diferencias de formateo con cargo fmt."
        FAILED_STEPS+=("cargo fmt --check")
    fi

    # 2. Clippy
    header "2/6: Análisis Estático con Clippy (cargo clippy)"
    if cargo clippy --workspace --all-targets -- -D warnings; then
        pass "Clippy completado exitosamente."
    else
        fail "Clippy encontró avisos o errores."
        FAILED_STEPS+=("cargo clippy")
    fi
else
    warn "Saltando pasos de formateo y clippy (--skip-lint activo)."
fi

# 3. Gobernanza de Tamaño de Archivos
header "3/6: Auditoría de Gobernanza de Tamaño de Archivos (Anti-God Files)"
LARGE_FILES=0
CRITICAL_FILES=0

while IFS= read -r file; do
    LINES=$(wc -l < "$file")
    if [ "$LINES" -gt 1000 ]; then
        fail "$file tiene $LINES líneas (>1000 líneas - PROHIBIDO)"
        CRITICAL_FILES=$((CRITICAL_FILES + 1))
    elif [ "$LINES" -gt 400 ]; then
        warn "$file tiene $LINES líneas (>400 líneas - techo AGENTS §6)"
        LARGE_FILES=$((LARGE_FILES + 1))
    fi
done < <(find crates -name "*.rs")

if [ $CRITICAL_FILES -gt 0 ]; then
    FAILED_STEPS+=("File Size Governance (>1000 lines)")
else
    pass "Todos los archivos de crates cumplen con el límite de 1000 líneas."
fi

# 4. Compilación
header "4/6: Compilación de Producción (cargo build --release --bin vn)"
if cargo build --release --bin vn; then
    pass "Binario 'vn' compilado exitosamente."
else
    fail "Error al compilar el binario 'vn'."
    FAILED_STEPS+=("cargo build --release")
    exit 1
fi

VN_BIN="./target/release/vn"

header "4c/6: Bundle std para cuadrantes @embedded (vn std-bundle)"
if $VN_BIN std-bundle --std-dir "std" --out "dist/std.vnb"; then
    pass "Bundle std listo."
else
    fail "No se pudo compilar el bundle std."
    FAILED_STEPS+=("vn std-bundle")
fi
STD_BUNDLE="$PWD/dist/std.vnb"

header "4d/6: Corpus de errores (tests/errors)"
if pwsh -NoProfile -File scripts/error-corpus.ps1 -VnBin "$VN_BIN"; then
    pass "Corpus de errores: todas las fixtures conformes."
else
    fail "Corpus de errores en rojo."
    FAILED_STEPS+=("Corpus de errores (tests/errors)")
fi

header "4d/6: Formato del código Varn (vn fmt --check)"
if $VN_BIN fmt --check; then
    pass "Formato del código Varn: todo conforme a vn fmt."
else
    fail "Hay código Varn sin formatear."
    FAILED_STEPS+=("Formato del código Varn")
fi

header "4e/6: Bails del JIT"
if pwsh -NoProfile -File scripts/jit-bails.ps1 -VnBin "$VN_BIN"; then
    pass "Bails del JIT: solo los previstos (scripts/jit-bails.allow)."
else
    fail "El JIT rechazó funciones que debía compilar."
    FAILED_STEPS+=("Bails del JIT")
fi

header "4f/6: Capas del workspace"
if pwsh -NoProfile -File scripts/layers.ps1; then
    pass "Capas del workspace: solo las dependencias previstas (scripts/layers.allow)."
else
    fail "Hay dependencias fuera del contrato de capas."
    FAILED_STEPS+=("Capas del workspace")
fi

# Helper cuadrante
run_quadrant() {
    local name="$1"
    local env_flags="$2"
    local cmd_args="$3"

    echo ""
    echo -e "${YELLOW}--- Ejecutando Cuadrante: $name ---${NC}"

    if [ $CLEAN_CACHE -eq 1 ]; then
        $VN_BIN cache clean > /dev/null 2>&1 || true
    fi

    if eval "$env_flags $VN_BIN $cmd_args"; then
        pass "Cuadrante superado: $name"
        return 0
    else
        fail "Cuadrante fallido: $name"
        return 1
    fi
}

# 5. Matriz de Validación de 4 Cuadrantes
header "5/6: Matriz Obligatoria de 4 Cuadrantes (tests/*.test.vn)"

# Q1: dev-checkout + JIT
if ! run_quadrant "1/4: [dev-checkout] + [JIT Habilitado]" "" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("Cuadrante 1 (dev-checkout + JIT)")
fi

# Q2: dev-checkout + No-JIT
if ! run_quadrant "2/4: [dev-checkout] + [Intérprete Pure (VN_NO_JIT=1)]" "VN_NO_JIT=1" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("Cuadrante 2 (dev-checkout + No-JIT)")
fi

# Q3: @embedded + JIT
if ! run_quadrant "3/4: [@embedded std] + [JIT Habilitado]" "VN_STD=@embedded VN_STD_BUNDLE=$STD_BUNDLE" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("Cuadrante 3 (@embedded + JIT)")
fi

# Q4: @embedded + No-JIT
if ! run_quadrant "4/4: [@embedded std] + [Intérprete Pure (VN_NO_JIT=1)]" "VN_STD=@embedded VN_STD_BUNDLE=$STD_BUNDLE VN_NO_JIT=1" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("Cuadrante 4 (@embedded + No-JIT)")
fi

# Q5: caché JIT tibia de Q1 verificada byte a byte
if ! run_quadrant "5: [cache JIT] + [VN_JIT_CACHE=verify]" "VN_JIT_CACHE=verify" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("Cuadrante 5 (cache JIT verify)")
fi

# GC: estrés con verificador de invariantes
if ! run_quadrant "GC: [estrés GC 256] + [verificador de invariantes]" "VN_GC_STRESS=256 VN_GC_VERIFY=1" "test \"tests/*.test.vn\""; then
    FAILED_STEPS+=("GC estrés + verificador")
fi

# 6. Benchmark
if [ $FAST -eq 0 ]; then
    header "6/6: Benchmark de Estabilidad (vn bench tests/benchmarks/vn/bench_fib.vn -v)"
    if $VN_BIN bench tests/benchmarks/vn/bench_fib.vn -v; then
        pass "Benchmark de estabilidad completado satisfactoriamente."
    else
        fail "El benchmark de estabilidad reportó errores."
        FAILED_STEPS+=("Benchmark de Estabilidad")
    fi
else
    warn "Saltando paso de benchmark (--fast activo)."
fi

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo -e "${CYAN}======================================================================${NC}"
echo -e "${CYAN}  RESUMEN DE PRE-VERIFICACIÓN LOCAL (Tiempo total: ${ELAPSED}s)${NC}"
echo -e "${CYAN}======================================================================${NC}"

if [ ${#FAILED_STEPS[@]} -eq 0 ]; then
    echo -e "\n${GREEN} [TODO CORRECTO] Todas las verificaciones y la matriz de 4 cuadrantes pasaron exitosamente.${NC}"
    echo -e "${GREEN} El código está listo para producción y es seguro hacer commit / push.\n${NC}"
    exit 0
else
    echo -e "\n${RED} [FALLOS DETECTADOS] Los siguientes pasos fallaron:${NC}"
    for step in "${FAILED_STEPS[@]}"; do
        echo -e "${RED}  - $step${NC}"
    done
    echo -e "\n${RED} Por favor corrige los errores anteriores antes de enviar a producción.\n${NC}"
    exit 1
fi
