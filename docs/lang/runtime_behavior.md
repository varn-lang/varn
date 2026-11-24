# Varn — Comportamiento en tiempo de ejecución (links)

> Fuentes: `tests/async.test.vn`, `tests/isolates-multithread.test.vn`, `../VM_ARCHITECTURE.md`, `../RUNTIME_ARCHITECTURE.md`, `../COMPILER_ARCHITECTURE.md`.

Sin cifras: FrameStore, GC, JIT y pipeline ya están documentados en esos archivos.
No duplicarlos aquí.

## Lo verificado en tests

- Dos backends: intérprete bytecode y JIT. `VN_NO_JIT=1` fuerza intérprete
  (matriz de validación en `CONTRIBUTING.md`).
- Heap compartido en hilo principal; cada `spawnIsolate` crea heap separado,
  comunicación solo por canales
  (`tests/isolates-multithread.test.vn`).
- Top-level `await` en módulo (`tests/async.test.vn`,
  `tests/async-generators.test.vn`).
- `import.meta.main`: `true` solo en el módulo de entrada; `false` en módulos
  importados y en los que carga un isolate. Vive en el slot 0 de la región de
  globales de cada módulo y lo escribe el loader (`tests/import-meta-main.test.vn`).
- `int` 64 bits con `IntegerOverflow`, sin wrap
  (`tests/int-division.test.vn`); `int / int` = `int` truncado
  (`tests/int-division.test.vn`).
- La stdlib se carga siempre desde una imagen `std.vnb`: en un checkout se
  construye desde `std/` en la caché; en un binario de release va embebida
  (`VN_STD=@embedded`). Ver `../MODULES_SYSTEM.md`.

## Closures

Capturan por upvalue del frame padre (`tests/closures.test.vn`); ver
`../VM_ARCHITECTURE.md` para el mecanismo.
