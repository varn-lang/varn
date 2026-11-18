# ADR-0030: Cierre estático de capabilities fs/net y erradicación de deuda verificada

## Estado
Propuesta (2026-11-18). Extrae el plan de la auditoría de código del ABI
`runtime: → std: → usercode` (commits `718e5958`, `0cdd931d`, `76b8dc81`),
del escaneo de deuda de los 12 crates del pipeline y del mapeo de huecos
`std`/JIT. Cada entrada cita evidencia en código, no en docs (Ley 12).

## Contexto

1. **El corpus de errores está huérfano.** `c97f47a0` borró `error_corpus.rs`
   (decisión del dueño: literalidad Ley 11, cero tests Rust) y dejó
   `tests/errors/*.vn` (439 fixtures con `// expect:`) sin runner: ningún
   `.rs` los consume, `verify.ps1` no los nombra. El checker puede regresar
   en 439 casos sin que ningún gate lo note. La semántica del runner antiguo
   es recuperable de git (`c97f47a0^:crates/vn-cli/tests/error_corpus.rs`):
   cabecera `error[VNxxxx]`/`warning`, `suggestion:`, fragmentos de fallo;
   `vn check` y `vn run` como oráculos; tokens ordenados; `help: ` presente;
   posiciones válidas; `pending/` excluido (subdir, el lector solo tomaba
   `*.vn` del nivel superior).
2. **`runtime:buffer` es nativo muerto.** Registrado en
   `crates/vn-builtins/src/modules/mod.rs:15` con 11 fns
   (`buffer_runtime.vn:1-11`), cero importadores en `std/`, sin `std/buffer/`.
   `Bytes` vive en `core:types/bytes` (`bytes.rs` propio); `buffer` duplica
   `fromBytes/len/getByte/...` como funciones libres. Las dos puertas de
   capa (`binder/imports.rs`, `vm/exec/modules.rs`) niegan `User → Runtime`,
   luego es inalcanzable desde usercode y nada lo usa: borrar no rompe nada
   observable y restaura la invariante 1:1 que asume `xtask/scaffold.rs:55`.
3. **`fs`/`net` son dinámicos donde el diseño exige estático.**
   `fs.rs`/`net.rs` deniegan en runtime (`FsRead/FsWrite`, `NetServer/NetClient`)
   pero `fs_runtime.vn`/`net_runtime.vn` declaran 0 `@capability`, y ningún
   wrapper de `std/{fs,net,http,ws}` re-declara. El corpus ya trata
   `fs.read`/`net.client` como capabilities de primera clase
   (`tests/116:24,65`, `tests/errors/decorator-capability-missing.vn`),
   y `sys.env/sys.exec/sys.ffi/ui/sqlite` ya son estáticos: es migración
   incompleta, no invento. Dominios canónicos únicos en
   `vn-core/src/sandbox.rs` (`fs.read fs.write net.client net.server
   sys.env sys.exec sys.ffi ui`), validados en parse
   (`decorators.rs:257-261`). Sin `io/task/ws` (sin gates dinámicos: nada
   que reflejar; añadir sería enforcement nuevo, fuera de alcance).
4. **`required_caps` muere en el lowering.**
   `vn-compiler/src/ssa/emit/mod.rs:217` emite `Vec::new()`; el campo existe
   (`vn-types/.../definition.rs:40`) y solo lo lee `cap_trace` de debug.
   Bloquea `vn audit` por dependencia (roadmap item 3).
5. **Deuda medida en 12 crates** (tgrep, fuera `tests/`): `panic!` en
   `sem/.../resolution.rs:12,25,33,42`, `core/atom.rs:90`,
   `core/runtime_ops.rs:34`, `compiler/.../{infra.rs:186,ops.rs:11,values.rs:314}`,
   `vm/.../jit_helpers/*` + `globals/mod.rs:35`; `unwrap` en
   `checker/.../{resource.rs:35,decorator_purity.rs:77,narrowing/*,calls/*}`,
   `sem/resolve/*` (7× `next().unwrap()`), `compiler/passes/*`,
   `binder/core/loader.rs:44`, `pipeline/class_link.rs:82`; `_ =>` en
   `checker/.../exhaustiveness.rs:282`, `tir/.../assign.rs:67`,
   `sem/infer/ops.rs:57` (`=> Dynamic`), 13 en `sem/.../compat/*`;
   `std::HashMap/Set` en `modules/contain.rs:12` y
   `compiler/.../compile.rs:182` (solo membresía hoy); `unsafe` Cell en
   `pipeline/execute.rs:114`. Cero `TODO/FIXME`, cero `unsafe` en
   lexer/parser/binder/sem/checker/tir/modules.
6. **JIT sin stubs** (macro impone 1:1), modelo two-tier verificado; 9 slots
   muertos (`load_const invoke_dynamic dispatch_intrinsic load_static_fn
   make_closure make_enum_variant array_pop str_slice
   build_object_with_shape`) y bails silenciosos.
7. **Estático medido**: `typeloss` en `HEAD`: 1337 genéricos / 1362 tipados
   en sitios mejorables; `GetProperty` en 215/356 filas.

## Decisión

- D1. Runner del corpus como **script** (`verify.ps1`), nunca tests Rust
  (respeta `c97f47a0`): misma semántica del runner borrado. `pending/`
  sigue excluido. Nueva fixture por cada código de error nuevo
  (primera: `decodes-unknown-op`, cubre el `VN4401` de `718e5958`).
