# PENDIENTES: todo lo que falta, con el estado exacto del corte

Corte: 2026-11-14, rama `main`, árbol limpio, 12 commits locales por encima de
`4a31037c`. **Nada se ha pusheado.** Si este documento contradice al código,
manda el código (Ley 12): toda cifra y afirmación se vuelve a comprobar antes de
actuar.

Este documento es el traspaso de tres planes que avanzaron a la vez. Los planes
de detalle siguen siendo `FIXED-WIDTH.md`, `CONTRACT-V2.md` y `TO-NATIVE.md`;
aquí está lo que quedó sin hacer, lo que se descubrió y cómo retomarlo.

---

## 0. Cómo retomar

### 0.1 Estado y commits de este corte (más reciente primero)

| Commit | Qué hizo |
|---|---|
| `e0d92c53` | W7a: clases con API para `i8 i16 i32 u8 u16 u32 u64 f32` (plantillas) |
| `4bf77565` | W3b: `u64` completo (int o bigint por encima de `i64::MAX`) |
| `9b347c8f` | W2c+W3a: los siete tipos estrechos son nombres de tipo, con overflow checked |
| `cf4fe443` | C2 parcial: arrays de escalares como `Vec<T>` tipado en los nativos |
| `3cc23f3c` | C3: `natives!` sustituye a `vn_contract!`; 39 módulos migrados |
| `44ab5442` | C1: crate `vn-contract`, un solo modelo del contrato |
| `6bcc137b` | docs: plan `CONTRACT-V2.md` |
| `a7b3caa3` | W2b: operadores y literales tipados por `NumKind` |
| `d4990bae` | W2a: `LangPrimitive::Num`, un id sembrado por `NumKind` |
| `2e3ec676` | docs: plan `FIXED-WIDTH.md` |
| `19a1c600` | W1: `NumKind` y `BackendTy::Num` |

### 0.2 Verificación que SÍ se corrió sobre `e0d92c53`

- `tests/main.vn` en JIT: 6140 pasan, 0 fallan.
- `tests/main.vn` con `VN_NO_JIT=1`: 6140 pasan, 0 fallan.
- `cargo test -p vn-cli --test error_corpus` y `--test debug_golden`: ok
  (goldens `*.symbols.golden.txt` regenerados con `UPDATE_DEBUG_GOLDENS=1`).
- `cargo clippy --workspace --all-targets --profile quick -- -D warnings`: limpio.
- `cargo fmt --all` aplicado.

### 0.3 Verificación que NO se corrió (obligatoria antes de cualquier push, Ley 19)

- `.\scripts\verify.ps1` completo (matriz de 4 cuadrantes: dev-checkout/embedded
  × JIT/sin JIT). Solo se corrió `-Quick` (cuadrante 1) en hitos intermedios.
- `.\scripts\verify.ps1 -Fast` (lint, release, camino de producción `@embedded`
  + JIT), exigido por la Ley 19 antes de push.
- `VN_JIT_CACHE=verify`, `VN_GC_STRESS=256 VN_GC_VERIFY=1`, `VN_STD=@embedded`
  sobre los cambios de W7a (clases nuevas), W3b (u64) y `natives!`.
- Los benchmarks (`xtask`, `vn_bench_file`): **ninguno** se midió. Todo cambio de
  rendimiento está sin medir; ver §3.W6 y §4.C4/C6.
- `cargo test --workspace` completo: solo se corrieron `vn-cli` (error_corpus,
  debug_golden), `vn-lsp`, `vn-core`, `vn-contract`, `vn-macros`, `vn-sem`,
  `vn-checker`. Faltan los demás crates (`vn-tir` y `vn-pipeline` se corrieron
  en W1, no tras W2–W7).

### 0.4 Comandos y trampas del entorno (Windows, Git Bash y PowerShell)

- Compilar el binario: `cargo build --profile quick --bin vn`; ejecutable en
  `target/quick/vn`. Probar: `./target/quick/vn run tests/main.vn` y
  `VN_NO_JIT=1 ./target/quick/vn run tests/main.vn`.
- `vn-builtins` solo compila sus módulos con la feature `runtime`: usar
  `cargo check -p vn-builtins --all-features`.
- `cargo check --workspace --all-targets --all-features` **falla** en
  `crates/vn-types/src/bin/sizeof.rs` (imports a `Expr`, `Stmt`, `Value` que ya
  no existen). Es preexistente y no es de este trabajo; no usar `--all-features`
  sobre el workspace.
- `cargo fmt --all` reformatea `crates/xtask/src/scaffold.rs` (estaba mal
  formateado antes). Tras cada `fmt`, mirar `git status` y decidir; el commit
  `3cc23f3c` ya incluye una versión de ese archivo.
- Regenerar goldens de `vn debug`: `UPDATE_DEBUG_GOLDENS=1 cargo test --profile
  quick -p vn-cli --test debug_golden`, y **revisar el diff** antes de commitear.
- `.\scripts\verify.ps1` escribe todo con `Write-Host`; para capturar solo el
  resumen: `$o = .\scripts\verify.ps1 -Quick *>&1 | Out-String` y filtrar
  `PASSED:|FAILED:|\[TODO CORRECTO|\[FAIL\]|drifted`.
