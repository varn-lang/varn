# ADR-0035: Una sola SSA para el compilador y el JIT

## Estado
Propuesta (2026-11-23).

## Contexto
El compilador optimiza sobre `vn_compiler::ssa::ir::InstKind` (89 variantes) y el JIT
consume `vn_types::ssa::SsaOp` (91 variantes), que viaja serializado en el artefacto.
`ssa/portable/` (unas 2000 lineas) traduce una a otra. Medido sobre el codigo
(2026-11-23), de los 81 brazos de la proyeccion:

- 55 son copia campo a campo: solo cambian el tipo del id de valor (`Value` frente a
  `u32`) y el de las cadenas (`Arc<str>` frente a `Box<str>`).
- 26 transforman de verdad, y caen en dos grupos:
  - Representacion: `ConstStr`, `ConstBigInt`, `ConstDecimal`, `Build*`.
  - Anotaciones de backend que solo existen despues de la asignacion de registros y
    la emision de bytecode: ranura de cache inline (`GetProperty`, `SetProperty`,
    `MethodCall`, `InvokeSlot`, `BuildObjectSpread`), registros capturados
    (`MakeClosure`, `LoadCaptured`, `StoreCaptured`, `CallDirect`, `CloseUpvalues`),
    `ip` de cada instruccion, y la eleccion del operador por tipo (`Binary`, `Unary`,
    via `lower::binary_opcode`).

Ademas se duplican, con la misma forma, `Terminator`/`SsaTerm` y el recorrido de
operandos (`vn-compiler/src/ssa/uses.rs`, 264 lineas, y `vn-types/src/ssa/uses.rs`).

La frontera de fase es legitima (Ley 0): el JIT no puede depender del compilador y
recibe un artefacto inmutable. Lo que no es legitimo es que la forma de cada
instruccion se declare dos veces (Leyes 6 y 8).

## Decision
Un unico conjunto de instrucciones en `vn-types`, que el compilador usa durante sus
pases y el JIT lee del artefacto, mas una tabla de anotaciones que produce el
backend. La proyeccion desaparece.

Fases, cada una con su puerta (suite completa y matriz de `verify.ps1` en verde, y
sin regresion fuera del ruido en `vn bench` frente a la fase anterior):

- [x] F1a. `SsaTerm<V = u32, B = u32>` es generico sobre los ids: el compilador
  lo instancia con `Value`/`BlockId`, el JIT con `u32`. `Terminator` y
  `term_uses!` se borran; `for_each_use`, `for_each_use_mut` y `map_ids` se
  escriben una vez (3159abfa).
- [x] F2. Cadenas `Arc<str>` tambien en la SSA portable (b07acd0c).
- [x] F1b. Identificadores compartidos. `vn_types::ssa::{ValueId, BlockId}`
  sustituyen a `Value`/`BlockId` del compilador y a los `u32` de `SsaOp`/`SsaTerm`
  (2e0a98d0; JIT en 4 lotes, 62 ficheros).
- F3. Un solo enum de instrucciones: `SsaOp` vive en `vn-types` y el
  compilador lo usa directamente (se borra `InstKind`); los pases hacen match
  sobre el. La proyeccion portable colapsa a `portable/lower.rs`: clona la op
  y solo baja 3 casos (remap de capturadas, `CallDirect` sin slot a `Call`,
  `Array/BytesGetIndex` con indice no entero a `Get/SetIndex`); el resto es
  copia mas las anotaciones (`cache`, `resume`) y las tablas (`call_sites`,
  `closure_sites`). `uses.rs` del compilador se borra en favor de
  `SsaOp::for_each_use`/`for_each_use_mut`.
  Hecho como limpieza previa (cada paso con puerta `-Min` en verde):
  - `CloseUpvalues` nombra `vars` en ambos lados (957481c8).
  - `MapGetIndex`/`MapSetIndex` se borran; el compilador usa `GetIndex`/`SetIndex`
    tambien para mapas (8715602b; se borra ademas `emit_map_get_index`, que emitia
    un RRR sin opcode).
  - `KindOf` es variante propia en `InstKind` como en `SsaOp` (ade6cefc).
  - `ArrayExtend`, `BindMethod` y `WrapSpread` se borran de `SsaOp`: nunca los
    construia la proyeccion (3f5c3236; se borran sus brazos en el JIT y sus
    constructores muertos en `extra/props`).
  Tras esto ambos enums tenian 87 variantes. Faltaban `wire_byte`/`wire`,
  `Try`, `MakeClosure` y la fusion:
  - `wire_byte` pasa a `wire` en el compilador (ea9d449b).
  - `Try` portable conserva `handler: BlockId` y `dest` como el compilador;
    el JIT lee el valor de `dest` (2bf73d4b).
  - `MakeClosure` conserva `func` en ambos; la constante de closure viaja en
    `SsaProto.closure_sites` (ip, proto) como `call_sites`, y el JIT la lee
    con `closure_proto_at` (0453c19e).
  - `InstKind` se borra; `const_ty` pasa a funcion libre; los 59 ficheros del
    compilador hacen match sobre `SsaOp` (1064197f, ac32e68a).
  - `SsaOp` gana `for_each_use_mut`; `replace_uses_with_map` pasa a metodo de
    `SsaFunc`; se borra `ssa/uses.rs` del compilador (1ae79031). Al unificar
    aparece una divergencia latente: la version portable no visitaba
    `AllocInstance.class` ni `DefineMember.member`; se corrige en la unica
    implementacion (los pases del compilador los necesitan como usos; sin
    esto la suite falla con `new: expected a class, got null`).
  - `ssa/portable/inst/` (6 ficheros de copias campo a campo) se borra;
    `lower.rs` clona y baja los 3 casos (f3998293).
  No hizo falta ningun parametro de nivel `Op<V, L>`: tras F1b (ids
  compartidos) y F3a (spreads genericos) las 87 variantes ya eran textualmente
  identicas una vez normalizados `Try` y `MakeClosure`; las diferencias que
  quedan son anotaciones fuera de la op (`cache`, `resume`, `call_sites`,
  `closure_sites`).
