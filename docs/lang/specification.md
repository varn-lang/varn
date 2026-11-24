# Varn Language Specification (índice verificado)

Fuente única: `tests/*.vn` (347 archivos; `tests/main.vn` importa 345). Código manda.

| Área | Doc | Test canónico |
|---|---|---|
| Panorama + regla sin sintaxis no probada | `overview.md` | `main.vn` |
| Sentencias, control, `match`, `using` | `statements.md` | `control-flow.test.vn`, `match.test.vn`, `errors.test.vn`, `ranges.test.vn` |
| Expresiones, `\|>`, templates | `expressions.md` | `pipeline.test.vn`, `templates.test.vn` |
| Tipos, nulables, uniones, genéricos, enums | `types.md` | `null-safety.test.vn`, `nullable-types.test.vn`, `unions.test.vn`, `generics.test.vn`, `advanced-generics.test.vn`, `enums.test.vn`, `advanced-enums.test.vn`, `char-type.test.vn`, `decimal-bigint.test.vn` |
| Funciones, closures, named-args, HOF | `functions_methods.md` | `closures.test.vn`, `complex-closures.test.vn`, `named-arguments.test.vn`, `advanced-higher-order.test.vn` |
| Clases, interfaces, enums avanzados | `classes_objects.md` | `classes.test.vn`, `interfaces.test.vn`, `advanced-enums.test.vn` |
| Módulos e imports | `modules_packages.md` | `main.vn`, `stdlib-migration-test.test.vn`, `stdlib-comprehensive-test.test.vn` |
| Async, isolates, canales | `async_concurrency.md` | `async.test.vn`, `globals-async-coherence.test.vn`, `async-generators.test.vn`, `isolates-multithread.test.vn`, `channels.test.vn` |
| Generadores | `generators.md` | `generators.test.vn`, `async-generators.test.vn` |
| Errores | `error_handling.md` | `errors.test.vn`, `try-operator.test.vn`, `multi-catch.test.vn` |
| Gramática formal (EBNF) | `grammar.md` | derivada de `vn-lexer` y `vn-parser`; verificada con `vn debug -p ast` |
| Léxico y literales | `lexical.md` | `arithmetic.test.vn`, `boolean.test.vn`, `strings.test.vn`, `templates.test.vn`, `char-type.test.vn`, `decimal-bigint.test.vn` |
| Identificadores y keywords | `identifiers.md` | `crates/vn-lexer/src/keywords.rs` |
| Glosario | `glossary.md` | términos con su test |
| Ejemplos que corren | `examples.md` | cada bloque cita su `tests/*.vn` |
| Runtime | `runtime_behavior.md` | links a `../internals/VM_ARCHITECTURE.md`, `../internals/RUNTIME_ARCHITECTURE.md` |
| Núcleo y plataforma | `SPEC_NUCLEO_Y_PLATAFORMA.md` | índice, propuesta histórica parcial |

Lo no citado con su `tests/*.vn` no es contrato.
