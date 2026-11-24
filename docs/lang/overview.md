# Varn — Overview (desde tests)

Fuente única: `tests/*.vn`. Código manda.

## Suite

- 347 archivos `tests/*.vn`; `tests/main.vn` importa 345.
- No importados: `main.vn` y `async-shapes-lib.test.vn` (auxiliar de `async-shapes.test.vn`).
- Cierre: `import { summary } from "std:test"` + `summary()` al final de `main.vn`.
- Huecos de numeración: 39, 81, 91, 104–108, 172, 249, 333.

## Rasgos probados (cita mínima)

- `try/catch/finally`: `tests/errors.test.vn` (safeDivide, withFinally), `44-48` (catch TypeError, `e.name`).
- `try` prefijo (early-return sobre Result/Option/nullable): `tests/try-operator.test.vn` (Result), `:35` (Option), `:44` (nullable `str?`).
- Multi-catch tipado + unión: `tests/multi-catch.test.vn` (handler `catch (e: TypeError)`, `RangeError`, network, database).
- `match`: `tests/match.test.vn` (literal, `|`, `_`).
- Clases: `tests/classes.test.vn` (static, getter/setter, herencia, `super`, `override`, abstract).
- Async: `tests/async.test.vn` (`async/await`, `spawn`, `parallel`).
- Generadores: `tests/generators.test.vn` (`function*`, `yield`, `.next()` `{value,done}`).
- Async-generadores: `tests/async-generators.test.vn` (`async function*`, `yield await`).
- Isolates + canales: `tests/isolates-multithread.test.vn` (imports `spawnIsolate,channel,Sender,Receiver,ChannelClosed`), `:10-39` (workers), más `tests/channels.test.vn` (close, `ChannelClosed` en send/receive).

## Regla

Sin sintaxis no probada: lo no citado arriba ni en `docs/lang/*.md` con su `tests/*.vn` no es contrato.
