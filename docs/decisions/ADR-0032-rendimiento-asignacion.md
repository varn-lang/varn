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

- A1. `split` de un byte en una pasada, sin `Vec` intermedio de rangos ni `vals`.
  Puerta: `split` en bucle puro por debajo de 45 ns; checksums intactos.
- A2. Almacenamiento inline en celda para arrays y tuplas pequeñas (sin `malloc`
  ni `drop_object`). Puerta: `http_routing` y `generator` mejoran en `compare.ps1`.
- A3. Escape analysis interprocedural con resúmenes de parámetros no escapantes
  (hoy `passes/escape.rs` solo reemplaza instancias dentro de un bloque) y
  asignación en región de frame para arrays y `Map` que no escapan.
  Puerta: `new Map`/`split` no escapantes por debajo de 10 ns.
- A4. Si A2 deja el vivero sin celdas con destructor externo: barrido por bloque
  (bloques jóvenes totalmente muertos se reinician en O(1)).
  Puerta: GC menor por debajo del 5% en `http_routing`.

Cada paso se mide con `compare.ps1` y se borra el código que sustituye (Ley 10).
Comparación (Ley 21): JavaScriptCore y V8 usan vivero con bump allocation y
copia, donde los muertos no cuestan nada; Go y .NET igual con generaciones. Varn
mantiene celdas no móviles porque el JIT y los helpers retienen referencias crudas,
y compensa con almacenamiento inline y análisis de escape estático (Ley 20).

## Descartado
- Reducir el vivero (`NURSERY_BYTES`): empeora cargas que retienen datos.
- Saltar el desmontaje del heap al salir: sin ganancia medible (149.3 vs 148.8 ms).
- Despacho estático de métodos: ya empata con Bun.