- **Los archivos del repo tienen finales CRLF.** Una edición que busque texto
  multilínea con `\n` no coincide. Usar siempre un helper que detecte `\r\n` y
  convierta el patrón (el usado en esta sesión vivía en el scratchpad de la
  sesión, que puede desaparecer: es un `edit(path, [(viejo, nuevo), ...])` en
  Python que lee con `newline=''`, detecta `'\r\n' in s`, reemplaza `\n` por
  `\r\n` en ambos textos, exige que el viejo aparezca **una sola vez**, y
  escribe con `newline=''`).
- Los *heredocs* de Bash con apóstrofes o muchos escapes fallan con
  `unexpected EOF while looking for matching`: crear los archivos con la
  herramienta de escritura de archivos, no con `cat <<EOF`.
- Un literal `'\\'` pasado por Python desde un heredoc se rompe; construir el
  carácter con `chr(92)`.
- rust-analyzer marca `[macro-error] expected =>` en las clases con `$T` de
  `natives!`: es un falso positivo de diagnóstico (el macro real las acepta).
- Clippy aplica `wildcard_enum_match_arm` (Ley 7): ningún `_ =>` sobre enums del
  proyecto. Cada `match` nuevo enumera las variantes.

### 0.5 Reglas del dueño que gobiernan todo lo que sigue

- **Sin co-autor** en ningún commit (`~/.claude/AGENTS.md`): ni `Co-Authored-By`
  ni variante. Un hook lo rechaza.
- Un commit atómico por cambio (Ley 9), con la comparación C#/Python/Dart/Swift/
  Kotlin/Rust en el mensaje (Ley 21) y la nota de `cargo fmt` si aplica.
- Cero comentarios de código y cero tests unitarios locales (Ley 11): la
  validación es `tests/main.vn` y `tests/errors/*.vn` (fixtures
  `// expect: error[VNxxxx]`).
- Un test que valida comportamiento erróneo se reescribe (Ley 15); no se
  conserva por mantener verde.
- Una deficiencia es un item del plan: se implementa la capacidad que falta, no
  se rodea (memoria `feedback-deficiency-is-a-plan-item`).
- El tipo estático es el tipo de ejecución: nada de chequeos dinámicos ni tests
  que asuman que la representación diverge del tipo estático (memoria
  `feedback-static-type-is-runtime-type`).
- Cero tablas paralelas ni rutas duplicadas: lo viejo se borra en el mismo
  esfuerzo que lo reemplaza (Leyes 8 y 18).
- Antes de un push: `.\scripts\verify.ps1 -Fast` limpio (Ley 19).

---

## 1. Qué existe hoy (para no rehacerlo)

### 1.1 Tipos numéricos

- `vn_core::NumKind` / `IntKind` / `FloatKind` (`crates/vn-core/src/num_kind.rs`):
  tabla única con nombre, ancho, signo, rango (`min`/`max`/`contains`),
  `mantissa_bits`, `max_finite`, `round`, `range_text`, `core_module`,
  `from_name`, `ALL` (10 kinds), `INT`, `FLOAT`, `is_default_width`.
  `int` es `I64` y `float` es `F64`; no hay alias `i64`/`f64`.
- Los 7 anchos nuevos son nombres de tipo en el lenguaje: `i8 i16 i32 u8 u16 u32
  u64 f32` (`LangPrimitive::ALL` se deriva de `NumKind::ALL`).
- `BackendTy::Num(NumKind)` con constantes `BackendTy::INT` / `FLOAT`;
  `LangPrimitive::Num(NumKind)`; `NumericOperand::Num(NumKind)`;
  `CheckerTyId::num(kind)` / `num_kind()` (ids 0, 1 y 13–20);
  `Type::num(kind)`, `num_kind()`, `is_integer()`, `is_numeric()`.
- Reglas del checker: ambos operandos del mismo `NumKind`; un literal entero
  adopta un ancho solo si cabe (`IntKind::contains`); un literal flotante
  adopta `f32` si es finito en el rango de `f32`; `-x` rechaza los sin signo;
  `as` convierte explícitamente entre cualquier par numérico; el literal se
  adopta también a través de `T?` y uniones.
- Ejecución: un valor estrecho vive normalizado en una palabra i64 (o f64 para
  `f32`). El compilador envuelve cada resultado aritmético y cada conversión con
  `ValueConv::NarrowI8/I16/I32/U8/U16/U32/F32` (comprueba rango y lanza
  `IntegerOverflow`; `NarrowF32` redondea a 32 bits). `~` sobre sin signo es XOR
  con el máximo. Un desplazamiento que pierde bits es `IntegerOverflow`.
- `u64` = `int` no negativo o `bigint` por encima de `i64::MAX`. Aritmética y
  comparación por `bigint` (`U64ToBigInt`) con `NarrowU64` (rango [0, 2^64) y
  normalización); `& | ^ >> ~` por patrón de bits (`U64ToBits`, `U64FromBits`);
  `<<` multiplica por 2^n. Slot de registro `Dynamic`; campo `Boxed`.
- API de clase por ancho (plantillas): ver §1.3.

### 1.2 Mecanismo de contratos nativos

- Crate `vn-contract` (`parse`, `ModuleContract`, `ContractTy`, `Member`,
  `Kind`, `template::{kinds, instantiate, EXTENSION}`).
