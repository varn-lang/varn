# ADR-0034 — La sintaxis de Varn evaluada con la Ley 23

Estado: aceptada. Decisiones en la sección 4.1; la implementación sigue la sección 5.
Fecha: 2026-11-22

## 1. Criterio

La Ley 23 cambia la pregunta del ADR-0033. Ya no basta con saber si una forma de TypeScript se acepta: hay que comprobar qué significa. Una forma es correcta si su semántica es la de Varn y el tipo estático describe exactamente lo que pasa en runtime. Es una grieta si arrastra una semántica que TypeScript solo tiene por depender de JavaScript, o si se escribe igual que en TypeScript pero significa otra cosa sin avisar.

Evidencia: 136 programas de comportamiento ejecutados con `vn run` sobre el binario actual (Ley 12).

## 2. Lo que ya es Varn: TypeScript tiene la grieta y Varn el cimiento

| Forma | En TypeScript/JS | En Varn (verificado) |
|---|---|---|
| `if (n)` con `int`, `!n`, `n ? a : b` | Veracidad: `0`, `""` y `NaN` son falsos | VN3027: la condición es `bool` o nulable |
| `"a" + 1`, `1 + "a"`, `+"5"` | Coerción a texto o número | VN3010; se usa una plantilla o la conversión explícita |
| `1 == "1"`, `1 < "2"` | Coerción en la comparación | VN3010 |
| `7 / 2` | `3.5` | `3`, división entera (ADR-0023) |
| `1 / 0` entero | `Infinity` | `DivisionByZero` |
| `int + float` | Mismo tipo `number` | VN3010: la conversión es explícita |
| Desbordamiento | Pérdida de precisión silenciosa | `IntegerOverflow` |
| `a[5]` fuera de rango, `a[-1]` | `undefined` | `RangeError` |
| `[10, 9, 1, 2].sort()` | Orden lexicográfico: `[1, 10, 2, 9]` | `[1, 2, 9, 10]` |
| `indexOf` sin resultado | `-1` | `null` |
| `"😀".length` | `2` (UTF-16) | `1` (puntos de código) |
| `throw "boom"`, `throw 42` | Se lanza cualquier valor | VN4008: solo `Error` |
| `undefined`, `globalThis`, `parseInt` | Existen | No existen |
| `o.b` inexistente, `o.b = 2` sobre un literal | `undefined` o propiedad nueva | VN3004 |
| `const o = {..}; o.a = 2` | Muta | VN3005: un registro es un valor |
| `new A() == new A()` | Identidad sin aviso | VN3010: la igualdad exige tipos valor; `sameValue` para identidad |
| `{ x: 1 } == { x: 1 }` | `false` | `true`: los registros son valores |
| `x!` sobre `null` | Mentira silenciosa | Error en runtime con mensaje |
| `x as int` falso | Mentira silenciosa | Conversión comprobada |
| `a.get` extraído, `this` en flechas | `this` dinámico | Método enlazado, `this` léxico |
| Variable de bucle capturada | Compartida con `var` | Una por iteración |
| Clase asignada a otra clase con la misma forma | Aceptado (estructural) | VN3001: las clases son nominales |
| `Task` sin `await` | La promesa corre igual | VN3031: una tarea solo corre al esperarla (fría) |
| Genéricos | Borrados en runtime | `x is int` sobre `T` funciona |

## 3. Defectos: se corrigen sin decisión (Leyes 15 y 23)

| # | Defecto | Evidencia | Corrección |
|---|---|---|---|
| B1 | `n == 0` con `n: int? = null` revienta en runtime con "convert: value is not numeric" | `t6` | Una comparación con un nulable compara la ausencia primero (`null == 0` es `false`) en vez de desempaquetar |
| B2 | `const a: int[] = [1, , 3]` crea un `int[]` con un `null` dentro: el tipo miente | `t11` | Los huecos de array son un resto de los arrays dispersos de JS; se rechazan con pista de `ForeignForm` |
| B3 | `print(a.push(2))` usa una llamada `void` como valor y la imprime como `null` | `arr_push_return` | VN3022 también en posición de argumento |
| B4 | `#s` (campo privado de JS) se declara pero no se puede leer | `t12` | Se rechaza con pista hacia `private` |
| B5 | `catch` sin tipo y `as` fallido dan mensajes de conversión internos | `as_wrong` | El fallo de `as T` lanza `TypeError` con el tipo esperado y el recibido |
| B6 | `parseInt`, `Object.keys` y `globalThis` dan "undefined variable" | — | Entradas de `ForeignForm`: `int.parse`, los campos de un registro tipado, sin equivalente |

## 4. Grietas que necesitan decisión

### D7 — Veracidad de los nulables

Hoy `if (s)` con `s: str? = ""` toma la rama "present", y `if (n)` con `n: int? = 0` también. Es la sintaxis de JS con el resultado contrario: en JS `""` y `0` son falsos. Lo mismo pasa con `!n`, `s || "x"` (devuelve `""`, mientras que JS devolvería `"x"`) y `a && 5` (devuelve `5`): `&&`, `||` y `!` devuelven el operando en vez de un `bool`.

- Kotlin, Swift, C#, Dart y Rust no tienen veracidad: la condición es `bool` y la ausencia se comprueba explícitamente (`!= null`, `if let`, `is Some`).
- Python tiene veracidad, pero con la semántica de JS (`""` es falso).

Recomendación: la condición y los operandos de `&&`, `||` y `!` son siempre `bool`. La ausencia se comprueba con `x != null` (y el estrechamiento ya existe), y el valor por defecto se escribe con `??`. Así desaparece una sintaxis que promete el comportamiento de JS y hace otra cosa.

### D8 — Interfaces estructurales o nominales

