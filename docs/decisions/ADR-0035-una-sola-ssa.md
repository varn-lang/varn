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

- F1. Identificadores compartidos. `vn_types::ssa::{ValueId, BlockId}` sustituyen a
  `Value`/`BlockId` del compilador y a los `u32` de `SsaOp`/`SsaTerm`. `Terminator` y
  `SsaTerm` pasan a ser un solo tipo, con un solo recorrido de operandos.
- F2. Cadenas y constantes con una sola representacion (`Arc<str>` en ambos lados;
  serde con `rc`). Las 55 variantes de copia quedan identicas en forma.
- F3. Un solo enum de instrucciones. `InstKind` pasa a ser `vn_types::ssa::Op`; los
  pases del compilador hacen match sobre el. `uses.rs` del compilador se borra en
  favor de `Op::for_each_use`/`for_each_use_mut`.
- F4. Las anotaciones de backend (ranura IC, registros capturados, `ip`, landing de
  `try`) se guardan en una tabla paralela del `SsaProto` en lugar de reescribir la
  instruccion. El JIT las lee de esa tabla.
- F5. La eleccion del operador por tipo (`binary_opcode`) baja a `vn-types` como
  funcion pura de los tipos, y la usan la emision de bytecode y el JIT; `SsaBinOp` y
  `SsaUnOp` se derivan de ella. Se borra `ssa/portable/`.

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