- Macro `natives!` en `crates/vn-macros/src/natives/` (`input.rs`, `mod.rs`,
  `member.rs`, `fast.rs`, `mapping.rs`). Sintaxis: `nombre(params) => cuerpo,`;
  el módulo sale del directorio (`core/*` → `globals`, `runtime/<dir>` →
  `runtime:<dir>`); el contrato es el único `.vn` (o `.tpl`) del directorio; el
  contexto es el primer nombre cuando sobra uno; `this` es el primero de un
  método de instancia; faltar o sobrar un miembro del contrato no compila (en
  secciones de plantilla, los miembros sobrantes se ignoran por instancia).
- Marshalling: `FromVm`/`IntoVm` para escalares, `i8..u64`, `f32`, `Vec<T>`;
  retornos de lista como `impl IntoVm`; `into_native_ret` normaliza valor o
  `Result`.
- `Mapped` (enum del macro) tiene `Int`, `Float`, `Narrow(NumKind)`, `Bool`,
  `Char`, `Str`, `StrRecv`, `Array`, `List(Box<Mapped>)`, `Dynamic`, `Void`,
  `Opt(Box<Mapped>)`.

### 1.3 Plantillas numéricas

- `crates/vn-builtins/src/modules/core/types/fixed_int/{fixed_int.tpl,
  fixed_int.rs}` y `.../fixed_float/{fixed_float.tpl, fixed_float.rs}`.
  Cabecera `// @kinds integer|float`; `$T` es el nombre del tipo; sufijo
  `// @signed` / `// @unsigned` filtra una línea. `build.rs` registra un módulo
  core por ancho (`core:types/u8`, …); `natives! { class $T { ... } }` expande los
  cuerpos una vez por ancho con `T` (tipo Rust) y `K` (`NumKind`) en un módulo
  privado `__vn_instance_<nombre>`.
- Las plantillas instancian solo los anchos **no por defecto** (`kinds()` filtra
  `is_default_width`). `int` y `float` siguen en archivos manuales.

---

## 2. Correcciones a lo afirmado antes (para no perder honestidad)

1. **El commit `3cc23f3c` dice "Declared @throws on module functions are now
   honoured". Es falso.** El adaptador `crates/vn-macros/src/contract_parse.rs`
   (`fn member`) pone `throws = Vec::new()` cuando `kind == Kind::Function`, así
   que el `@throws(TypeError, RangeError)` de `sendHttpResponse$` y
   `sendHttpHead$` (`runtime/net/net_runtime.vn:41-42`) sigue ignorándose. El
   modelo (`vn-contract`) sí los recoge. Arreglo: quitar esa rama del adaptador
   y verificar que la suite sigue pasando (los nativos de módulo que lancen una
   clase no declarada pasarán a fallar con `NativeError::within`).
2. La cifra "8037 → 7287 líneas Rust" cuenta todo `src/modules/**/*.rs`, no solo
   el código nativo. Es indicativa, no una medida del macro.
3. La cifra "63 de 500 miembros usan la ruta rápida" es de **elegibilidad según
   el contrato**, no de uso medido en el JIT.
4. "~229 funciones con `VmValue`" y "168 líneas que nombran `VmValue`" son
   conteos por `grep` sobre firmas, no equivalentes entre sí.

---

## 3. FIXED-WIDTH: lo que falta

### 3.1 W7b — migrar `int` y `float` a las plantillas (el siguiente paso)

Objetivo: borrar la ruta paralela (Ley 8). Pasos exactos:

1. `vn_contract::template::kinds` hoy excluye `is_default_width`: incluir
   también `I64` y `F64` (ids de módulo `core:types/int` y `core:types/float`).
2. Ampliar `fixed_int.tpl` con lo que tiene `int.vn` y la plantilla no:
   `static isSafeInteger(val: $T): bool`, `toLocaleString(): str`,
   `@throws(RangeError) toFixed(digits?: int): str`, `bitwiseNot(): $T`.
   `sign()` devuelve `int` en ambos. Ampliar `fixed_float.tpl` con lo de
   `float.vn` que no está (`toFixed` ya está; revisar `valueOf`, `toInt`).
3. Portar los cuerpos de `int.rs`/`float.rs` a `fixed_int.rs`/`fixed_float.rs`
   (los cuerpos de `int.rs` usan helpers `i64` de `vn_core`: `add_int`,
   `div_int`, `floor_div_int`, `ceil_div_int`, `rem_int`, `mod_int`, `neg_int`,
   `pow_int`; la plantilla ya implementa lo equivalente por `i128`; decidir si
   `vn_core::*_int` se queda como única definición (ADR-0023 lo exige) y la
   plantilla la llama, para no tener dos semánticas).
4. **Conservar** las funciones públicas de `float.rs` que se reexportan en
   `core/types/mod.rs`: `float_is_finite`, `float_is_nan`, `float_parse`
   (usadas fuera de la clase); moverlas, no borrarlas.
5. Borrar `core/types/int/{int.vn,int.rs}` y `core/types/float/{float.vn,
   float.rs}` y sus entradas en `core/types/mod.rs` y `modules/mod.rs`
   (`types::int::NATIVE_ENTRIES`, `types::float::NATIVE_ENTRIES`).
6. `vn_core::op_id::core_class_names` y `abi::WIDTH_CLASSES`/`NATIVE_GLOBALS`
   dejan de necesitar el filtro `!is_default_width` para evitar duplicar `int`
   y `float`: revisar que el orden de `NATIVE_GLOBALS` (índices horneados en el
   bytecode) no cambie para `int`/`float` (hoy están en posiciones fijas del
   array base; **cambiar posiciones cambia los índices `LoadNativeGlobalIdx`**).
