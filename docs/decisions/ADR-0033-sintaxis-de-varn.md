# ADR-0033 — La sintaxis de Varn: qué es, qué sobra y qué falta

Estado: aceptada. Decisiones D1–D6 en 5.1; la implementación sigue el plan de la sección 7.
Fecha: 2026-11-22

## 1. Qué lenguaje es Varn

Varn tiene la superficie de TypeScript y la semántica de un lenguaje estático compilado (Leyes 17, 20 y 21):

- Los tipos se resuelven por completo en compilación: el tipo estático es el tipo en runtime, sin `any`, sin `undefined` y sin búsquedas por nombre.
- La memoria tiene un modelo propio: `struct` son valores, las clases son referencias, `T?` es la única ausencia (ADR-0024), los enteros tienen ancho fijo (ADR-0015) y los layouts son compactos.
- Los errores son valores tipados (`Result<T, E>`, `try`) y excepciones con jerarquía (ADR-0022).

De ahí sale un criterio para cada construcción. Se escribe con la forma de TypeScript cuando TypeScript la tiene. Cuando Varn se aparta, se debe a que la semántica estática lo exige, y entonces el compilador enseña la forma de Varn en vez de fallar con un error genérico.

## 2. Cómo se obtuvo la evidencia

Se escribieron 87 programas mínimos, uno por construcción, y se ejecutaron con `vn run` sobre el binario actual (Ley 12). Se contrastaron con el corpus `tests/*.vn` (357 suites) y con el parser (`crates/vn-parser`), el lexer (`vn-core/src/token`) y el AST (`vn-core/src/ast`).

## 3. Lo que ya es coherente (se conserva)

| Construcción | Forma en Varn | Por qué es correcta |
|---|---|---|
| Igualdad | `==` / `!=` por valor; `===` da `VN2002` con la forma correcta | TS necesita `===` por la coerción de `==`, y Varn no tiene coerción. Kotlin, Swift, C# y Rust usan `==` |
| Variables | `let` / `const`; `var` rechazado con pista | `var` tiene un alcance de función que Varn no modela |
| Selección | `match (x) { p => e }`; `switch` rechazado con pista | Exhaustividad comprobada (`128-match-exhaustiveness`). Es la forma de Rust, Kotlin `when`, Swift `switch` y C# `switch` expresión |
| Ausencia | `T?`, `?.`, `??`, `??=`, `!` | Igual que TS, Kotlin, Swift y C# (ADR-0024) |
| Pruebas de tipo | `x is T`, predicados `x is T` | C#, Kotlin y Dart usan `is` |
| Errores como valor | `try expr` sobre `Result` y `T?` | Equivale al `?` de Rust y al `try` de Swift; se escribe delante, sin chocar con `?.` |
| Números | `int`, `float`, `i8`…`u64`, `f32`, `bigint` (`10n`), `decimal` (`1.5d`) | Ancho fijo y estático (ADR-0015, ADR-0016) |
| Tuplas | `(a, b)`, tipo `(int, str)`, `(x,)`, `()` | Igual que Swift, Rust y Python. Los arrays de Varn son homogéneos y tienen layout propio, así que no pueden ser tuplas |
| Valores | `struct` con campos, métodos y `static` | Igual que C#, Swift y Rust |
| Copia con cambios | `p with { x: 5 }` | Igual que C# `with` y que `copy` en Kotlin |
| Argumentos nombrados | `new P(y: 2, x: 1)` | Igual que C#, Kotlin, Swift y Dart |
| Recursos | `using r = …` + `Disposable` | Igual que TS 5.2 `using` y C# `using` |
| Extensiones | `extension [Nombre] on T { … }` | Igual que Dart `extension on`, C#, Kotlin y Swift |
| Rangos | `a..b`, `a..=b` | Igual que Rust y Kotlin |
| Pipeline | `x \|> f` | Propuesta de TC39 y F# |
| Descarte | `_ = f()` | Igual que C#, Dart y Rust |
| Herencia explícita | `override` obligatorio | Igual que C#, Kotlin y Swift; en TS es opcional (`noImplicitOverride`) |
| Diagnósticos de puente | `===`, `var`, `switch`, `delete`, firmas de índice | Señalan la forma de Varn |

## 4. Defectos encontrados (se corrigen sin decisión: Leyes 8, 14 y 15)

