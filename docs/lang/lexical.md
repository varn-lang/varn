# Varn Language — Lexical Syntax

> Fuentes: `tests/arithmetic.test.vn`, `tests/boolean.test.vn`, `tests/strings.test.vn`, `tests/templates.test.vn`, `tests/char-type.test.vn`, `tests/decimal-bigint.test.vn`, `tests/raw-strings-and-with-expressions.test.vn`.

---

## 1. Identificadores

Los identificadores siguen la convención *camelCase* (variables, funciones) y *PascalCase* (clases, interfaces, enums). Reglas:

- Deben comenzar con letra o guión bajo (`_`).
- Pueden contener letras Unicode, dígitos y `_`.
- El guión bajo solitario (`_`) actúa como identificador de descarte en destructuring.

```vn
let count = 0
const _ignored = 42
const [head, _] = [1, 2]
```

---

## 2. Palabras Reservadas

Lista exacta del lexer (`crates/vn-lexer/src/keywords.rs:12-78`):

```
let  const  var  function  class  struct  interface  type  enum
namespace  module  extension  on  if  else  switch  case  default
while  for  do  break  continue  return  throw  try  catch  finally
using  with  import  export  from  as  async  await  yield
new  this  super  delete  typeof  instanceof  in  of  void  is
true  false  null  public  private  protected  static  abstract
override  readonly  declare  native  extends  implements
get  set  constructor  destructor  match
```

NO son keywords: `record`, `spawn`, `parallel`, `dynamic`, `meta` (en `import.meta`).

---

## 3. Comentarios

```vn
// Comentario de una sola línea

/* Comentario
   multilínea */
```

---

## 4. Literales

### 4.1 Literales enteros (`int`)

```vn
assertTrue("int add",      1 + 2 === 3)       // tests/arithmetic.test.vn
assertTrue("int power",    2 ** 10 === 1024)  // tests/arithmetic.test.vn
assertTrue("bitwise and",  (12 & 10) === 8)   // tests/arithmetic.test.vn
assertTrue("left shift",   1 << 4 === 16)     // tests/arithmetic.test.vn
```

Soporta notación decimal. Las operaciones son: `+`, `-`, `*`, `/`, `%`, `**` (potencia), `&`, `|`, `^`, `~`, `<<`, `>>`.

### 4.2 Literales flotantes (`float`)

```vn
assertTrue("float add",  0.1 + 0.2 > 0.29 && 0.1 + 0.2 < 0.31)  // tests/arithmetic.test.vn
assertTrue("float div",  1.0 / 3.0 > 0.333 && 1.0 / 3.0 < 0.334)  // tests/arithmetic.test.vn
```

Requiere punto decimal. `int / int` es `int` truncado
(`tests/int-division.test.vn`); la división `float` usa operandos `float`
(`tests/int-division.test.vn`).

### 4.3 Literales `decimal`

Sufijo `d`. Precisión exacta para cálculos financieros.

```vn
const d1: decimal = 1.5d    // tests/decimal-bigint.test.vn
const d2: decimal = 2.5d
assertTrue("decimal add", d1 + d2 === 4.0d)   // tests/decimal-bigint.test.vn
assertTrue("decimal toFixed 2", (3.14159d).toFixed(2) === "3.14")  // tests/decimal-bigint.test.vn
```

### 4.4 Literales `bigint`

Sufijo `n`. Enteros de precisión arbitraria.

```vn
const bi1: bigint = 100n     // tests/decimal-bigint.test.vn
assertTrue("bigint toString", bi1.toString() === "100")
assertTrue("bigint toInt",    bi1.toInt() === 100)
assertTrue("bigint lt",       1n < 100n)
```

### 4.5 Literales booleanos (`bool`)

```vn
assertTrue("and short-circuit", (false && (1/0 === 0)) === false)  // tests/boolean.test.vn
assertTrue("or short-circuit",  true  || (1/0 === 0) === true)    // tests/boolean.test.vn
assertTrue("not false",         !false === true)                   // tests/boolean.test.vn
assertTrue("triple eq null",    null === null)                     // tests/boolean.test.vn
```

