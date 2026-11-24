# Varn — Núcleo y plataforma (índice)

Documento histórico de 3000 líneas reducido a índice. Era propuesta arquitectónica,
no spec vigente. La mayoría de sus §§ (StringView, Span, intersecciones, literales
como tipos, Unit, Type\<T\>, unsafe, SIMD, WASM, FFI, capas std:*) no aparece en
`tests/*.vn` (cero matches de `StringView`, `Span<`, `struct`, `unsafe` en tests/).
Lo no citado abajo es NO-VERIFICADO.

Verificado (contrato real, detalle en `types.md` / `specification.md`):

- 4 tipos numéricos, sin anchos físicos públicos: `int, float, bigint, decimal`.
- `int` = entero 64 bits con signo; `+ - * / % **` lanzan `IntegerOverflow`
  fuera de rango (`tests/int-division.test.vn`, `crates/vn-core/src/numeric_conv.rs`).
- `int / int` = `int` truncado hacia cero; `x / 0` lanza `DivisionByZero`
  (`tests/int-division.test.vn`).
- Conversiones explícitas con `as` (`tests/numeric-conversions.test.vn`).
- `T?` = `T | null` (`tests/nullable-types.test.vn`); uniones `A | B`
  (`tests/unions.test.vn`); enums simples, con payload, genéricos
  (`tests/enums.test.vn`, `tests/advanced-enums.test.vn`).
- `Array<T>` ≡ `T[]`; tuplas `(…)` y structs inmutables con `==`
  estructural (`tests/tuples-structs.test.vn`).
- `dynamic` como frontera explícita (`tests/control-flow-narrowing.test.vn`,
  `tests/numeric-conversions.test.vn`).

Contratos entre fases y runtime: `../COMPILER_ARCHITECTURE.md`,
`../RUNTIME_ARCHITECTURE.md`, `../VM_ARCHITECTURE.md`, `../MODULES_SYSTEM.md`.