| # | Defecto | Evidencia | Corrección |
|---|---|---|---|
| F1 | `destructor { … }` se parsea y nunca se ejecuta | 200 000 instancias con destructor: ninguna lo ejecuta | Eliminar `destructor` (palabra clave, `ClassMember::Destructor` y sus ramas). La finalización determinista ya existe con `using` + `Disposable` (Ley 20: hay sustituto equivalente) |
| F2 | `export default function f()` produce un ICE `invalid TIR` | Prueba `default_export` | Soportarlo o rechazarlo en el parser con diagnóstico. Ver D6 |
| F3 | `const p: typeof o = …` falla con "a `const` must be initialized" | Prueba `typeof_ty` | El parser de tipos consume mal `typeof`; hay que repararlo |
| F4 | `Decl::SumType` / `SumTypeDecl` no se construyen en ningún sitio | Ningún constructor en el repo; solo ramas de `match` | Eliminar la variante y sus ramas (Ley 8). Los enums con payload ya cubren los tipos suma |
| F5 | `ExprKind::Spawn` y los tokens `Spawn`/`Parallel`/`Start` no son alcanzables | No están en la tabla de palabras clave; `spawn` es una función de `std:task` | Eliminarlos |
| F6 | `namespace` y `module` declaran lo mismo | `module M {}` funciona igual que `namespace` | Quitar `module` como declaración (TS lo marcó obsoleto). Una sola forma (Ley 8) |
| F7 | Formas de TS sin equivalente dan errores genéricos | `number`/`string`/`any`/`unknown`: "unknown type"; `undefined`: "undefined variable"; `instanceof`/`typeof x`: "expected `)`"; `[int, str]`: "unexpected `[` in a type"; sobrecargas: "expected `{`" | Una tabla canónica en `vn-core` (Ley 16) que, para cada forma de TS, diga la de Varn: `number`→`int`/`float`, `string`→`str`, `undefined`→`null`, `instanceof`→`is`, `[A, B]`→`(A, B)`, `any`/`unknown`→genéricos o uniones |
| F8 | Un patrón cualificado no compila | `match (s) { Shape.Circle(r) => … }`: "expected `=>`, found `(`". En expresión se exige `Shape.Circle(…)` y en patrón solo vale `Circle(r)` | El patrón acepta la forma cualificada, la misma que la expresión, y la corta se mantiene como abreviatura resuelta contra el tipo del escrutinio |
| F9 | Un literal de objeto no construye un `struct` | `const p: P = { x: 1, y: 2 }`: "expected 'P', got 'object'". Sí funciona con interfaces y records | Con un tipo contextual `struct`, el literal construye el valor. Es la forma de TS, resuelta en compilación como el inicializador de C# y el memberwise de Swift |

## 5. Distancias respecto a TypeScript que necesitan decisión

### D1 — Iteración: `for (x in xs)` frente a `for (x of xs)`

Hoy `for (const x in xs)` recorre valores y `of` está reservado (`VN2007`).

- En TS `for…in` recorre claves de objeto y `for…of` recorre valores. Un desarrollador de TS lee `for (const x in arr)` como índices.
- C#, Python, Dart, Swift, Kotlin y Rust usan `in` para valores.
- El `for…in` de TS es una trampa (claves en texto, prototipos) que Varn no tiene, así que la forma de TS más cercana a la semántica de Varn es `of`.

Recomendación: `for (const x of xs)` como única forma, con `in` rechazado y una pista. Es la Ley 21: la forma de TS existe y no choca con la semántica. Cambia el corpus y `std`; el cambio es mecánico.

### D2 — Tipo de retorno de `async`

Hoy valen `async function f(): int` y `async function f(): Task<int>`: dos escrituras de un mismo hecho (Ley 8).

- TS exige `Promise<T>` y C# `Task<T>`.
- Kotlin `suspend fun f(): Int`, Swift `async -> Int` y Rust `async fn -> i32` no envuelven el tipo; Dart escribe `Future<int> f() async`.
- En el sitio de llamada el valor es siempre `Task<T>`.

Recomendación: solo `Task<T>`. Es lo más cercano a TS y C#, y el tipo escrito coincide con el que recibe quien llama.

### D3 — Constructores primarios

Hoy valen `class User(public id: int) {}` y `constructor(public id: int) {}`, que son las propiedades de parámetro de TS.