Hoy una clase satisface una interfaz sin declararlo: `f(new B())` acepta `B` donde se espera `I` porque tiene `x: int`.

- Go y TypeScript son estructurales. C#, Kotlin, Swift, Dart y Rust (traits) son nominales: la clase declara lo que implementa.
- Varn ya es nominal entre clases (VN3001) y estructural para registros, que son valores.

Recomendación: una interfaz describe la forma de un registro (estructural) y es un contrato que una clase declara con `implements` (nominal). Una clase solo es asignable a una interfaz que implementa. El corpus ya declara `implements` 31 veces.

### D9 — La maquinaria de tipos de TypeScript

Varn acepta tipos condicionales, `infer`, tipos mapeados, `keyof`, acceso indexado, tipos de plantilla e intersecciones. Existen en TypeScript para tipar objetos dinámicos de JS, y en Varn están a medias: `keyof P` existe pero `p[k]` no compila (sin indexado por nombre). Uso en el corpus y en la std: `keyof` 0, tipos mapeados 0, condicionales 0, `infer` 1.

Recomendación: retirarlos. Las intersecciones de interfaces se conservan porque componen contratos. Las uniones de literales también, porque son enums ligeros. Es la excepción que permite la Ley 20: el sustituto es el tipado nominal y genérico, con el mismo poder sobre datos estáticos.

### D10 — La asignación como expresión

Hoy `a = b = 3`, `let b = (a = 5)` y `i++ + ++i` son expresiones con valor.

- Swift, Kotlin, Python y Rust hacen de la asignación una sentencia; C# y Java la mantienen como expresión.
- Swift eliminó `++` y `--` (SE-0004); Kotlin los mantiene.

Recomendación: la asignación y `++`/`--` son sentencias. `i++` sigue valiendo como sentencia y en el paso de un `for`, pero no dentro de otra expresión. No hay usos encadenados en el corpus.

### D11 — `function` como expresión

`const f = function () { return this.x }` dentro de un método devuelve `this.x` del método: Varn lo trata como una flecha (`this` léxico), y en JS tendría su propio `this`. Es la misma sintaxis con otra semántica, y hay dos formas para lo mismo. Uso: 2 casos.

Recomendación: retirar la expresión `function`; las funciones anónimas se escriben como flechas. Las declaraciones `function f() {}` se conservan.

### 4.1 Decisiones tomadas (2026-11-22)

- **D7: solo `bool`.** Las condiciones de `if`, `while`, `?:` y los operandos de `&&`, `||` y `!` son `bool`. La ausencia se comprueba con `x != null`, que estrecha el tipo, y el valor por defecto se escribe con `??`. Usar un nulable como condición da un error con pista.
- **D8: registros por forma, clases por `implements`.** Un registro encaja en una interfaz por su forma. Una clase solo es asignable a una interfaz que declara con `implements`, directamente o por herencia.
- **D9: se retira la maquinaria de tipos de TypeScript:** tipos condicionales, `infer`, tipos mapeados, `keyof`, acceso indexado y tipos de plantilla, con pista de `ForeignForm`. Se conservan las intersecciones de interfaces y las uniones de literales.
- **D10: la asignación y `++`/`--` son sentencias.** No producen valor; `i++` vale como sentencia y en el paso de un `for`.
- **D11: se conservan las expresiones `function`**, por decisión del propietario. Su `this` es léxico, igual que en una flecha, y esa distancia con JS queda documentada aquí.

## 5. Plan

1. B1–B6, un commit cada uno, con fixture o test.
2. D7: condiciones booleanas, con migración del corpus y de la std.
3. D8: clases nominales frente a interfaces.
4. D10: asignación e incremento como sentencias.
5. D9: retirada de la maquinaria de tipos.

## 6. Estado de la implementación (2026-11-22)

| Punto | Estado | Commit |
|---|---|---|
| B1 igualdad con nulables | Corregido; el test 357 fijaba el defecto | `2595bb7c` |
| B2 huecos de array | Rechazados en literales y en asignaciones | `00925fd0` |
| B2 `_` en declaraciones | Descarta en vez de ligar | `59ba8671` |
| B3 llamada `void` como argumento | VN3022 | `a0f881a5` |
| B4 `#privado` | Rechazado | `ef9f887e` |
| B5 `as` fallido | `TypeError` con el tipo recibido | `2f2c72f7` |
| B6 globales de JS | Pista de `ForeignForm` | `be49ddaf` |
| D7 condiciones `bool` | Hecho; `dynamic` tampoco es condición y el runtime ya no tiene veracidad | `f8737870`, `194018ec`, `dc8bf09f` |
| D8 clases por `implements` | Hecho para interfaces con nombre | `a0610203` |
| D10 asignación como sentencia | Hecho | `fc63e2a8` |
| D9 maquinaria de tipos | Hecho, tras las extensiones con patrón de receptor (ADR-0035) | `9fbf3b68` |

### 6.1 Bloqueos y pendientes

- **D8.** Una interfaz genérica instanciada se expande a una forma anónima antes de compararse y sigue siendo estructural. Hacerla nominal exige que el tipo conserve su interfaz de origen.
- **Heredado del ADR-0033 (D5).** Auditoría de `dynamic`: cada uso debe estar en una frontera con el host o desaparecer.
- **Heredado del ADR-0033.** El emisor resuelve variantes con `match_variant_by_name` cuando no conoce el enum del sujeto: es una búsqueda por nombre en runtime (Ley 17) que se elimina resolviendo el enum en compilación.
- **Heredado del ADR-0033.** `vn-vm/src/exec/compare.rs` tiene comodines `_ =>` (Ley 7), y `class_key.rs` un `_ if` con guarda.