- D2. Borrar `runtime:buffer` (registro + directorio). Sin superficie
  pública (puertas de capa), sin usuarios: Ley 10/18.
- D3. Reflejar estáticamente lo que el runtime ya deniega (línea de
  principio: **sin enforcement nuevo**):
  `fs_runtime.vn`: `fs.read` en `stat$/readFile*/readLines$/readDir$/exists$`
  (+Async), `fs.write` en `write*/append*/mkdir*/remove*` (+Async),
  `open$` partido en `openRead$`/`openWrite$(path, append)` (una función ↔
  un set; `FileMode` sigue como intención de usuario; el ctor `File`
  declara ambas por ramas), `copyFile$/rename$` (read+write); `net_runtime.vn`: `net.server` en
  `tcpListen$/tcpListenTls$/udpBind$`, `net.client` en
  `tcpConnect$/tcpConnectTls$`; resto libre (fd-ops: cap en
  `openRead$/openWrite$`;
  `tcpAccept/Read/Write/Close$`, `udpSendTo/RecvFrom/Close$`,
  cómputo puro, `tempDir$`: sin gate dinámico).
  Re-declarar en cada wrapper `std` que los llame (`fs,file,async_ops,glob,ops`,
  `net/*`, cascada `http/* ws/*` donde el checker lo exija). Aceptado:
  los wrappers anotados dejan de ser forwarders directos (decorados);
  el costo es una llamada en rutas frías de IO.
- D4. Tests que no declaren lo que usan se reescriben (Ley 15); los errores
  del checker son la checklist exhaustiva.
- D5. Enhebrar `required_caps`: checker → TIR → `proto` → manifiesto de
  paquete; puerta: paquete que toca red sin declarar es rechazado antes de
  ejecutar, suite verde.
- D6. Deuda: `panic!/unwrap` → diagnóstico acumulativo u `Option` en
  bordes; `_ =>` exhaustivos explícitos; `Fx*` en los dos `std::*`
  restantes; `execute.rs:114` a método seguro en `vn-vm`. Orden: tabla de
  tipos → checker → TIR → compiler → VM (solo runtime, a `RuntimeError`).
- D7. JIT: podar los 9 slots muertos (+bump `JIT_HELPERS_VERSION`) y
  auditar bails con causa visible; sin cambios de codegen en este ADR.
- D8. `GetProperty` y ADR-0028 restante (métodos/clases, estado no-const,
  nullary-en-paquete, `import *`, std-std): medir → intervenir → `xtask
  compare`; cada uno su commit con ratio y veredicto perf.

## Migración (un cambio por commit, Ley 9; `-Min` por commit, `-Fast` antes de push)

- [x] C0. `718e5958/0cdd931d/76b8dc81`: forwarder exacto, Layer pura, nullary.
- [x] C1. ADR-0030 (este archivo).
- [x] C2. Runner `tests/errors` en `verify.ps1` + fixture `decodes-unknown-op`.
- [x] C3. Borrar `runtime:buffer`. Puerta: build + suite + `scaffold` 1:1.
- [x] C4. Contratos `fs/net` (D3, con `open` partido). Puerta: parse + caps en std.
- [x] C5. Wrappers `std` + llamadores (D3/D4). Puerta: imagen std + `main.vn` verde.
- [x] C6. `required_caps` checker→TIR→proto (dato + `cap_trace` real).
  Enforcement por paquete: pendiente de decisión (roadmap item 3).
- [x] C7. Deuda (D6): contenedores Fx, unsafe encapsulado, unwraps→
  `Option`, wildcards exhaustivos, pases y lowering a `Result`,
  panics VM→`RuntimeError`. Puerta: clippy + suite + corpus.
  Retenidos con justificación (auditoría, no omisión): `un_op`,
  `suspend -> !`, tripwires de heap (fail-stop), invariante de boot,
  `ids.rs` u32, wildcards estructurales/defer/fail-closed de `compat`,
  reglas generales de narrowing, `op_id` solo-constantes (rutas de
  usuario usan `try_op_id`).
  Follow-ups con diseño propio: migración `CheckerTyTable::get`/
  `AtomInterner::resolve` a tipos totales, enum `RuntimeOp`,
  enforcement de caps por paquete/manifiesto.
- [x] C8. JIT: 9 slots muertos podados (D7). Sin bump (nada los
  referenciaba ni se serializa). Puerta: suite + tiers sin regresión.
- [x] C9. `GetProperty`: medido, sin cambio de compilador justificado.
  `typeloss` en HEAD: 1337 genéricos/1362 tipados en sitios mejorables;
  `GetProperty` domina tests/ (216 filas: `x`49 `y`39 `kind`38 `name`32…),
  pero en `tests/benchmarks` (33 ficheros, escritos para ser rápidos) solo
  quedan 5 filas. El gap no sobrevive donde el código va anotado: es
  test-code dinámico, no defecto inherente (skill vn-static-coverage).
  Sin benchmark que muestre impacto, Ley 10 prohíbe el opcode nuevo.
  ADR-0028 (métodos/clases, estado no-const, nullary-en-paquete,
  `import *`, std-std) queda como proyectos RFC con su propia puerta
  (ratio + `xtask compare`).
