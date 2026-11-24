# Varn Language — Expresiones

> Fuentes: `tests/arithmetic.test.vn`, `tests/templates.test.vn`, `tests/arrays.test.vn`, `tests/destructuring.test.vn`, `tests/closures.test.vn`, `tests/null-safety.test.vn`, `tests/match.test.vn`, `tests/generics.test.vn`, `tests/ranges.test.vn`, `tests/pipeline.test.vn`, `tests/option-destructuring.test.vn`, `tests/complex-closures.test.vn`, `tests/advanced-higher-order.test.vn`, `tests/advanced-enums.test.vn`, `tests/tuples-structs.test.vn`, `tests/tagged-templates.test.vn`, `tests/primary-constructors-nullish-assign.test.vn`, `tests/range-indexing.test.vn`, `tests/raw-strings-and-with-expressions.test.vn`.

---

## 1. Expresiones Literales

Ver [`lexical.md`](lexical.md) para la especificación completa de literales.

```vn
42          // int
3.14        // float
1.5d        // decimal
100n        // bigint
true        // bool
'a'         // char
"hello"     // str
"""raw"""   // raw string
null        // null
```

---

## 2. Variables

```vn
let x = 42          // mutable
const y = "hello"   // inmutable (alias: const)
// `var` no existe: el compilador lo rechaza (VN2002)

let typed: int = 100
const explicit: str = "world"
```

---

## 3. Operadores y Precedencia

> NO-VERIFICADO: la tabla completa no está fijada por ningún test. Solo esto:
> `2 + 3 * 4 === 14`, `(2 + 3) * 4 === 20` (`tests/arithmetic.test.vn`).

| Nivel | Operadores |
|-------|-----------|
| 1 | `()` (agrupación), `[]`, `.`, `?.`, `!` |
| 2 | `**` (exponenciación, derecha-a-izquierda) |
| 3 | `-` (unario), `~`, `!` |
| 4 | `*`, `/`, `%` |
| 5 | `+`, `-` |
| 6 | `<<`, `>>` |
| 7 | `&` |
| 8 | `^` |
| 9 | `\|` |
| 10 | `<`, `<=`, `>`, `>=` |
| 11 | `===`, `!==`, `==`, `!=` |
| 12 | `&&` |
| 13 | `\|\|` |
| 14 | `??` |
| 15 | `?:` (ternario) |
| 16 | `\|>` (pipeline) |
| 17 | `=`, `??=` |

```vn
assertTrue("precedence *+",    2 + 3 * 4 === 14)    // tests/arithmetic.test.vn
assertTrue("precedence parens",(2 + 3) * 4 === 20)
```

---

## 4. Literals de Array

```vn
const nums = [3, 1, 4, 1, 5, 9]     // tests/arrays.test.vn
const spread = [...[1, 2], ...[3, 4]]   // spread
let explicitGeneric: Array<int> = [10, 20, 30]

// Acceso por índice
assertTrue("index", nums[0] === 3)

// Acceso por rango
let slice = nums[1..4]    // [1, 4, 1]  (tests/range-indexing.test.vn)
let sliceInc = nums[1..=4]
```

---

## 5. Literals de Objeto

```vn
const obj1 = { a: 1, b: 2, c: 3 }    // tests/destructuring.test.vn
const base = { x: 1, y: 2 }
const ext  = { ...base, z: 3 }        // spread en objeto
```

---

## 6. Tuplas y Structs (Inmutables)

```vn
struct Release { name: str; v: int }
let t = (1, "hello", true)       // Tuple  (tests/tuples-structs.test.vn)
let r = new Release("Varn", 1)   // Struct

// Igualdad estructural profunda con ==
let t2 = (1, "hello", true)
assertTrue("tuple eq", t == t2)
let r2 = new Release("Varn", 1)
assertTrue("struct eq", r == r2)
```

---

## 7. Llamadas a Funciones y Métodos

```vn
// Positional
assertTrue("int add", 1 + 2 === 3)
identity<int>(42)        // con parámetro de tipo explícito

// Named arguments
describe(name: "Alice", age: 30, city: "London")   // tests/named-arguments.test.vn
describe(age: 25, name: "Bob")                      // fuera de orden
describe("Charlie", age: 40)                         // mezcla posicional y nombrado

// Method chaining
new Builder().add("hello").add("world").build()
```