7. `KnownOp::IntToString` (`vn-core/src/op_id.rs`) resuelve por nombre
   `("int", "toString")`: debe seguir resolviéndose.
8. Tests a vigilar: `123-int-api`, `121-float-ieee`, `53/102 int overflow`,
   `119 numeric errors`, `120 int division`, `122 numeric conversions`,
   `305 number conversions and formatting`, `358` y las de `Math`.
9. Después de borrar: `int` y `float` pasan a ser instancias; `Mapped::num`
   sigue mapeándolos a `Int`/`Float` (ruta rápida de funciones escalares).

### 3.2 W4 — layout compacto de campos

Hoy un campo `u8` ocupa 8 bytes (`ScalarRepr::I64`) porque
`BackendTy::exact_field_kind` usa `RuntimeKind::of_num(kind)` por clase.

- Reemplazar `ScalarRepr::{I64, F64}` por `ScalarRepr::Num(NumKind)`
  (`crates/vn-core/src/layout/mod.rs`; 37 referencias a `ScalarRepr::I64/F64` en
  `vn-core`, `vn-jit` (3 archivos), `vn-types`, `vn-pipeline`, `vn-debug`).
- `FieldKind { kind: RuntimeKind, nullable }` (byte empaquetado con
  `NULLABLE_BIT`, `FieldKind::from_u8`) no lleva el ancho: añadirlo
  (`FieldKind::num(kind)` o un campo) y propagarlo por `TypeLayout::of_field`,
  `of_kind`, `of_nullable`, el `FieldAccess` del IR (`GetFixedField`,
  `SetFixedField`), `ic_code()` y la caché de inline caches.
- Tamaños y alineaciones: 1/2/4/8 bytes; `f32` 4 bytes; el JIT debe cargar con
  extensión de signo/cero (`sload8/uload8/sload16/uload16/sload32/uload32`) y
  guardar truncando (`istore8/16/32`); el intérprete igual.
- `u64` no puede ser un campo de 8 bytes (puede ser un `bigint`): sigue
  `Boxed` (16 B) o se decide otra representación (§7, decisión D3).
- Nulables: `u8?`/`i8?` no tienen valor de nicho libre; `bool?` y `char?` ya usan
  nicho (N3 de TO-NATIVE). Decidir: 2 bytes (valor + marca) o `Boxed`.
- Actualizar `vn debug -p layout` y su golden.
- Medir antes y después con un benchmark de objetos con muchos campos estrechos.

### 3.3 W5 — arrays de ancho real

- `ArrayRepr` (`crates/vn-types/src/vm_value/array.rs`) tiene `Boxed`, `I64`,
  `F64`, `Char`, `Bool`, `Ref`. Añadir variantes por ancho (contenedor genérico
  sobre `NumKind`, no una variante `Vec<i8>` suelta por tipo) y `Elem::Num` en
  el JIT (`crates/vn-jit/src/clif/from_ssa/arrays.rs`, 10 usos de `Elem::*`).
- **Bloqueo N5** (TO-NATIVE): la representación del array la elige hoy
  `ArrayRepr::unboxed()` mirando los valores en ejecución y `migrate_to_boxed`
  la cambia; un `[]` vacío no sabe su tipo de elemento. Para que `u8[]` sea
  `Vec<u8>` la instrucción que crea el array debe llevar la representación del
  tipo estático y no cambiarla (borrando `unboxed`, `push_repr_change`,
  `migrate_to_boxed`). Con genéricos borrados, un `T[]` creado en código
  genérico no conoce `T`: exige monomorfizar o pasar la representación de `T`.
  **Es decisión del dueño** (§7, D1).
- `Bytes` y `u8[]` tienen la misma forma: unificar la representación evita
  copiar al pasar entre ambos.
- Operaciones vectorizadas (`sum`, `fill`, `copy`, `indexOf`, comparaciones,
  `memchr`, `memcmp`) sobre arrays de ancho real con SIMD de Rust; es el ítem 2
  de la auditoría original y depende de W5.

### 3.4 W6 — rendimiento del JIT

Hoy cada `Narrow*` y cada operación estrecha pasa por el helper de conversión
(`convert_boxed` → `ctx.helpers.convert`), correcto pero lento.

- `crates/vn-jit/src/clif/from_ssa/numeric.rs` (`is_inline_convert`,
  `emit_convert`, `Expected::of`): emitir `icmp` + `brif` + `guard_overflow` en
  línea para `NarrowI8..NarrowU32`, y `fdemote`/`fpromote` para `NarrowF32`.
- Operaciones de 8/16/32 bits nativas en Cranelift (`iadd` con `sadd_overflow`
  /`uadd_overflow` a ancho nativo) en vez de i64 + comprobación.
- `crates/vn-jit/src/clif/from_ssa/ranges.rs` (análisis de rangos, `no_overflow`,
  `fast_bounds`): que conozca los rangos de los anchos para eliminar chequeos
  (p. ej. `u8 & 0xFF`, índices acotados).
- Pasadas del compilador con `Convert` que puede fallar (`ValueConv::can_fault`
  incluye `Narrow*` enteros y `U64ToBits`, no `NarrowF32` ni `U64ToBigInt` ni
  `U64FromBits`): revisar `passes/const_fold.rs` (plegar `Convert Narrow` sobre
  constantes y detectar el overflow en compilación), `passes/dce` (pureza),
  `passes/cse.rs`, `passes/licm.rs`, `passes/algebraic.rs`.
