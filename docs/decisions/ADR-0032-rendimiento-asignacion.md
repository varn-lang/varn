# ADR-0032: Rendimiento de asignación frente a Bun

## Estado
Propuesta (2026-11-21).

## Contexto
Medido con `compare.ps1 -SkipPython -Runs 7` (release, checksums verificados):
5 victorias, 3 empates y 7 derrotas frente a Bun. Peores: `http_routing` 2.22x,
`generator` 1.72x, `csv_pipeline` y `json_native` 1.54x, `json_api_payloads` 1.52x.

Perfil nativo (samply + mapa de enlazado) de `http_routing` con caché JIT caliente:
~35% código JIT, ~15% asignación y GC menor, ~9% `mi_malloc`, ~7% `split`,
~6% `json.stringify`, ~5% concatenación. Sin JIT el benchmark es 4.1x más lento, así
que el JIT funciona; el despacho de métodos y `new` de clase empatan con Bun.

Causa estructural: `Map`, arrays y strings largas guardan su almacenamiento fuera
de la celda (`Vec`, tabla hash). Cada objeto muerto del vivero exige
`forget_identity` + `release` + `drop_object` por celda, y cada asignación hace una
`malloc` aparte. En un bucle de 30M `new Map`: 39% en `alloc_map_vm`, 30% en GC menor.
Reducir el vivero acelera el bucle sintético 27% pero empeora `http_routing`
(120 a 154 ms), por lo que el tamaño del vivero no es la palanca.

## Decisión
Atacar el ciclo de vida del objeto joven, en este orden y con una puerta por paso:

- A1. DESCARTADO tras medir: `split` sin `Vec` intermedio no cambia el tiempo
  (655 vs 652 ms en 10M); el coste esta en el ciclo de vida del array resultado.
- [x] A2a. El almacenamiento del array literal lo fija el compilador
  (`ArrayStorage` en `vn-core`, decidido en `from_tir`, viaja por `InstKind`,
  `SsaOp`, el segundo byte del opcode y el helper JIT; `ArrayRepr::from_slice`
  eliminado). Medido: array literal que escapa 333 a 318 ms (10M); suite 6165
  verde. Los natives tambien declaran el almacenamiento: `NativeCtx::alloc_array`
  lo recibe, `IntoVm::STORAGE` lo fija por tipo y la inferencia por escaneo queda
  confinada a `alloc_array_inferred` (sin regresion en json, csv, str_ops).
- A2c. Los natives genericos (`map`, `filter`, JSON, `decode`, tuplas) siguen
  infiriendo con `alloc_array_inferred`: no reciben el tipo estatico del
  resultado. Cerrarlo exige que el compilador pase el almacenamiento como
  metadato de `CallNativeOp` y que las tuplas sean siempre `Boxed`.
  Puerta: cero usos de `ArrayRepr::inferred` y sin regresion en `compare.ps1`.
- A2b. Vivero con copia (sustituye a A2 inline y A4). Hallazgos que lo condicionan:
  el JIT lee `elems_ptr_off`/`elems_len_off` de `JitArrayLayout`, por lo que un
  `SmallVec` rompe el codigo inline; `malloc`/`free` de `mimalloc` suman solo
  ~6 ns de 33; el coste dominante es la muerte individual del objeto joven
  (`forget_identity` + `release` + `drop_object`). El GC solo corre en
  safepoints con raices precisas (`gc_backedge_safepoint`), nunca dentro de una
  asignacion, de modo que mover objetos es viable. Requiere: raices mutables
  (hoy `gc_roots()` devuelve una copia), reenvio en `for_each_child`, tablas por
  direccion (`identity_index`, `young_cells`) actualizadas, y que ningun native
  retenga un `HeapRef` joven a traves de un safepoint.
  Puerta: array literal que escapa y `new Map` por debajo de 12 ns; GC menor
  por debajo del 5% en `http_routing`.
- A3. Escape analysis interprocedural con resumenes de parametros no escapantes
  (hoy `passes/escape.rs` solo reemplaza instancias dentro de un bloque) y
  asignacion en region de frame para arrays y `Map` que no escapan.
  Puerta: `new Map`/`split` no escapantes por debajo de 10 ns.

Cada paso se mide con `compare.ps1` y se borra el código que sustituye (Ley 10).
Comparación (Ley 21): JavaScriptCore y V8 usan vivero con bump allocation y
copia, donde los muertos no cuestan nada; Go y .NET igual con generaciones. Varn
mantiene celdas no móviles porque el JIT y los helpers retienen referencias crudas,
y compensa con almacenamiento inline y análisis de escape estático (Ley 20).

## Descartado
- Reducir el vivero (`NURSERY_BYTES`): empeora cargas que retienen datos.
- Saltar el desmontaje del heap al salir: sin ganancia medible (149.3 vs 148.8 ms).
- Despacho estático de métodos: ya empata con Bun.