---

## 8. Destructuring

### De objetos

```vn
const obj1 = { a: 1, b: 2, c: 3 }
const { a, b } = obj1                         // extracción directa
const { a: renamed } = obj1                   // renombrado
const { name = "default", age = 0 } = { name: "Alice" }  // default values
const { point: { x: nx, y: ny } } = nested    // nested destructuring
```

### De arrays

```vn
const [f1, f2, f3] = [10, 20, 30]
const [skip1, _, skip3] = [100, 200, 300]    // _ descarta elemento
const [head, ...tail] = [1, 2, 3, 4]         // rest
const { coords: [cx, cy] } = { coords: [5, 6] }  // array en objeto
```

### De tuplas

```vn
const (code, label) = (200, "OK")            // los paréntesis desestructuran una tupla
const [a, b] = (1, 2)                         // error VN3001: una tupla se desestructura con (a, b)
```

### Asignación por desestructuración

El lado izquierdo de `=` puede ser un literal de array, tupla u objeto cuyas hojas sean variables,
miembros o índices ya declarados (`tests/destructuring-assignment.test.vn`). El lado derecho se evalúa
una sola vez y el valor de la expresión es ese lado derecho.

```vn
let a = 1
let b = 2
;(a, b) = (b, a)                              // intercambio
;[first, ...rest] = list                      // rest
;({ x, y: renamed } = point)                  // objeto, con renombrado (entre paréntesis como sentencia)
;(this.x, this.y) = (px, py)                  // miembros; cuenta como asignación del campo en el constructor
;[a, b = 9] = [1, null]                       // default cuando el elemento es null
```

Las reglas son las de la declaración: `(a, b)` exige una tupla de esa aridad (`VN3016`), `[a, b]` exige un array
(`VN3001`), cada hoja debe aceptar el tipo del elemento (`VN3001`) y no se puede reasignar una constante (`VN3005`).

---

## 9. Closures y Lambdas

```vn
// Lambda expresión
const double = (n: int) => n * 2
const inc    = (n: int) => n + 1

// Lambda con bloque
const square = (n: int) => {
    return n * n
}

// Closure captura variables del ámbito
function makeAdder(n: int): (a: int) => int {
    return (x: int) => x + n   // captura 'n'
}
const add5 = makeAdder(5)
assertTrue("closure add5", add5(3) === 8)   // tests/closures.test.vn

// Closure mutable
function makeCounter(): () => int {
    let c = 0
    return () => {
        c = c + 1
        return c
    }
}
const cnt1 = makeCounter()
assertTrue("counter1 first",  cnt1() === 1)
assertTrue("counter1 second", cnt1() === 2)
```

### Composición de funciones

```vn
function compose<A, B, C>(f: (a:B) => C, g: (b:A) => B): (c:A) => C {
    return (x: A) => f(g(x))
}
const doubleInc = compose(double, inc)
assertTrue("compose", doubleInc(4) === 10)   // tests/closures.test.vn
```

---

## 10. Match Expressions

```vn
function describeNum(n: int): str {
    return match (n) {        // tests/match.test.vn
        0 => "zero",
        1 => "one",
        2 | 3 => "two or three",
        _ => "other"
    }
}
assertTrue("match 2",     describeNum(2) === "two or three")
assertTrue("match other", describeNum(99) === "other")
```

Match sobre enums con payload (`tests/advanced-enums.test.vn`):

```vn
enum R { A(int), B(str) }

let x = R.A(42)
let v = match x {
    A(n) => n,
    B(_) => -1,
}
assertTrue("match ok", v === 42)
```

Match en métodos de enum:

```vn
area(): float {
    return match (this) {
        Circle(radius)           => 3.14159 * radius * radius,
        Rectangle(width, height) => width * height
    }
}
```

---

## 11. Pipeline Operator (`|>`)

```vn
function double2(n: int): int { return n * 2 }
function addN(n: int, x: int): int { return n + x }

assertTrue("pipe simple",      5 |> double2 === 10)            // tests/pipeline.test.vn
assertTrue("pipe placeholder", 7 |> addN(_, 3) === 10)
assertTrue("pipe multi _",     4 |> addN(_, _) === 8)
assertTrue("pipe clamp lo",    -5 |> clamp2(0, 10, _) === 0)

const piped = (3 |> double2) |> double2
assertTrue("pipe chained", piped === 12)
```

