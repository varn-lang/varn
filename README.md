# Varn

[![CI](https://github.com/varn-lang/varn/actions/workflows/ci.yml/badge.svg)](https://github.com/varn-lang/varn/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Version 0.1.0](https://img.shields.io/badge/version-0.1.0-orange.svg)](Cargo.toml)

**Sintaxis de alto nivel, rendimiento de bajo nivel.**

Varn es un lenguaje estáticamente tipado con sintaxis familiar al estilo TypeScript, compilado a bytecode de registros con VM propia y JIT Cranelift. El tipo que ve el checker es el tipo que ejecuta el runtime: sin `any` implícitos, sin coerciones ocultas, sin `undefined`.

```vn
import { describe, it, expect } from "std:test"

enum Forma { Circulo(float), Rect(float, float) }

function area(f: Forma): float {
    return match (f) {
        Circulo(r) => 3.14159 * r * r,
        Rect(w, h) => w * h,
    }
}

const par: #[int, str] = #[1, "a"]
const punto: #{ x: int, y: int } = #{ x: 10, y: 20 }

await describe("formas", async () => {
    await it("suma áreas", async () => {
        expect(area(Forma.Rect(2.0, 3.0))).toBe(6.0)
        expect(par[0]).toBe(1)
        expect(punto.x + punto.y).toBe(30)
    })
})
```

> Estado: pre-1.0 (`0.1.0`). APIs inestables por diseño: un *breaking change* se acepta cuando trae ganancia medible en corrección, determinismo o simplicidad.

---

## Por qué Varn

| Pilar | Qué significa en la práctica |
|---|---|
| **Estático de punta a punta** | El tipo del checker viaja al TIR, al bytecode y al JIT. `dynamic` existe, pero solo en fronteras explícitas con cast. |
| **Sintaxis sin ambigüedad** | Tuplas `#[a, b]`, records `#{ x: 1 }`, arrays `[x]`. `()` agrupa, `{ }` es bloque o literal de `struct`. Sin coma mágica `(x,)`, sin unidad `()`. |
| **Nulos y errores explícitos** | `T?`, `?.`, `??`, estrechamiento por flujo. `try`/`catch`/`finally` tipado más operador `try` sobre `Result`/`Option`/nulables. |
| **Bajo nivel controlado** | Permisos de host al estilo Deno (`--sandbox`, `--allow-read/write/net/env/ffi/ui`), tareas e isolates reales, `Regex` como tipo del lenguaje. |

## Qué ofrece

- **Tipos**: primitivos fijos (`int`, `float`, `str`, `bool`, …), uniones, tuplas `#[T, U]`, records `#{ x: T }`, enums con payload, `match` exhaustivo con guardas, genéricos con varianza.
- **Objetos**: clases, herencia, abstractas, constructores primarios, `struct` inmutables con igualdad estructural, interfaces, extensiones, decoradores `@throws` / `@capability` / `@pure`.
- **Async real**: `async`/`await`, generadores y generadores async, `Task`, `TaskGroup`, canales, isolates en hilos del SO.
- **Std amplia**: `fs`, `http`, `ws`, `net`, `sqlite`, `crypto`, `encoding` (JSON, CSV, TOML, msgpack…), `time`, `task`, `signals`, `test`, `gpu`, `ui`.
- **Tooling**: LSP, DAP, formateador, gestor de paquetes, `vn debug` por fases, `vn test` con cuadrantes JIT/intérprete.

## Empezar en 60 segundos

Requiere Rust (la toolchain está fijada en `rust-toolchain.toml`; rustup la instala sola).

```bash
git clone https://github.com/varn-lang/varn
cd varn
cargo build --profile quick --bin vn

./target/quick/vn run hola.vn
./target/quick/vn check hola.vn
./target/quick/vn debug -p bytecode hola.vn
```

- Primeros pasos: [primeros pasos](https://varn.twwk.cc/guias/primeros-pasos/).
- Instalación y perfiles: [instalación](https://varn.twwk.cc/guias/instalacion/).

## CLI

`run`, `check`, `eval`, `repl`, `bench`, `debug`, `build`, `test`, `task`, `fmt`, `pkg`, `init`, `doctor`, `cache`, `lsp`, `completions`, `explain`, `dap`, `std-bundle`.

Referencia completa y variables de entorno (`VN_*`, `VN_NO_JIT`, `VN_STD`, `VN_CACHE_DIR`): [CLI de `vn`](https://varn.twwk.cc/guias/cli/).

## Cómo está hecho

```text
texto → tokens → AST → check → TIR tipado → SSA → pases → bytecode → VM ⇄ JIT (Cranelift)
```

30 crates más `xtask` en un workspace (`vn-lexer`, `vn-parser`, `vn-checker`, `vn-emit`, `vn-compiler`, `vn-vm`, `vn-jit`, `vn-builtins`, `vn-lsp`…). Cada fase produce un artefacto inmutable que consume la siguiente, y las dependencias van en una sola dirección.

Mapa completo: [arquitectura del compilador](https://varn.twwk.cc/arquitectura/architecture/). Decisiones: [`docs/decisions/`](docs/decisions/) y [`rfcs/`](rfcs/) (ver RFC 0002 para `#[...]`/`#{...}` y RFC 0003 para macros `#nombre`).

## Rendimiento

Medido el 2026-11-13 con los benchmarks de [`tests/benchmarks/`](tests/benchmarks/README.md) (binario `release`, Windows x64, una corrida, contra Node y Bun): Varn es el más rápido en 14 de 23 cargas comparables en tiempo de proceso y en 15 de 23 en estado estable. Pierde con claridad en ordenar objetos con comparador (`sort_objects`), en llamadas por interfaz con asignación (`polymorphism`) y en generadores. La tabla completa y el método de medición están en el README de benchmarks.

## Documentación

| Para | Leer |
|---|---|
| Aprender el lenguaje | [guía del lenguaje](https://varn.twwk.cc/lenguaje/indice/) — cada rasgo con ejemplos verificados; gramática en [gramática](https://varn.twwk.cc/lenguaje/grammar/) |
| Usar el CLI | [referencia del CLI](https://varn.twwk.cc/guias/cli/), [inspección por fases](https://varn.twwk.cc/guias/inspeccion/) |
| La biblioteca estándar | [biblioteca estándar](https://varn.twwk.cc/lenguaje/standard-library/), [arquitectura de la std](https://varn.twwk.cc/arquitectura/stdlib-architecture/) |
| Entender el compilador y el runtime | [arquitectura](https://varn.twwk.cc/arquitectura/indice/) |
| Decisiones de diseño | [`docs/decisions/`](docs/decisions/) (ADRs), [`rfcs/`](rfcs/) |
| Qué viene | [roadmap](https://varn.twwk.cc/guias/roadmap/) |

`docs/dev/` (auditorías, notas, planes de trabajo) es registro interno, no especificación. Si un documento contradice al código, manda el código.

## Contribuir

Lee [`AGENTS.md`](AGENTS.md) (contrato de arquitectura) y [`CONTRIBUTING.md`](CONTRIBUTING.md). La prueba canónica es la suite `tests/main.vn` (más de 5600 casos), no `cargo test`:

```powershell
.\scripts\verify.ps1 -Quick   # iteración: build quick + un cuadrante
.\scripts\verify.ps1 -Fast    # antes de commitear: fmt + clippy + release + 5 cuadrantes
```

Los cuadrantes cruzan la procedencia de la std (`dev-checkout` / `@embedded`) con JIT / intérprete (`VN_NO_JIT=1`), más la verificación de la caché JIT.

## Licencia

Apache-2.0. Ver [`LICENSE`](LICENSE).
