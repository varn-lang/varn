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
- **W2. Checker — hecho** junto con la parte de W3 que da semántica. `i8 i16 i32
  u8 u16 u32 f32` son nombres de tipo (`NumKind::LANGUAGE`); los operadores
  exigen el mismo `NumKind` en ambos lados; un literal adopta un ancho solo si
  cabe; `-x` rechaza los sin signo; `as` es explícito entre cualquier par
  numérico. `u64` queda fuera de `LANGUAGE` hasta W3b.
- **W3. Semántica del backend — hecho para ≤32 bits y f32.** Un resultado
  aritmético o una conversión a un ancho estrecho pasa por `ValueConv::Narrow*`
  (comprueba el rango y lanza `IntegerOverflow`; `NarrowF32` redondea a 32 bits).
  Los valores viven normalizados en una palabra i64 o f64, así que el intérprete
  y el JIT comparten la semántica (el JIT llama al helper de conversión).
- **W3b. `u64`:** aritmética, comparación, división, desplazamiento y
  conversiones sin signo sobre 64 bits; el literal por encima de `i64::MAX`
  necesita un `IntLiteral` más ancho que `i64`. Entonces `u64` entra en
  `NumKind::LANGUAGE`.
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
