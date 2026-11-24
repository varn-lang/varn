# Varn — Biblioteca estándar (puntero)

Mapa completo de paquetes: `../STDLIB_ARCHITECTURE.md` (30 dirs en `std/`, incl. `gpu`, `ui`, `text`).
Este archivo no duplica esa lista; solo ancla lo ejercitado por la suite.

## Núcleo async y tareas (`std:task`)

`async/await`, `spawn`, `parallel`, `TaskGroup`, canales, isolates:
`async_concurrency.md`. Tests: `tests/async.test.vn`,
`tests/stdlib-task-combinators.test.vn` (`Task.all`, `allSettled`, `race`),
`tests/stdlib-consolidation.test.vn` (`Mutex`, `Semaphore`, `WaitGroup`),
`tests/isolates-multithread.test.vn`, `tests/channels.test.vn`.

## Colecciones y tipos de plataforma

- `Array` (`map/filter/reduce/find/push/pop/sort/slice/join/spread`):
  `tests/arrays.test.vn`, `tests/range-indexing.test.vn`.
- `Map` / `Set`: `tests/map-set.test.vn`.
- `Range` (`..`, `..=`, `Range.from`, `step`, `toArray`, slicing):
  `tests/ranges.test.vn`, `tests/range-indexing.test.vn`.
- Tuplas `(…)` / structs: `tests/tuples-structs.test.vn`.
- `str`: `tests/strings.test.vn` (`trim`, `padStart/End`, `charCode`,
  `substr`, `isEmpty/isBlank/isDigit`, `capitalize`, `str.fromCharCode`, `str.join`).
- `char`: `tests/char-type.test.vn` (`charCodeAt`, `char.fromCode`, `toString`).
- `int` API (`wrapping/saturating/checked*`, `div/floorDiv/ceilDiv/rem/mod`,
  `parse/tryParse`): `tests/int-api.test.vn`.
- `decimal` / `bigint`: `tests/decimal-bigint.test.vn`,
  `tests/bigint-arbitrary.test.vn`, `tests/numeric-conversions.test.vn`.
- `Bytes` + streams: `tests/canonical-bytes-and-streams.test.vn`.
- Rangos genéricos, tipos literales, intersecciones, capacidades y operadores:
  `tests/range-generic.test.vn`, `tests/literal-types.test.vn`,
  `tests/intersections.test.vn`, `tests/capabilities.test.vn`,
  `tests/operator-capabilities.test.vn`.

## Formatos, red, sistema

- `std:encoding` (`Csv`, `Json`): `tests/std-csv.test.vn`.
- `Regex` (tipo del núcleo, sin import), `std:crypto` (UUID), `std:time`:
  `tests/canonical-types-regex-uuid-time.test.vn`.
- `std:process`, `std:compress`, `std:cli`, `std:env`, `std:path`:
  `tests/stdlib-process-compress-cli-env.test.vn`.
- `std:sqlite`, `std:ws`, archivos: `tests/stdlib-sqlite-ws-archive.test.vn`.
- Tagged templates + SQL (`db.sql`, `db.sqlOne`): `tests/tagged-templates.test.vn`.
- `std:encoding`, `std:cli` (Color/Table/Prompt), `std:log`, `std:collections`
  (PriorityQueue, LRUCache): `tests/stdlib-consolidation.test.vn`.
- `std:net`, `std:http`, `std:ws`:
  `tests/network-apis-overhaul.test.vn`.
- Migración y superficie completa: `tests/stdlib-migration-test.test.vn`,
  `tests/stdlib-comprehensive-test.test.vn`.

## Test y reflexión

- `std:test` (`describe/it/expect`): `tests/async.test.vn`,
  `tests/stdlib-task-combinators.test.vn`; framework: `tests/stdlib-test-framework.test.vn`.
- `std:reflect` (`MetaKey`, `ClassRef`; contextos de decorador globales):
  `tests/decorators.test.vn`.
- `std:fs` (`readText/writeText/remove/exists`), `std:env` (`Env`), `std:sys` (`platform/cwd`):
  `tests/capabilities-security.test.vn`.
- Ficheros vía `using`: `tests/implicit-extensions-test.test.vn`,
  `tests/simple-file-test.test.vn`.

Métodos no citados arriba: NO-VERIFICADOS. Ver `../STDLIB_ARCHITECTURE.md`.