- TS tiene la segunda forma. C# 12 y Kotlin tienen la primera.
- La Ley 21 prohíbe inventar sintaxis cuando TS ofrece una equivalente.

Recomendación: conservar solo las propiedades de parámetro de TS. Alternativa: conservar la forma primaria porque reduce texto, pero entonces esta distancia queda justificada por escrito y la otra forma se elimina.

### D4 — `match (x)` frente a `match x`

Hoy se aceptan ambas. Recomendación: solo `match (x)`, coherente con `if (x)` y `while (x)` y con la propuesta de TC39. El formateador normaliza el corpus.

### D5 — `dynamic` en código de usuario

Hoy `dynamic` se acepta en cualquier sitio (`93-match-guards` lo usa en predicados de tipo). La Ley 17 prohíbe lo dinámico por comodidad, y su uso legítimo es la frontera con el host.

Recomendación: `dynamic` solo en contratos y ficheros `declare`. El código de usuario usa uniones, genéricos o `JsonValue`.

### D6 — `export default`

TS lo tiene, pero nombra lo mismo de dos maneras: `default` y el nombre real. Ninguno de los seis lenguajes de referencia tiene una exportación anónima.

Recomendación: rechazarlo con una pista hacia `export function f`, porque una exportación es un nombre portable (Ley 2). Si se prefiere la cercanía con TS, se implementa como alias del nombre declarado.

### 5.1 Decisiones tomadas (2026-11-22)

- **D1: solo `in`.** `for (const x in xs)` recorre valores y `of` sigue reservado con la pista actual. Es la forma de C#, Python, Dart, Swift, Kotlin y Rust. La distancia con TS queda justificada porque Varn no tiene la iteración de claves de objeto que da su sentido al `for…in` de TS: en Varn no hay dos iteraciones que distinguir. No requiere cambios en el código.
- **D2: solo `Task<T>`.** `async function f(): Task<int>`; el tipo de retorno desnudo en una función `async` se rechaza con una pista que muestra la forma `Task<T>`. Hay que migrar el corpus y `std`.
- **D3: se conservan ambas formas de constructor.** `class User(public id: int)` y `constructor(public id: int)`, por decisión del propietario. La forma primaria es una comodidad de alto nivel (Ley 20) que se acepta como excepción explícita a la Ley 8; ambas deben producir el mismo AST de constructor.
- **D4: solo `match (x)`.** `match x {` se rechaza con una pista. El corpus tiene 77 usos con paréntesis y 5 sin ellos (tests 29 y 51), que se migran.
- **D5: `dynamic` se conserva como única frontera dinámica, acotada.** La std no lo usa (0 usos); los tests sí (210 usos en 75 ficheros, con suites dedicadas a la frontera: 162, 167, 169, 170, 171, 178 y 179). No se elimina (Ley 20). La regla es que un valor `dynamic` solo vuelve a un tipo estático con un `as T` validado en runtime; hay que comprobar que no existe ninguna otra vía implícita.
- **D6: `export default` se rechaza en el parser con una pista** hacia `export function f` + `import { f }`. Hoy produce un ICE (F2) y no tiene usos (0 en el corpus y la std). Una exportación es siempre un nombre portable (Ley 2).

## 6. Lo que se conserva sin cambios aunque pese

Los tipos condicionales, `infer`, los tipos mapeados, `keyof`, el acceso indexado, los tipos literal de plantilla y las intersecciones funcionan y se resuelven en compilación, así que no violan la Ley 17. Se conservan (Ley 20). El trabajo pendiente es comprobar que cada uno baja a un layout estático y no a `dynamic`.

## 7. Plan, en commits atómicos

1. F4, F5, F6: eliminar las variantes y los tokens muertos (`SumType`, `Spawn`, `module`).
2. F1: eliminar `destructor`.
3. F3: reparar `typeof` en posición de tipo.
4. F7: tabla canónica de equivalencias de TS en `vn-core` y diagnósticos.
5. F8: patrones de variante cualificados.
6. F9: literal de objeto que construye un `struct`.
7. D2: `async` exige `Task<T>`; migración del corpus y de `std` en el mismo commit.
8. D4: `match` exige paréntesis; migración de los 5 usos.
9. D6: rechazo de `export default` con pista (cierra F2).
10. D5: auditoría de que `dynamic` solo vuelve a un tipo estático mediante `as T`.
