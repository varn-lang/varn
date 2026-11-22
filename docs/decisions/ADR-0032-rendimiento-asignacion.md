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
- [x] A2b. Politica adaptativa del vivero (`nursery_target`: mitad con 2% o menos
  de supervivencia, doble con 10% o mas, suelo 1 MiB, techo 16 MiB). Medido:
  `new Map` 879 a ~720 ms, array literal 318 a ~265 ms, `split` 655 a ~555 ms,
  `http_routing` sin cambio; `compare.ps1` pasa de 5/3/7 a 8/1/6 frente a Bun.
  DESCARTADO el vivero con copia: `MapKey` hashea los valores heap por direccion
  (`tag`+`payload`), las cadenas internadas se identifican por direccion y `===`
  compara direcciones; mover objetos rompe `Map`, `Set` e internado. Hacerlo
  exigiria una identidad independiente de la direccion (handles con indireccion
  en cada acceso), lo que penaliza el JIT.
- [x] A2d. Arrays y tuplas pequenos dentro de su celda: `CellVec<T>` (ptr, len y
  capacidad con bit de propiedad, 24 bytes) con los elementos inline tras el
  `ArrayRepr` (hasta 32, minimo 4 si nace vacio); desbordar pasa a heap y se
  contabiliza en el ledger. Los offsets `ptr`/`len` que lee el JIT son constantes
  de `CellVec`. Medido: array literal que escapa 265 a ~175 ms (10M); suite 6165
  verde y estres de GC con verificacion sin fallos. Al medirlo aparecio un
  precipicio de pacing (el ledger externo fijo en 16 MiB disparaba un segundo GC
  menor y uno mayor en `http_routing`, que retiene el 76%), corregido en A2b
  escalando vivero y presupuesto por 1/(1 - supervivencia) y midiendo el tamano
  medio de celda (antes caia siempre a 24 B).
- [x] A2e. Coste de la celda y de la construccion. (1) `take` precarga la siguiente
  celda libre. (2) Las celdas se escriben en sitio: `alloc_map_vm` copiaba el
  `ValueMap` por valor con `movups` justo tras stores de 8 B (fallo de reenvio de
  store, ~12 ns por Map); `new Map` 700 a ~540 ms (30M). (3) Entradas de `Map`/`Set`
  de hasta 8 elementos dentro de la celda (`OrderedTable` sobre `CellVec`,
  `alloc_map_sized` + `map_put`): `Map.set` con alloc 239 a 153 ms (5M). (4) El
  pacing no vuelve a recolectar hasta asignar 2x lo sobrevivido (tipo GOGC).
  Perfil de `http_routing` tras esto: JIT 34%, construccion de Map 14.6%, GC menor
  10.7%, `mimalloc` 8%, `split` 7.4%, `json.stringify` 6.7%, concatenacion 4.2%.
- A2f. GC por bitmaps con barrido perezoso (marcas fuera de la celda, bloques sin
  vivos reiniciados por bump, sin lista libre ni `born`). Medido: el coste de
  recoleccion por objeto es ~4 ns de ~21 en `new Map`, de modo que su techo es
  ~10 a 20% en bucles de asignacion y menos en `http_routing`. Es buena
  arquitectura pero no la raiz de la brecha; va despues de A5.
- A5. Codigo JIT. Medido por fases en `http_routing` (100k peticiones): construir
  las peticiones, Varn 28 ms contra Bun 33 ms (ganamos); autenticacion, 9 contra
  8; handler (`jsonRes`, `stringify`, `Map`), 26 contra 28 (ganamos); **escaneo
  de rutas, 53 contra 23 ms: toda la brecha esta ahi**. Aislado en un
  microbenchmark (20 rutas, 6M iteraciones) Varn es 2.35 a 2.6x mas lento que Bun
  incluso en la variante minima (comparar metodo y longitud), de modo que el coste
  es el base por operacion y no una sola: la ruta rapida de `this.method != "*"`
  son ~100 instrucciones (prologo con volcado de argumentos y de sp/fp al ctx,
  comprobaciones de tag sobre una constante SSO que el JIT no reconoce, comparacion
  de cadenas general). `VN_CLIF_OPT` none/speed/speed_and_size no cambia el tiempo
  (120/119/119 ms): no es calidad de instrucciones sino el modelo de valores.
  Hecho: lecturas de campo compactas no anulables sin revalidar tipo
  (`Out::Trusted`, -11% de codigo). Descartado por no medirse: `startsWith` y
  `endsWith` SSO inline. Dirección (Ley 17 y 20): el JIT debe especializar por el
  tipo estatico que ya conoce (valores `str` y `Ref` no anulables sin tag
  dinamico, constantes SSO comparadas por valor, `this` y receptores fijos sin
  guarda de instancia, prologo sin volcados innecesarios), no por operaciones
  sueltas. `VN_JIT_SYMS=<fichero>` anota la direccion de cada funcion JIT para
  atribuir muestras de un perfilador (samply ahora exige administrador).
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
