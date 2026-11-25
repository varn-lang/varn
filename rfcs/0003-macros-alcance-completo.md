# RFC 0003: alcance completo de macros `#nombre`

- Estado: draft
- Fecha: 2026-11-26
- Continúa a RFC 0002 (reserva de `#`, tuplas `#[...]`, records `#{...}`)

## Resumen

Varn tendrá un único sistema de macros con sintaxis `#nombre` y `#nombre(...)`, más formas con `#` para tuplas y records ya aceptadas. Sin `!`, sin proc-macros arbitrarias, sin `eval`. Tres familias y nada más: derivación, atributo y función. Expansión pura entre parse y bind, higiénica, determinista, sin disco ni red directos, con diagnósticos acumulativos.

## Motivación

La std y los tests repiten patrones que hoy se resuelven con texto o con decoradores `@` que envuelven en runtime:

```vn
for (const #[k, v] in obj) { encodeValue(w, k); encodeValue(w, v); }
function comparatorOf(part: str): #[Comparator, int] { ... }
const TYPES = new Map<str, str>([#["html", "text/html"], #["css", "text/css"]]);
```

Faltan tres cosas que ningún decorador `@` puede dar sin dispatch dinámico: generar código nominal (impls para `struct`/`enum`), validar literales en compilación (`sql`, `regex`, `html`) y compilar por plataforma (`cfg`). Hacerlo con funciones de runtime pierde el tipo estático y rompe la Ley 17. Hacerlo con plantillas de texto rompe la Ley 5.

## Separación `@` frente a `#`

- `@nombre` es decorador: no genera sintaxis, anota una declaración ya existente y el checker la valida contra su objetivo (`BuiltinDecoratorKind::supports`). Puede envolver en runtime (`wraps_at_runtime`). Casos: `@throws`, `@capability`, `@pure`, `@inline`, `@decodes`.
- `#nombre` es macro: genera sintaxis antes del binder. Nunca envuelve en runtime, nunca decide por tipos, nunca toca disco.

Ningún nombre existe en ambos canales. Si una idea necesita tipos, es `@`. Si necesita sintaxis, es `#`.

## Sintaxis

```vn
#derive(Debug, Clone)
#test
#cfg(target == "wasm")

const q = #sql("select id from t where id = ?", id);
const re = #regex("^[a-z]+$");
const t: #[int, str] = #[1, "a"];
const r: #{ x: int } = #{ x: 1 };

#route("GET /users/:id", handler)
```

Reglas léxicas:

- `#` pegado al nombre, a `[` o a `{` forma una unidad: `#[`, `#{`, `#ident`. `#` suelto o `# [` con espacio es error.
- `#nombre` sin paréntesis es atributo. `#nombre(...)` usa la misma gramática de args de llamada: posicionales, nombrados (`label: expr`), spread (`...xs`), más `#[...]`, `#{...}`, lambdas y bloques.
- Posiciones: expresión (`#sql(...)`), tipo (`#cfg_t(...)` solo si el RFC de tipos lo autoriza; por defecto no hay macros en tipos salvo lista cerrada), patrón (solo `#` de tupla/record ya existente; macros de patrón quedan fuera del alcance inicial), item (`#derive(...)` ante `struct`/`enum`/`class`, `#test` ante `function`, `#route(...)` ante `function`).

Lo que no existe: `#nombre[...]`, `#nombre{...}`, `nombre!(...)`, `$var`, `macro_rules!`, interpolación de texto. Los args son AST, nunca texto.

## Taxonomía cerrada

Solo tres familias. Toda propuesta nueva entra en una de ellas o se rechaza.

### 1. Derivación `#derive(Nombre, ...)`

Genera miembros nominales para el item anotado. No lee el cuerpo de funciones, no infiere tipos, no toca otros módulos.

Lista inicial cerrada: `Debug`, `Clone`, `Eq`. Cada una tiene expansión fijada en este RFC y no acepta args salvo los documentados.

```vn
#derive(Debug, Clone)
struct Point { x: int; y: int; }
```

Expande a impls con el mismo `Atom` de campos, rangos del sitio de llamada y sin nombres nuevos visibles fuera del item.

### 2. Atributo `#test`, `#bench`, `#cfg(...)`

Modifica cómo se compila el item anotado, sin generar items nuevos con nombres inventados.

- `#test` ante `function`: la registra como caso de `std:test`. Equivale al registro manual actual, sin cambiar su tipo.
- `#bench` ante `function`: igual para benchmarks.
- `#cfg(predicado)` ante cualquier item o rama: compilación condicional con predicado cerrado (`target`, `feature`). El predicado es AST evaluado por el expandor contra `Session`, nunca código de usuario.