- Instrucciones de CPU para los métodos nuevos: `countOnes`→`popcnt`,
  `leadingZeros`→`clz`, `trailingZeros`→`ctz`, `rotateLeft/Right`→`rotl/rotr`,
  `swapBytes`→`bswap`. Hoy son llamadas nativas (`core_method_ops`). Mapearlas
  como intrínsecos del JIT (patrón de `MathOp`/`emit_intrinsic`).
- Ruta rápida de los métodos nativos de `int`/`float`/`bool`/`char`: 38 métodos
  escalares podrían usar la ruta `extern "C"` (ver §4.C4); medir primero.
- ISA/CPU: `cranelift_native::builder_with_options(true)` ya infiere las
  características del host; para el AOT conviene un `target-cpu` explícito
  (baseline x86-64-v2/v3 con detección en ejecución). Sin hacer.
- `preserve_frame_pointers` siempre activo (lo exigen los stack roots del GC):
  medir su coste antes de tocarlo.
- Medir con `vn_bench_file`/`xtask` antes y después; no hay ninguna medida aún.

### 3.5 W7 — resto de `std`, `Bytes` y descriptores

- **`TypeDesc` no se migró** (`crates/vn-core/src/type_desc.rs`; 27 referencias a
  `TypeDesc::Int/Float` en `vn-core`, `vn-compiler/from_tir/describe.rs` (6),
  `vn-vm/src/exec/decode.rs` (7), `conform.rs` (4), `ctx_json/typed.rs` (2),
  `contract.rs` (2)). Es la causa de dos fallos reales (§5.1 y §5.2). Pasar a
  `TypeDesc::Num(NumKind)` y que decode/conform comprueben el rango (y que
  `u64` acepte `bigint`).
- `FfiKind` (`vn-core/src/ffi_kind.rs`) y `Slot` de `runtime/ffi/call.rs`: ver si
  deben conocer los anchos (llamadas C con `u8`/`i32`/`f32`).
- `Bytes` sobre `u8[]` y APIs de `std` donde un ancho estrecho es natural
  (puertos `u16`, modos de archivo `u32`, bytes de criptografía y red).
- JSON: `Json.stringify` de campos estrechos funciona (`{"v":3}`); la decodificación
  no valida el rango (§5.2).
- Documentación de usuario: no existe ninguna (`docs/lang/*`, README, CHANGELOG,
  hover del LSP). Falta un ADR que **revierta ADR-0015 D11** (los anchos
  estrechos vuelven como optimización probada) y recoja las decisiones de
  W1–W7 (overflow siempre error, `as` explícito, `u64` como int|bigint).
- LSP: completado y hover de las clases nuevas, tokens semánticos para
  `u8`…`f32` (`LangPrimitive::ALL` ya los lista; comprobar que las pruebas del
  LSP y los snapshots no se rompen, no se corrieron todas).

---

## 4. CONTRACT-V2: lo que falta

### 4.1 C2 — marshalling por tipo del contrato (resto)

Hecho: listas de escalares. Falta tipar (conteo por posición de tipo en los
contratos actuales, sobre 511 miembros): `Bytes` (40), `TaskHandle<T>` (35),
genéricos `T`/`U`/`K`/`E` (~30), uniones (16), funciones `fn` (14), `dynamic`
(29), `decimal` (9), `Error` (6), `Map`/`Set`/`Range`/`Result`/`Task`, clases
(`SqlParam`, `ProcessOutput`), `array<tuple>` (3), `array<dynamic>` (6),
`opt<named>` (5). Objetivo: que el cuerpo reciba un tipo de Rust y no `VmValue`
crudo. Después borrar `Mapped` por completo (hoy lo usa también `vn_record.rs`,
`contract_records.rs`, `vn_enum.rs`).

### 4.2 C3 — pendiente dentro de `natives!`

- **Honrar `@throws` de funciones** (§2.1).
- Flexibilidad de retorno: el cuerpo devuelve valor o `Result`; cuando el tipo
  del cuerpo es ambiguo hace falta turbofish (`rand::random::<f64>()`,
  `.collect::<Vec<_>>()`, `.collect::<String>()`); hubo 12 sitios. Un cuerpo
  que termina en `!` (`std::process::exit`) no compila: se envolvió en
  `fn terminate(code: i32)` en `runtime/sys/sys.rs`.
- Falta una forma de que `natives!` verifique el contrato en pruebas
  automáticas (hoy el rechazo de un miembro faltante/sobrante se comprobó a
  mano; no hay fixture porque es un error de compilación de Rust).
- Documentar `natives!` (sintaxis, `cx`, `this`, plantillas con `$T`, `T`, `K`).
- `cargo run -p xtask -- std-scaffold` ya genera el estilo nuevo (`ping() =>
  "pong".to_string()`); falta que genere el registro en `modules/mod.rs` y
  `core/types/mod.rs`, que sigue siendo manual (cada módulo nuevo toca tres
  sitios).

### 4.3 C4 — un solo ABI con clase de registro (sin empezar)

- Medido: de 500 miembros, 63 son elegibles para la ruta rápida (`extern "C"`,
  escalares, función o estático, sin `@throws`, ≤7 parámetros); 38 más son
  métodos escalares de `int`/`float`/`bool`/`char`; 75 más con `str` solo en
  parámetros o retorno.