### 4.6 Literales de cadena (`str`)

Entre comillas dobles. Admiten secuencias de escape estándar: `\n`, `\t`, `\r`, `\\`, `\"`.

```vn
const s = "  Hello, World!  "
assertTrue("trim",       s.trim() === "Hello, World!")   // tests/strings.test.vn
assertTrue("slice",      "hello world".slice(6, 11) === "world")
assertTrue("length",     "hello".length === 5)
```

### 4.7 Raw String Literals (`"""…"""`)

Cadenas delimitadas por triple comilla. No procesan secuencias de escape; el contenido se preserva tal cual.

```vn
let jsonStr = """{
  "name": "Varn",
  "path": "C:\Program Files\Varn",
  "escapes": "\n \t \r"
}""";
// tests/raw-strings-and-with-expressions.test.vn

let text = """Hello "world" and 'quotes' without escapes!""";
// tests/raw-strings-and-with-expressions.test.vn
```

- Pueden ser multilínea sin barras invertidas adicionales.
- Permiten comillas dobles internas sin escape.
- El tipo resultado es `str`.

### 4.8 Template Literals (`` ` `` `` ` ``)

```vn
const tval = 42
assertTrue("simple template",   `answer = ${tval}` === "answer = 42")   // tests/templates.test.vn
assertTrue("expr in template",  `${tval * 2}` === "84")                  // tests/templates.test.vn
assertTrue("nested template",   `prefix_${"mid"}_suffix` === "prefix_mid_suffix")
assertTrue("template + concat", `a${1 + 1}b` + "c" === "a2bc")
```

Las interpolaciones `${expr}` admiten cualquier expresión.

### 4.9 Literales de carácter (`char`)

Entre comillas simples. Tipo primitivo distinto de `str`.

```vn
const a: char = 'a'             // tests/char-type.test.vn
assertTrue("char code point", 'A'.charCodeAt() === 65)
assertTrue("char from code",  char.fromCode(65) === 'A')
assertTrue("char toString",   'x'.toString() === "x")
```

### 4.10 Literal `null`

```vn
const p3: Profile? = null    // tests/null-safety.test.vn
assertTrue("triple eq null", null === null)
```

### 4.11 Literales de expresión regular (`Regex`)

```vn
const fecha = /(\d{4})-(\d{2})/       // tests/regex-literals.test.vn
const m = fecha.exec("el 2026-10")
const ok = /^ab+c$/i.test("ABBC")
```

- Un literal `/cuerpo/flags` tiene tipo `Regex`, un tipo del núcleo como `Array` o `Range`: no se importa.
  `new Regex(patrón, flags?)` es la otra forma de construirlo (lanza `ParseError` si el patrón es inválido).
- `str.replace`, `str.replaceAll` y `str.split` aceptan un `Regex` donde aceptan un `str`: `replace` reescribe
  la primera coincidencia, `replaceAll` todas, y `$1`, `${nombre}` y `$$` expanden grupos en el reemplazo.
- Un módulo que declara su propia `Regex` no puede usar literales (`VN2021`): el literal construiría esa clase.
- El cuerpo y los flags se validan al **parsear**: un patrón inválido, un flag desconocido o una
  construcción que el motor no admite (retroreferencias, lookaround) es el error `VN2021`.
- Flags admitidos: `i`, `m`, `s`, `x`. No existe `g`: `findAll` y `replace` son siempre globales.
- Cada patrón con sus flags se construye **una vez por módulo**, al arrancar el módulo; dos literales
  iguales comparten la misma instancia.
- El motor es lineal (sin retroceso); `index` de una coincidencia cuenta puntos de código, igual que
  `indexOf` y `substring`.
- `/` es división después de un valor (identificador, literal, `)`, `]`, `}`); en cualquier otra
  posición empieza una expresión regular.

