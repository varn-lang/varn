# FIXED-WIDTH: tipos numéricos de ancho fijo, secundarios a `int` y `float`

Si este documento contradice al código, manda el código (Ley 12). Toda cifra
se vuelve a comprobar antes de actuar.

- Contexto: ADR-0015 borró `i8 i16 i32 u8 u16 u32 u64 f32` y dejó D11: el
  ancho angosto vuelve solo como optimización probada. Este plan es esa vuelta.
- Principio: el tipo estático decide el ancho; ninguna fase lo re-deriva (Ley 6).
  `int` (i64) y `float` (f64) siguen siendo los tipos por defecto; los anchos
  fijos son para quien sabe lo que hace.

## 0. Decisiones del dueño

- Tipos: `i8 i16 i32 u8 u16 u32 u64 f32`. Sin alias `i64`: `int` es el i64 (ADR-0009).
- Overflow: `IntegerOverflow` siempre, sin diferencia debug/release (ADR-0023).
  `wrapping*`, `saturating*` y `checked*` explícitos (ADR-0015 D13).
- Conversión: siempre `as`. Ninguna implícita, tampoco ensanchar. Estrechar
  comprueba rango y lanza `IntegerOverflow` (como `float as int`, D3).
- Literales: adoptan el tipo del contexto si caben (D6). Sin sufijos.

## 1. Cimiento: una tabla canónica en `vn-core`

Hoy hay 19 enums paralelos que repiten entero/flotante (`BackendTy`, `SlotKind`,
`ScalarRepr`, `ArrayRepr`, `Elem`, `RuntimeKind`, `NumericDomain`, `ValueConv`,
`LangPrimitive`, `TypeDesc`, `FfiKind`, `Literal`, `ArgType`, `SendValue`, ...).
Añadir un ancho tocaría cientos de sitios.

```
IntKind   { I8, I16, I32, I64, U8, U16, U32, U64 }
FloatKind { F32, F64 }
NumKind   { Int(IntKind), Float(FloatKind) }
```

`NumKind` es la única fuente de nombre, ancho, signo, rango, dominio y código de
IC. Los demás enums llevan `Num(NumKind)` en lugar de `Int` y `Float`, y los
consumidores consultan métodos en vez de enumerar anchos (Ley 16).

## 2. Representación

- Registro: entero de hasta 64 bits normalizado en una palabra i64; `u64` guarda
  los bits y la operación lleva el signo. `SlotKind` sigue siendo `Int`/`Float`.
- `f32`: aritmética nativa de 32 bits.
- Campo y array: ancho real.
- Overflow: ≤32 bits se calcula en i64 y se comprueba el rango; 64 bits usa el
  flag del procesador; el análisis de rangos elimina el chequeo si lo prueba.

## 3. Items (un commit atómico cada uno, Ley 9)

- **W1. `NumKind` y `BackendTy::Num(NumKind)`.** Hecho para `BackendTy`: sin
  cambio de comportamiento (suite idéntica, 5716 casos). `RuntimeKind`, `SlotKind`,
  `KIND_*` y `SendValue` son clase de valor en ejecución y no cambian; el ancho de
  un campo lo llevará `FieldKind` (W4). `LangPrimitive` y `TypeDesc` se migran en W2.
  Los sitios que distinguen entero de flotante por clase usan `Num(NumKind::Int(_))`
  y `Num(NumKind::Float(_))`: son la lista de trabajo de W3 (cada uno decide su ancho).
  Borra: `BackendTy::Int` y `BackendTy::Float`.
- **W2. Lexer, parser y checker.** Tipos, literales, `as`, reglas de overflow.
  El backend aún no soportado falla con un diagnóstico explícito.
- **W3. SSA y VM.** Operaciones tipadas con ancho; registro normalizado.
- **W4. Layout de campos** (`ScalarRepr::Num`).
- **W5. `ArrayRepr` de ancho real** y `Elem::Num` en el JIT.
- **W6. JIT.** Operaciones nativas de 8, 16 y 32 bits, `u64`, `f32`; eliminación
  de chequeos por rangos.
- **W7. `std`.** `wrapping*`, `saturating*`, `checked*`, conversiones, `Bytes`
  sobre `u8[]`.

Puerta por item: `tests/main.vn` en JIT, `VN_NO_JIT=1`, `VN_JIT_CACHE=verify`,
`VN_GC_STRESS=256 VN_GC_VERIFY=1`, `VN_STD=@embedded`, `error_corpus`,
`debug_golden`, clippy, rustfmt y `.\scripts\verify.ps1` completo desde W3.

## 4. Comparación (Ley 21)

Rust: ancho fijo y `as`, pero wrapping en release; adoptamos nombres y `as`
explícito, no el wrapping. Swift: trap siempre; es nuestra política. C#:
`checked` opcional y ensanche implícito; no. Kotlin: confirma que un tipo
secundario con ergonomía propia es viable. Dart y Python: sin anchos en el
lenguaje; Dart los reserva a `typed_data`, apoyo a que sean para especialistas.