- `fast.rs` ya tiene una rama para `Kind::Method` desactivada por
  `is_fast_eligible` (`mapping.rs`); la firma `SignatureDescriptor`/`ArgType`
  (8 valores, 7 parámetros) y el consumidor
  (`vn-vm/src/jit/table_build.rs:65`, `NativeOpTarget`) habría que generalizarlos
  a la clase de registro (`SlotKind`) y borrar `ArgType`.
- **No tocar el JIT sin medir antes** que algún bucle caliente llama a esos
  métodos.
- Los métodos de anchos estrechos y `u64` (receptor `Narrow`) no son elegibles.

### 4.4 C5 — convención de llamada por tipo de miembro

- Borrar `is_static_receiver` (`vn-vm/src/heap/access.rs:258`, `native_ctx.rs`,
  `exec/host/mod.rs`, `heap/native.rs`). Hoy un estático decide en ejecución si
  su primer argumento es el receptor (`null`, una `Class` o un `Module`).
  Hipótesis (sin comprobar): cubre `Clase.m(x)` (receptor antepuesto) frente a
  `const f = Clase.m; f(x)` (sin receptor). El compilador sabe cuál es: debe
  emitir `CallStatic` frente a `CallMethod` y el wrapper dejar de adivinar
  (viola la Ley 17, y hay riesgo si el primer argumento real es `null`).
- Código afectado: `member.rs` (`arg_base_is_dynamic`, el bloque `decode`).

### 4.5 C6 — ids deterministas y registro

- Hoy `registry()` (`vn-builtins/src/dispatch/mod.rs`) asigna el id de cada
  operación nativa por posición de registro (`by_id.len()`) en el orden de
  `NATIVE_MODULES`, con 15 excepciones fijadas en `KnownOp` por nombre; añadir o
  reordenar un módulo desplaza ids horneados al emitir (`provider.native_op_id`
  en `vn-emit`). **No se verificó si algún caché en disco persiste esos ids**
  (`VN_JIT_CACHE`, bytecode cacheado): comprobarlo antes de cambiarlos.
- Plan: ids densos por orden ordinal en el contrato ordenado, tabla estática
  generada, sin `FxHashMap<String,u64>` ni `op_key` (concatena
  `módulo\1ns\1símbolo`) en arranque; `@intrinsic(Variante)` en el contrato con
  un enum tipado de `vn-core` sustituye a `KnownOp` (Ley 16).
- Medir antes el coste de arranque del registro (nunca se midió).
- `native_modules!` (`modules/mod.rs`) y `core/types/mod.rs` siguen listando a
  mano cada módulo.
- `CORE_CLASSES` (14 `RuntimeKind`) + `core_class_names()` ya añade los anchos;
  tras W7b se simplifica.

### 4.6 C7 y C8

- C7: clases nativas como descriptores estáticos (`ClassDef` construida hoy
  llamando a `cls.add_method(nombre, ctx.alloc_fn(...))` por miembro en
  ejecución). Borra el `add_method` imperativo.
- C8: `vn_record!` y `vn_enum!` (19 usos) siguen leyendo el `.vn` con su propio
  recorrido en `vn-macros/src/contract_records.rs` (usa `vn_lexer`, `vn_parser`,
  `classify`, `unwrap_export` de `contract_parse.rs`): derivarlos del modelo
  `vn-contract` y quitar esas dependencias de `vn-macros`.
- `vn-macros/Cargo.toml` aún depende de `vn-lexer` y `vn-parser` por C8.

---

## 5. Fallos reales encontrados y NO corregidos (con reproducción)

Todos reproducidos con `target/quick/vn run` sobre `e0d92c53`.

1. **Un `u64` con valor `bigint` no entra en un parámetro tipado `u64` por una
   frontera dinámica.**
   ```
   const big: u64 = 9223372036854775808n as u64
   const list: u64[] = [big]
   print(list.map((v: u64) => v)[0])
   // TypeError: type mismatch: argument 1 expected 'int', got 'bigint'
   ```
   También rompe `v.sort((a, b) => ...)` sobre un `u64[]` con `bigint`. Causa
   probable: los descriptores de parámetro usan `TypeDesc::Int`
   (`describe.rs:62`) y las guardas `ExpectInt`
   (`checked_values.rs`, `functions.rs:147`, `lower_checked_*`) aceptan solo
   `int`. Para `u64` deben aceptar `int` o `bigint` (`NarrowU64`); para los
   estrechos deben comprobar el rango (`NarrowX`).
2. **La decodificación no valida el rango de un ancho estrecho.**
   ```
   import { Json } from "std:encoding"
   interface C { v: u8 }
   print(Json.parse<C>("{\"v\": 300}"))   // { v: 300 }  (debería fallar)
   ```
   Misma causa (`TypeDesc::Int` a nivel de clase). Probar también
   `conform`/`as` de contenedores (`u8[]`, `Map<u8, str>`).
3. **`x is u8` da `false` y `x is int` da `true` para `const x: u8 = 3`.** El
   test de tipo se resuelve por el tipo de ejecución (`KIND_INT`), que no
   distingue anchos. Con tipo estático conocido debe plegarse (la regla de
   `::type` ya pliega, commit `7434a2ae`); con `dynamic` el ancho no existe en
   ejecución y `x is u8` solo puede significar "es un entero que cabe" o debe
   rechazarse (decisión D2).
4. **`x::type` da `"int"` para un `u8`.** Debería ser `"u8"` cuando el tipo
   estático lo fija.