---

## 5. Operadores

### 5.1 Aritméticos
| Operador | Descripción |
|----------|-------------|
| `+` | Suma / concatenación de strings |
| `-` | Resta / negación unaria |
| `*` | Multiplicación |
| `/` | División: `int / int` es `int` truncado hacia cero (`tests/int-division.test.vn`); `float / float` es `float` |
| `%` | Módulo |
| `**` | Potencia |

### 5.2 Bit a bit
`&`, `|`, `^`, `~`, `<<`, `>>`

### 5.3 Comparación
`===`, `!==`, `<`, `<=`, `>`, `>=`  
`==` realiza igualdad estructural profunda (tuplas, records).

### 5.4 Lógicos
`&&`, `||`, `!`  
Con cortocircuito: `&&` no evalúa el lado derecho si el izquierdo es falso; `||` no evalúa si el izquierdo es verdadero.

### 5.5 Null-coalescing
```vn
const maybe: int? = 42
assertTrue("?? left non-null", (maybe ?? 0) === 42)  // `??` sobre un no-nulo es error (VN3028)
assertTrue("?? left null",     (null ?? 99) === 99)   // tests/null-safety.test.vn
assertTrue("?? chained",       (null ?? null ?? 7) === 7)
```

### 5.6 Null-coalescing Assignment (`??=`)
```vn
let a: int? = null
a ??= 42          // a es ahora 42  (tests/primary-constructors-nullish-assign.test.vn)
a ??= 999         // no cambia: a sigue siendo 42
```

Asigna el valor derecho únicamente si la variable (o propiedad, o índice) es `null`.

### 5.7 Optional Chaining (`?.`)
```vn
assertTrue("?. non-null chain",  p1?.address?.city === "NY")   // tests/null-safety.test.vn
assertTrue("?. missing nested",  p2?.address?.city === null)
assertTrue("?. null root",       p3?.name === null)
```

### 5.8 Pipeline (`|>`)
```vn
assertTrue("pipe simple",       5 |> double2 === 10)             // tests/pipeline.test.vn
assertTrue("pipe placeholder",  7 |> addN(_, 3) === 10)          // tests/pipeline.test.vn
assertTrue("pipe multi _",      4 |> addN(_, _) === 8)
const piped = (3 |> double2) |> double2
assertTrue("pipe chained", piped === 12)
```

El `_` en el lado derecho actúa como marcador de posición del valor del lado izquierdo.

### 5.9 Ternary
```vn
function classify(n: int): str {
    return n < 0 ? "neg" : n === 0 ? "zero" : "pos"  // tests/control-flow.test.vn
}
```

### 5.10 Spread (`...`)
```vn
const spread = [...[1, 2], ...[3, 4]]          // tests/arrays.test.vn
const ext    = { ...base, z: 3 }               // tests/destructuring.test.vn
```

### 5.11 Instanceof y typeof
```vn
assertTrue("instanceof PoliceDog", pd instanceof PoliceDog)  // tests/classes.test.vn
assertTrue("processTypeof string", typeof "abc" === "str")   // tests/control-flow-narrowing.test.vn
```
// `typeof` verificado: `"str"`, `"int"`, `"float"`, `"class"` (ver `identifiers.md` §6).

### 5.12 Type Assertion (`is`)
```vn
function describeChar(c: char): str {
    if (c is char) return "yes"   // tests/char-type.test.vn
    return "no"
}
```

---

## 6. Secuencias de Escape en cadenas `str`

> NO-VERIFICADO como tabla: ningún test fija el conjunto exacto.

| Secuencia | Carácter |
|-----------|---------|
| `\n` | Nueva línea |
| `\t` | Tabulador |
| `\r` | Retorno de carro |
| `\\` | Barra invertida |
| `\"` | Comilla doble |
| `\'` | Comilla simple |

En **Raw Strings** (`"""…"""`), ninguna de estas secuencias se procesa; el texto es literal.