---

## 12. `with` Expressions

```vn
let u1 = new User(10, "Alice", "admin")
let u2 = u1 with { name: "Bob", role: "editor" }  // tests/raw-strings-and-with-expressions.test.vn
```

Ver [`classes_objects.md`](classes_objects.md) §9 para detalles completos.

---

## 13. Null Coalescing (`??`) y Optional Chaining (`?.`)

```vn
assertTrue("?? left null",     (null ?? 99) === 99)      // tests/null-safety.test.vn
assertTrue("?. non-null",      p1?.address?.city === "NY")
assertTrue("?. + ??",          (p3?.name ?? "anon") === "anon")
assertTrue("?? chained",       (null ?? null ?? 7) === 7)
```

---

## 14. Null-Coalescing Assignment (`??=`)

```vn
let a: int? = null
a ??= 42     // asigna solo si es null (tests/primary-constructors-nullish-assign.test.vn)
a ??= 999    // no hace nada: a ya es 42

let cfg = new AppConfig()
cfg.timeout ??= 3000     // propiedad de objeto
cfg.timeout ??= 8000     // no cambia

let items: dynamic[] = [null, "existing", null]
items[0] ??= "filled_0"  // índice de array
items[1] ??= "overwritten_1"   // no cambia: "existing"
```

---

## 15. Template Literals y Tagged Templates

```vn
// Interpolación estándar
const tval = 42
assertTrue("template", `answer = ${tval}` === "answer = 42")  // tests/templates.test.vn

// Tagged template: función + backtick
function customTag(strings: str[], ...values: dynamic[]): str {
    let res = ""
    for (let i = 0; i < strings.length; i = i + 1) {
        res = res + strings[i]
        if (i < values.length) { res = res + "[" + values[i] + "]" }
    }
    return res
}
let name = "Varn"
let version = 1
let output = customTag`Language: ${name}, Version: ${version}!`
assertTrue("tagged template", output === "Language: [Varn], Version: [1]!")   // tests/tagged-templates.test.vn

// SQL parametrizado con tagged templates
let rows = db.sql`SELECT * FROM users WHERE role = ${targetRole} AND active = ${isActive}`   // tests/tagged-templates.test.vn
```

---

## 16. `instanceof` y `typeof`

```vn
assertTrue("instanceof Dog",  pd instanceof Dog)           // tests/classes.test.vn
assertTrue("instanceof Error", ve instanceof Error)        // tests/errors.test.vn

// typeof verificado: "str", "int" (tests/control-flow-narrowing.test.vn),
// "float" (tests/numeric-conversions.test.vn), "class" (tests/network-apis-overhaul.test.vn)
```

---

## 17. `is` (Type Assertion / Narrowing)

```vn
function describeChar(c: char): str {
    if (c is char) return "yes"   // tests/char-type.test.vn
    return "no"
}
```
// Otros usos de `is`: NO-VERIFICADOS.

---

## 18. Cast Explícito (`as`)

```vn
let jsonObj = JSON.parse("{\"message\": \"hello\", \"status\": 200}")
let status = (jsonObj as dynamic).status    // tests/std-csv.test.vn
assertEqual(status, 200)
```

---

## 19. `new` — Creación de Instancias

```vn
const map = new Map<str, int>()         // tests/map-set.test.vn
const intBox = new Box<int>(10)         // tests/generics.test.vn
const b = new Box(1)                    // deduce Box<int> (tests/generic-instantiation-from-new.test.vn)
const u1 = new User(1, "Alice", "admin")   // tests/primary-constructors-nullish-assign.test.vn
const db = new Database(":memory:")     // tests/stdlib-sqlite-ws-archive.test.vn
```

---

## 20. Operador Ternario

```vn
const label = n < 0 ? "neg" : n === 0 ? "zero" : "pos"  // tests/control-flow.test.vn
```

---

## 21. Spread (`...`) en Expresiones

```vn
const spread = [...[1, 2], ...[3, 4]]          // array spread
const ext    = { ...base, z: 3 }               // object spread
function rolesDeco(...r: str[]) { /* rest params */ }   // rest parameters
```

