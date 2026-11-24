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
- F3. Un solo enum de instrucciones, por el mismo camino que F1a: `SsaOp`
  generico sobre el id de valor, con las cargas de las 26 variantes que difieren
  (operador tipado, ranuras IC, registros capturados) en tipos asociados a un
  parametro de nivel. `InstKind` pasa a ser `vn_types::ssa::Op`; los
  pases del compilador hacen match sobre el. `uses.rs` del compilador se borra en
  favor de `Op::for_each_use`/`for_each_use_mut`.
  Hecho como limpieza previa (cada paso con puerta `-Min` en verde):
  - `CloseUpvalues` nombra `vars` en ambos lados (957481c8).
  - `MapGetIndex`/`MapSetIndex` se borran; el compilador usa `GetIndex`/`SetIndex`
    tambien para mapas (8715602b; se borra ademas `emit_map_get_index`, que emitia
    un RRR sin opcode).
  - `KindOf` es variante propia en `InstKind` como en `SsaOp` (ade6cefc).
  - `ArrayExtend`, `BindMethod` y `WrapSpread` se borran de `SsaOp`: nunca los
    construia la proyeccion (3f5c3236; se borran sus brazos en el JIT y sus
    constructores muertos en `extra/props`).
  Tras esto ambos enums tienen 88 variantes. Queda: `Binary`/`Unary` (guardar el
  `SsaBinOp`/`SsaUnOp` ya elegido en `from_tir`), `MakeClosure` (`func` +
  `Vec<SsaUpvalue>` en ambos, con la tabla `closure_consts` como anotacion),
  `CallDirect` (`shape` a la tabla `call_sites`), y fundir los enums con el
  parametro de nivel.
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
- F3 restante: fundir `InstKind` y `SsaOp` en un unico `Op` generico por nivel.
  Limpieza previa hecha (cada paso con puerta `-Min` en verde):
  - `CloseUpvalues`: `vars` en ambos (957481c8).
  - `MapGetIndex`/`MapSetIndex`: borrados, `GetIndex`/`SetIndex` para mapas
    (8715602b; se borra `emit_map_get_index`, que emitia un RRR sin opcode).
  - `KindOf`: variante propia en ambos (ade6cefc).
  - `ArrayExtend`, `BindMethod`, `WrapSpread`: borrados de `SsaOp`, nunca
    construidos (3f5c3236).
  - `Binary`/`Unary`: guardan el `SsaBinOp`/`SsaUnOp` ya elegido en `from_tir`;
    emision mapea con `opcode_of_bin` y la proyeccion copia (c0f932d5; pureza
    de `NegInt` a impura para no eliminar su trampa).
  - `MakeClosure`: `SsaUpvalue` en ambos (af326aa8).
  - `Call`/`CallDirect`: `shape` en ambos (639f0dec; la tabla `call_sites` se
    mantiene para el JIT).
  - `DefineMember` (`member`), `GetFixedField` (`access`), `SetFixedField`
    (`kind`): nombres unificados; `Cast` pierde `ty` (e14e6cf1).
  Tras esto ambos enums tienen 88 variantes. Quedan, para el `Op` generico:
  - `MakeClosure`: `func` (id TIR, disponible al construir) frente a `proto`
    (indice de constante, disponible tras emitir) como anotacion de backend.
  - `Try`: `handler` + `dest` frente a `catch_value` sin `dest` + `resume`.
  El `Op` pasa a ser `vn_types::ssa::Op<V, L>` con esas cargas en tipos
  asociados al nivel; los pases hacen match sobre el; se borran `uses.rs` del
  compilador y `ssa/portable/`.
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
`ssa/portable/` (proyeccion), `vn-compiler/src/ssa/uses.rs`, `Terminator`,
`ir::Value`/`ir::BlockId`, la mitad de las declaraciones de variantes de instruccion.

## Riesgos
F3 toca todos los pases del compilador (unas 20000 lineas) y F4 todo el lector del JIT.
Por eso F1 y F2 van primero: reducen la diferencia a un cambio de nombres antes de
fundir los enums.