5. **El índice de un array con un ancho estrecho se rechaza**: `arr[i]` con
   `i: u8` da `type mismatch: index key is 'u8', expected 'int'`. Es exacto
   (`Type::is_int` en `checker_expressions/infer/compound.rs:93-136`). Decidir
   si se acepta cualquier entero (convirtiendo) o se exige `as int` (decisión D4).
6. **Literal por encima de `i64::MAX`**: `const k: u64 = 18446744073709551615`
   da `error[VN2007]: integer literal overflows i64`. Hoy se escribe
   `18446744073709551615n as u64`. Arreglo de raíz: `IntLiteral.value` es `i64`
   en el AST (`vn-core/src/ast/expr.rs`) y el lexer lo rechaza
   (`ParsedNumber`); ampliarlo a `i128`/`u64` o emitir `BigIntLiteral` y adoptar
   a `u64`. Aplica también a `0xFFFF_FFFF_FFFF_FFFF`.
7. **`f32` se imprime ensanchado**: `print(0.1f32)`, `.toString()` y `${f}` dan
   `0.10000000149011612`; el más corto que recupera el `f32` sería `0.1`.
   Requiere formateo por `FloatKind` en `vn_core::float_to_string` (una sola
   definición) y que `print`/plantillas conozcan el ancho estático.
8. **Mensajes engañosos**: `const x: u8 = 300` dice "declared as 'u8' but
   initialised with 'int'" y `a < 300` con `a: u8` dice "invalid binary
   operation '<' between 'u8' and 'int'". Deben decir que `300` no cabe en `u8`
   (rango `0..=255`).
9. **`float as u64`** pasa por `i64` (`FloatToInt`): un flotante entre 2^63 y
   2^64 desborda aunque cabría en `u64`. `lower_cast` ya usa `FloatToBigInt`
   para `u64`; comprobar el caso y que `3.9 as u64` siga siendo 3.
10. **Rendimiento de `u64`**: cada operación asigna `bigint` (`U64ToBigInt`,
    aritmética `bigint`, `NarrowU64`). Correcto pero lento; un bucle de hash
    FNV con `wrappingMul` va por nativo. Medir y, si hace falta, una ruta
    nativa sin asignar cuando ambos operandos son `int` no negativos.
11. **`u64` en estructuras de datos**: verificar `Map<u64, V>` y `Set<u64>`
    (hash e igualdad de `int` frente a `bigint`: la forma normalizada garantiza
    igualdad de representación, pero no hay test), `sort` sin comparador,
    `JSON` de un `u64` ≥ 2^63, `Array<u64>` con migración de representación.
12. **El desplazamiento de `int` envuelve bits sin error** (`1 << 63` es
    `i64::MIN`, `3 << 62` es negativo) mientras que en anchos estrechos y `u64`
    un desplazamiento que pierde bits es `IntegerOverflow`. Es una asimetría de
    diseño: decidir si `int` pasa a ser estricto o si los estrechos envuelven
    (D5).
13. **`-x` sobre `i8` con `x == -128`**, `x / -1` y otros extremos están
    cubiertos; faltan tests de `++`/`--` sobre ancho estrecho (se comprobó a mano
    que `x++` en 255 desborda), de `x ** n` y de rangos (`for` sobre `u8`).
14. Los mensajes de error de overflow de narrow dicen `256 does not fit u8` pero
    no el nombre de la operación; los nativos dicen `integer overflow: abs(-128)
    is outside i8 (-128..=127)`. Unificar.

---

## 6. Qué NO está probado (huecos de cobertura)

- Anchos estrechos dentro de: genéricos con restricciones, uniones
  (`u8 | str`), `match` con patrones de rango, enums con base entera, tuplas,
  `Record`/`interface` con campos estrechos y valores por defecto, `with`,
  destructuring con tipos estrechos (`[u8, u8]` en una anotación da `unexpected
  [ in a type`: esa sintaxis de tupla no existe en Varn, no es un fallo), cierres
  que capturan, async/generadores, isolates (`SendValue`: los estrechos viajan
  como `int`), FFI, `Decimal`/`BigInt` ↔ estrechos (`x as bigint`, `decimal`).
- `f32`: `NaN`, `Infinity`, redondeo de `/` y `%`, `pow`, comparaciones, `round`,
  conversión a `int` con desborde, `toFixed`.
- Conversión `as` entre todos los pares (hay 10×10) y desde `char`/`bool`/`str`.
- Los métodos nuevos solo se probaron con una muestra (`wrapping`, `saturating`,
  `checked`, `div` familia, `rotate`, `swapBytes`, `toHex/Binary`, `parse`,
  `tryParse`); faltan `clamp`, `pow` con límites, `isEven/isOdd`, `sign`,
  `negate`, `min/max`, `toOctal`, `leadingZeros/trailingZeros` de otros anchos
  y todos los de `f32`.
- Estabilidad con `VN_GC_STRESS=256 VN_GC_VERIFY=1` y `VN_JIT_CACHE=verify`
  con un `u64` ≥ 2^63 vivo (el `bigint` es un objeto del heap).
- `vn debug` (fases `tir`, `ssa`, `bytecode`, `clif`) sobre programas con
  `ValueConv::Narrow*`: los goldens existentes solo cubren `int`/`float`.
- El `-Quick` no ejecuta los cuadrantes embebidos: las plantillas se
  incrustan en `registry.generated.rs` y el modo `@embedded` debe probarse.

---

## 7. Decisiones abiertas del dueño (bloquean o condicionan trabajo)