### 3. Función `#sql(...)`, `#regex(...)`, `#html(...)`

Devuelven una expresión con el tipo ya cerrado y validación en compilación.

- `#sql(texto, ...args)`: `texto` debe ser `StrLiteral`. Valida placeholders `?` contra el número de args. Devuelve un builder tipado, no un `str`.
- `#regex(texto)`: `texto` debe ser literal. Compila el patrón con el mismo motor del runtime en el expandor; si falla, diagnostica en compilación.
- `#html(texto, ...args)`: valida balance de etiquetas en compilación para los literales dados.

Ninguna ejecuta consultas, abre ficheros ni toca red.

## Fase de expansión

Nuevo crate puro `vn-expand`, fase `AST -> AST` entre parse tolerante y check, auditada contra el pipeline real:

`tokens -> AST con # -> AST sin # -> símbolos/alcances -> tipos -> TIR`

Puntos de inserción obligatorios para no divergir entry y dependencias:

- `crates/vn-pipeline/src/compilation.rs:129-145` tras `parse_tolerant`, antes de `check_program`.
- `crates/vn-pipeline/src/quiet_parse.rs:4-16` para dependencias.
- `crates/vn-pipeline/src/stdlib_loader.rs:109-139` en `compile_source_inner`.
- `crates/vn-resolver` `resolver_parse.rs` y `cache.rs` más `crates/vn-contract/src/parse.rs:11-15` si los contratos deben ver macros.

Contrato:

- Entrada inmutable: `AstArena` más tabla de macros registradas. Salida inmutable: `AstArena` nueva más `DiagnosticBag`. Sin estado global, sin `Arc` mutable compartido.
- Pura y re-ejecutable: misma entrada produce mismos bytes. Sin `HashMap`/`HashSet` para generación de nombres o símbolos; `IndexMap`/`BTreeMap` o vectores ordenados.
- Límite de recursión y de coste: profundidad máxima fija, cada macro declara si es recursiva; la expansión que la supere es error, no cuelgue.
- Rangos preservados: cada nodo generado hereda el `SourceRange` del sitio `#`. Los diagnósticos apuntan al `#`, nunca a código invisible.
- Higiene: los identificadores introducidos llevan el `Span` del sitio y no capturan locales del llamador. Si una expansión necesita un nombre auxiliar, usa un `Atom` que no puede escribirse en superficie o califica con el módulo del item.
- Sin IO: la firma es `trait MacroExpander { fn expand(input: MacroInput) -> MacroOutput }` en capa baja; el disco, el `ModuleLoader` y la `Session` viven arriba y se pasan por parámetro. `#include`, `#env` y `#embed` solo existen si pasan por ese trait con contenido ya cargado; en el alcance inicial no se incluyen.

Orden: primero atributos de item de fuera hacia dentro, luego funciones de dentro hacia fuera, una sola pasada salvo que una expansión devuelva otro `#` bien formado, con límite fijo. Nada de punto fijo hasta convergencia.

## Capacidades y sandbox

Pueden: inspeccionar el AST de sus args, generar `ExprKind`, `Pattern`, `Decl` y `TypeNode` ya existentes, leer la tabla de macros y la `Session` inmutable que se les pasa.

No pueden: leer disco o red, leer variables de entorno salvo valor ya inyectado por `Session`, generar `unsafe`, generar decoradores `@` nuevos, cambiar visibilidad, tocar la tabla de tipos, resolver nombres por su cuenta ni emitir bytecode.

Toda macro fuera de la lista cerrada es `VN???? macro desconocida` con sugerencia a la lista. Toda macro que falle en validación emite diagnóstico y deja un nodo `Missing` para no cascada.

## Interacción con el resto

