# CONTRACT-V2: un contrato, un modelo, y la lógica como único coste

Si este documento contradice al código, manda el código (Ley 12). Toda cifra
se vuelve a comprobar antes de actuar.

## 0. Decisiones del dueño

- Contract-first: el `.vn` es la única verdad de la superficie nativa.
- Migración de golpe: se rompe lo que haga falta; los módulos se reescriben todos.
- Se hace antes de W2c de `FIXED-WIDTH.md`; los enteros de ancho fijo se declaran
  encima de este mecanismo.
- Criterio de diseño: añadir una función nativa no cuesta más líneas ni más
  ceremonia que su lógica.

## 1. Estado medido (`vn-macros`, `vn-builtins`)

- El mismo `.vn` se recorre tres veces: `build.rs` (`ContractSurface`), el macro
  (`contract_parse.rs`) y el checker en ejecución.
- `Mapped` (9 variantes) reduce los tipos del contrato; lo que no es escalar,
  `str` o `Array` baja a `VmValue`. En ~468 funciones nativas, ~229 tienen
  `VmValue` en su firma.
- Cada miembro genera dos envoltorios: lento (`&[VmValue]`) y rápido
  (`extern "C"`, solo escalares, `ArgType`, máx. 7 parámetros).
- Un estático adivina en ejecución si su primer argumento es el receptor
  (`is_static_receiver`: `null`, clase o módulo).
- Los ids nativos son la posición en `NATIVE_MODULES` más 15 fijados en `KnownOp`
  por nombre, y se hornean al emitir (`provider.native_op_id`).
- Cada función repite `_ctx: &mut dyn NativeCtx`, los tipos que ya dice el `.vn`
  y `Result<_, NativeError>`.

## 2. Superficie objetivo

El contrato no cambia de forma (`.vn`, con `@throws`, `@permission`). El Rust
solo da el nombre, los parámetros y la lógica; los tipos salen del contrato:

```rust
natives! { "runtime:math",
    abs(x)      => x.abs(),
    atan2(y, x) => y.atan2(x),
    random()    => rand::random(),
}

natives! { "core:types/int", class int,
    MAX_VALUE        => i64::MAX,
    toString(this)   => this.to_string(),
    abs(this)        => this.checked_abs().ok_or_else(|| overflow("abs", this)),
    read(cx, path)   => cx.fs().read(path)?,
}
```

- `cx` solo aparece si la función lo usa; `this` solo en métodos de instancia.
- El cuerpo devuelve el valor o un `Result`; el envoltorio lo normaliza según
  `@throws` del contrato.
- El tipo de cada parámetro es el del contrato (`f64`, `&str`, `Option<i64>`,
  `Vec<T>`, `Map`, instancia de clase); un `dynamic` es `VmValue`.
- Faltar un miembro del contrato o sobrar uno es error de compilación.

## 3. Items (un commit atómico cada uno, Ley 9)

- **C1. `vn-contract`: un modelo del contrato.** Crate que parsea un `.vn` a
  `ContractModel` con un lenguaje de tipos portable (`ContractTy`). Lo usan
  `build.rs` y el macro. Gana: tres recorridos pasan a uno. Borra:
  `ContractSurface` y `contract_parse.rs`.
- **C2. Marshalling por tipo del contrato.** `FromVm`/`IntoVm` cubren todo
  `ContractTy`; el macro deja de usar `Mapped`. Gana: firmas con `VmValue` crudo
  (medidas antes y después). Borra: `Mapped`.
- **C3. Sintaxis declarativa `natives!`.** Parámetros sin tipo, `cx`/`this`
  opcionales, retorno normalizado. Migra los 40 módulos. Borra `vn_contract!`.
- **C4. Un solo ABI con clase de registro.** El envoltorio rápido sale de la
  firma del contrato para cualquier firma de registros, con `SlotKind`. Borra:
  `ArgType`, el límite de 7 parámetros y la elegibilidad por lista.
- **C5. Convención de llamada por tipo de miembro.** Estático, instancia y
  función se distinguen al emitir. Borra: `is_static_receiver`.
- **C6. Ids densos y deterministas.** Ordinal en el contrato ordenado, sin hash
  en arranque, con un snapshot de ids. `@intrinsic(Variante)` sustituye a
  `KnownOp`. Borra: `KnownOp` y `op_key`.
- **C7. Clases nativas como descriptores estáticos.** Borra `add_method` imperativo.
- **C8. `vn_record!` y `vn_enum!` salen del modelo.**
- **Después:** W2c de `FIXED-WIDTH.md` sobre `natives!` (el contrato numérico es
  una plantilla sobre `Self` con un `impl` genérico de Rust).

Puerta por item: `tests/main.vn` en JIT, `VN_NO_JIT=1`, `VN_JIT_CACHE=verify`,
`VN_GC_STRESS=256 VN_GC_VERIFY=1`, `VN_STD=@embedded`, `error_corpus`,
`debug_golden`, clippy, rustfmt y `.\scripts\verify.ps1` completo antes de push.

## 4. Por comprobar antes de C5 y C6

- Por qué un estático necesita `is_static_receiver`: parece cubrir `Clase.m(x)`
  (receptor antepuesto) frente a `const f = Clase.m; f(x)` (sin receptor).
- Si algún caché en disco persiste los ids nativos.
- Coste de arranque del registro por cadenas, para medir la ganancia de C6.

## 5. Comparación (Ley 21)

WIT/Component Model: un IDL y un ABI canónico con bindings generados. .NET
`[LibraryImport]`: marshalling generado en compilación y rutas blittable y
marshalled separadas. Deno `#[op2]` y PyO3: la firma de Rust dirige el
marshalling por traits; son Rust-first, con tipos duplicados a mano en el otro
lado. Kotlin/Native `cinterop` y Dart `ffigen`: bindings generados desde la
declaración. Se adopta contract-first con marshalling por traits genéricos.