- **D1. N5/N2/N6 de TO-NATIVE (genéricos borrados).** Bloquea arrays de ancho
  real, enums inmediatos y tuplas compactas. N2 (variante sin datos como
  inmediato) tiene dos diseños (A: identidad en la palabra de etiqueta; B:
  modelo C#/Swift, enum sin datos como su etiqueta en registro y campo de 8 B).
  N5: monomorfizar, pasar la representación de `T` como metadato (Swift) o
  genéricos reificados (C#). N6 depende de fijar el orden canónico de campos
  de los records.
- **D2. Frontera dinámica.** Regla acordada: un valor estrecho que entra en
  `dynamic` o en `T` genérico borrado entra **como `int`** (pierde el ancho) y
  vuelve con `as`. Falta decidir qué significa `x is u8` sobre un `dynamic`
  (§5.3) y cómo se comporta `u64` ≥ 2^63 al cruzar (hoy cruza como `bigint`).
- **D3. Representación del campo/array `u64`** (Boxed 16 B hoy). Alternativa:
  dos palabras, o restringir `u64` a registros/boxed y no compactarlo.
- **D4. Índice con ancho estrecho** (`arr[u8]`): aceptar o exigir `as int`.
- **D5. Desplazamientos**: ¿`int` estricto o estrechos envolventes?
- **D6. `i128`/`u128` y `isize`/`usize`**: Rust los tiene y Cranelift soporta
  `i128`; se dejaron fuera por no haberlos pedido.
- **D7. Literales de `u64`** por encima de `i64::MAX` (§5.6): ampliar el AST o
  añadir sufijo.
- **D8. Si `f32` se imprime como `f32`** (más corto) o ensanchado (§5.7).
- **D9. Orden de trabajo** entre W7b/W4/W5/W6 y C2–C8 (§8).

---

## 8. Orden recomendado

1. Corregir los fallos reales de §5.1, §5.2, §5.3/5.4 y §2.1 (soundness de las
   fronteras): `TypeDesc::Num`, guardas `Expect*` por ancho, `x is`, `::type`,
   `@throws` de funciones. Un commit por causa.
2. W7b (migrar `int` y `float` a plantillas): borra la ruta paralela.
3. Ejecutar la matriz completa `.\scripts\verify.ps1` y `-Fast`; solo entonces
   considerar un push.
4. Medir línea base (benchmarks de aritmética estrecha, objetos con campos
   estrechos, arrays) **antes** de W4/W5/W6.
5. W4 (campos) y W5 (arrays), previa decisión D1/D3. Después W6 (JIT en línea,
   rangos, intrínsecos de CPU).
6. C5 → C6 → C7 → C8, y C2 (resto) y C4 solo con medición previa.
7. ADR de reversión de ADR-0015 D11 y documentación de usuario.

---

## 9. Mapa de archivos clave

| Tema | Archivos |
|---|---|
| Tabla de numéricos | `crates/vn-core/src/num_kind.rs`, `lang_type.rs`, `numeric_conv.rs` (`ValueConv`), `numeric.rs` (`NumericOperand`), `abi.rs` (`NATIVE_GLOBALS`), `op_id.rs` |
| Checker numérico | `crates/vn-sem/src/types/{numeric_ops,numeric_literal,type_impl}.rs`, `types/interned/{ids,table}.rs`, `compat/{scalar,expr_sat}.rs`; `crates/vn-checker/src/checker_expressions/check/{binary_ops,ops,closures,operator_capability}.rs`, `checker/literal_fit.rs`, `checker_call_types/call_binary.rs` |
| Emisión y TIR | `crates/vn-emit/src/body/{expr_node,expr_convert,receiver_types}.rs`, `module_globals.rs`, `ty/lower.rs`; `crates/vn-tir/src/ty.rs` |
| Compilador (estrechos y u64) | `crates/vn-compiler/src/from_tir/build/{expr_arith,checked_values,functions,ops}.rs`, `describe.rs`, `ssa/verify/type_rules.rs`, `ssa/emit/register_meta.rs` |
| VM | `crates/vn-vm/src/exec/convert.rs` (`narrow`, `narrow_u64`, `u64_to_bigint`, `u64_to_bits`, `u64_from_bits`) |
| JIT | `crates/vn-jit/src/clif/from_ssa/numeric.rs`, `ranges.rs`, `arrays.rs`, `clif/emit/payload.rs` (`guard_overflow`) |
| Contratos nativos | `crates/vn-contract/src/*`, `crates/vn-macros/src/natives/*`, `contract_parse.rs`, `crates/vn-builtins/build.rs`, `src/modules/**`, `crates/vn-types/src/marshal.rs` |
| Plantillas | `crates/vn-builtins/src/modules/core/types/{fixed_int,fixed_float}/` |
| Tests | `tests/fixed-width-numbers.test.vn`; fixtures `tests/errors/{fixed-width-*,unsigned-has-no-negation,f32-literal-out-of-range,unknown-width-name-rejected,unknown-type-cast}.vn` |
| Planes | `docs/dev/plans/{FIXED-WIDTH,CONTRACT-V2,TO-NATIVE,PENDIENTES}.md` |

Los bugs probados por sondeo se reprodujeron con programas sueltos; ninguno
está aún como fixture. Al corregir cada uno de §5, añadir su caso a
`tests/fixed-width-numbers.test.vn` o a `tests/errors/` (debe fallar antes del
cambio).