- `vn-lexer`: sin cambios. `Hash=144` ya existe y hoy es sintaxis de valores/tipos (`#[...]` tupla en `primary/mod.rs:314-352`, `#{...}` record en `types/primary.rs:330-400`, patrones en `patterns.rs:211` y `match_expr.rs:330`), no preprocesador. `#privado` sigue rechazado por `HashPrivate`.
- `vn-parser`: `#` deja de ser error genérico y produce `ExprKind::Macro`, `StmtKind::Macro` o atributo de item según posición. `#[` y `#{` siguen siendo tupla y record, no macros.
- `vn-expand` (nuevo crate): único dueño de la expansión. Ninguna otra fase expande. Firma compatible con Leyes 1/3/5: recibe `(&Program, &AstArena, &AtomInterner)` más `&dyn ImportResolver` o `ModuleLoader`, devuelve `(Program, AstArena, AtomInterner, DiagnosticBag)`. Sin `CheckerTyId`, sin `TyTable`, solo `TypeNode` y nombres. Sin `HashMap` para nombres generados.
- `vn-binder` (`binder_entry.rs:10-152`, `imports.rs:7-260`), `vn-sem`, `vn-checker` (`stages.rs:9-37`), `vn-emit` (`lib.rs:33-245`), `vn-compiler` (`from_tir/compile.rs:108`), `vn-vm`, `vn-jit`: no conocen `#`; operan sobre el AST ya expandido.
- Determinismo y caché: ordenar expansiones como `module_precompile.rs:121-150`, incluir versión del expansor en `fingerprint`, `graph_hash` y `BUILD_FINGERPRINT`.
- `vn-debug`, `vn-lsp`: muestran el AST expandido más el rango del `#` origen. Sin sintaxis especial de depuración.

## Introspección comparada (Ley 21)

- C#: `[Attr]` atributos y generadores Roslyn con IO y estado. Varn adopta la posición de atributo pero rechaza el generador con IO: la expansión es pura y sin proyecto.
- Python: decoradores `@` que envuelven en runtime y `ast` manipulable con `eval`. Varn mantiene `@` para envoltura y reserva `#` para generación estática; `eval` no existe.
- Dart: `macro` experimentales con introspección de tipos. Varn lo rechaza: la macro no ve tipos, solo sintaxis, por Ley 14.
- Swift: `#available`, `#keyPath`, macros con roles y firmas. Varn adopta roles cerrados (derive, atributo, función) pero sin sobrecarga por tipo de retorno.
- Kotlin: `data class` generado por el compilador, sin macros de usuario. Varn adopta la lista cerrada de derivaciones como punto de partida.
- Rust: `macro_rules!` higiénicas por texto y proc-macros arbitrarias con IO. Varn adopta la higiene y los roles de Rust, pero rechaza el texto y la arbitrariedad: args son AST tipado y la lista es cerrada.

Puente TypeScript: TS no tiene macros de sintaxis; su vía es el sistema de tipos más funciones. Varn conserva la superficie TS (`function`, `struct`, `=>`, `#[...]` como única distancia justificada) y pone la generación donde TS pondría un tipo imposible de expresar. Donde TS recurre a `any` o a código generado fuera del lenguaje, Varn ofrece `#derive` y `#cfg` dentro del lenguaje y con tipo cerrado (Leyes 17, 20, 23).

## Ganancia y verificación

- Corrección: `sql` y `regex` mal formados fallan al compilar, no en runtime. `Debug` y `Clone` dejan de escribirse a mano.
- Determinismo: misma fuente produce mismo bytecode; la suite lo verifica con expansión activada y desactivada para macros marcadas puras.
- Medición: `cargo build --profile quick`, `clippy` sin `wildcard_enum_match_arm`, corpus con `#[...]`/`#{...}` ya migrado en std, más `vn test` para cada macro con casos de higiene, error de validación y preservación de rangos.

## Qué se borra y qué no se toca

- Se borra el error genérico `unexpected #` como estado final: `#` pasa a ser sintaxis reservada con lista cerrada.
- No se borra ni se duplica ningún `@`: decoradores y macros no comparten nombres ni objetivos.
- No se añade `eval`, `include!` con disco directo, proc-macros de usuario ni sintaxis `$`.

## Plan por fases

1. Parser reservado: `#nombre` y `#nombre(...)` producen nodo `Macro` y error `macro no implementada` con puntero a este RFC. Sin expansión.
2. `vn-expand` esqueleto: registra la lista cerrada, expande identidad, preserva rangos, emite diagnósticos, con límite de recursión y tests de determinismo.
3. Derivación: `Debug`, `Clone`, `Eq` para `struct` y `enum` de datos.
4. Función: `#regex` y `#sql` con validación en compilación.
5. Atributo: `#test`, `#bench`, `#cfg`.

Cada fase es un commit atómico con su verificación. Ninguna fase añade una ruta paralela de tuplas, records o decoradores.

## Preguntas abiertas

- Macros en posición de tipo más allá de la lista cerrada.
- `#embed` y `#env` pasando por `MacroLoader` sin romper la Ley 1.
- Formateo de código expandido en `vn fmt` y vista en LSP.
- Código de error definitivo por familia (`VN4xxx` para expansión, `VN4xxx` para validación de cada macro).