- [x] F3a. `SsaSpread` y `SsaObjectSpreadPart` son genericos sobre el id y el
  compilador los usa en lugar de tuplas (f00e0f21).
- [x] F4a. La ranura de cache y el punto de continuacion (`ip` + conjunto vivo) van
  en `SsaInst` (`cache`, `resume`), no en la operacion (f1cf5a87). Tras esto, 65
  de los 81 brazos de la proyeccion son copia.
- [x] F4b. Variables capturadas: `vn_types::ssa::SsaVar` (parametro o local) nombra
  la variable en compilador y JIT; `SsaProto::captured` es la tabla
  (variable, registro). `ir::VarId` se borra (65e9174c, f690e4ec). `Dispose`, que
  ningun constructor emitia, se borra (d2971db0).
- [x] F4c. `BuildObject` lleva pares en ambos lados y `ConstDecimal` el
  `BigDecimal` (7b8d611a, 90fbf8e7).
- [x] F5. La eleccion del operador por tipo baja a `vn-types` como funcion pura de
  los tipos (`select_bin`/`select_un` sobre `SlotKind`, `opcode_of_bin` para el
  backend). La emision de bytecode y la proyeccion llaman a la misma funcion;
  `SsaBinOp`/`SsaUnOp` se derivan de ella. Se borran `ssa/portable/ops.rs` y el
  modulo `lower` (8d7732c1).

## Pendientes
Orden de trabajo hecho: F5, F1b y limpieza previa de F3, de menor a mayor alcance.

- [x] F5: operador tipado en `vn-types`; se borran `portable/ops.rs` y `lower`.
- [x] F1b: `ValueId`/`BlockId` compartidos (62 ficheros entre `vn-types`,
  compilador y JIT).
- [x] F3: `SsaOp` unico en `vn-types`; el compilador lo usa, `uses.rs` del
  compilador borrado, proyeccion colapsada a `lower.rs` (3 lowerings).
  Detalle en la seccion F3 de arriba.
- Medir `vn bench` sobre `tests/benchmarks/vn` contra el binario previo para F4b y
  F4c, que pasaron la matriz completa sin esa medicion, y para F5, F1b y la
  limpieza de F3.

Defectos encontrados durante F4b, fuera de esta decision:

- `vn-emit/src/body/stmt_block.rs` (`drain_dispose_frame`) compara contra el literal
  `"dispose"` en lugar de un nombre canonico de `vn-core` (Ley 16).
- `drain_dispose_frame` llama siempre a `dispose`, nunca a `disposeAsync`: falta
  comprobar si `await using` libera el recurso de forma asincrona. La instruccion
  `Dispose` borrada distinguia ambos casos, pero ningun constructor la emitia.

## Verificacion
Cada fase compila sin la anterior duplicacion (no conviven dos rutas, Ley 8) y pasa la
matriz completa. Se mide `vn bench` sobre el corpus de `tests/benchmarks/vn` contra el
binario de la fase previa, con la cache JIT desactivada.

## Que se borra
`ssa/portable/inst/` (proyeccion campo a campo), `vn-compiler/src/ssa/uses.rs`,
`ir::Value`/`ir::BlockId` (ya eran alias), la mitad de las declaraciones de
variantes de instruccion. Quedan en `portable/`: `lower.rs` (los 3 lowerings +
anotaciones), `entries.rs`, `why_not.rs` y el driver `mod.rs`.

## Riesgos
F3 toca todos los pases del compilador (unas 20000 lineas) y F4 todo el lector del JIT.
Por eso F1 y F2 van primero: reducen la diferencia a un cambio de nombres antes de
fundir los enums.
