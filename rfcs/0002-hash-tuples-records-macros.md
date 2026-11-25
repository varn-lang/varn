# RFC 0002: tuplas `#[...]`, records `#{...}` y macros `#nombre(...)`

- Estado: draft
- Fecha: 2026-11-25

## Resumen

Las tuplas pasan de `(a, b)` a `#[a, b]` y los records anónimos de `{ x: 1 }` a `#{ x: 1 }`. `()` vuelve a ser solo agrupación y `{ }` en expresión sigue siendo literal de `struct`. El prefijo `#` queda reservado para formas especiales: `#[...]` tupla, `#{...}` record, `#nombre(...)` macro futura.

## Motivación

`(...)` hoy significa tres cosas: agrupación, tupla y lista de parámetros. `(x,)` es tupla de uno, `(x)` es agrupación, `()` es unidad, `(A, B)` es tipo tupla y `(x: int) => str` es tipo función. El parser distingue por coma y los humanos por contexto. Lo mismo pasa con `{...}`: bloque, objeto, literal de `struct` y tipo record.

```vn
const t = (1, "a")
const g = (1 + 2) * 3
const u: () = ()
function f(pair: (int, str)): (str, int) { return (pair[1], pair[0]) }
```

El coste es estático: cada `(` exige lookahead para saber si es tupla, cada `{` en tipo exige distinguir record de bloque, y los mensajes de error sugieren `(A, B)` cuando TypeScript escribe `[A, B]`.

## Diseño

### Tuplas

Valor, tipo, patrón y match usan la misma forma:

```vn
const t: #[int, str] = #[1, "a"]
const u: #[] = #[]
const #[a, b] = t
function swap(pair: #[int, str]): #[str, int] { return #[pair[1], pair[0]] }
match (t) { #[1, x] => x, _ => "no" }
```

Reglas:

- `#[ ]` vacío es la unidad. No hay sintaxis de un elemento con coma: `#[x]` ya es tupla de uno, `[x]` sigue siendo array de uno.
- `#[...xs]` admite spread en valor: cada `...x` es `ExprKind::Spread`.
- `(e)` es solo `Paren`. `(a, b)` en expresión, tipo o patrón es error con sugerencia a `#[a, b]`. `()` en valor o tipo es error con sugerencia a `#[]`; `() => T` sigue siendo tipo función sin parámetros.
- `match (x)` exige paréntesis simples: `match (#[a, b])` para sujeto tupla, nunca `match (a, b)`.
- TypeScript escribe tuplas `[A, B]` y arrays `T[]`. Varn no puede porque `[]` ya es array y `[A, B]` sería ambiguo en posición de tipo. El `#` desambigua sin salir de la familia de corchetes.

### Records

```vn
const r: #{ x: int, y: int } = #{ x: 10, y: 20 }
const #{ x, y } = r
match (r) { #{ x: 1 } => "uno", _ => "otro" }
```

Reglas:

- `#{...}` en valor, tipo y patrón produce el mismo `Object` / `Record` actual. El display de runtime y de tipos cambia a `#[...]` y `#{...}`.
- `{...}` en valor sigue aceptado y produce `Object`: es el literal de `struct` (`const p: Point = { x: 1, y: 2 }`). El checker lo adopta contra el tipo esperado como hoy. En una primera fase ambas formas valen para records; el endurecido futuro (record exige `#`, struct exige tipo nominal) queda como pregunta abierta.
- `{...}` en tipo es error con sugerencia a `#{...}`. Las declaraciones `struct`, `interface` y `class` conservan `{ }`.
- `with` (`value with { field: x }`) conserva `{ }`: es copia contra tipo conocido, no record anónimo.

### Macros `#nombre(...)`

Solo spec en este RFC, sin expansión implementada. Reserva de sintaxis:

```vn
#derive(Debug)
#sql("select * from t where id = ?", id)
```

Reglas previstas:

- `#` pegado al nombre o a `[` / `{` forma un token lógico: `#[`, `#{`, `#ident`. `#` suelto o `# [` con espacio es error.
- `#nombre` sin paréntesis es atributo sin args. `#nombre(...)` acepta lista de args con la misma gramática de llamada, más bloques `#{...}` y tipos `#[...]` como args.
- Fase: expansión antes del binder, sobre AST con rangos preservados, higiénica por defecto con `Span` del sitio de llamada. Sin acceso a disco ni red en expansión (Ley 1).
- El parser acepta `#nombre` y `#nombre(...)` como nodo `Macro` reservado y emite `VN???? macro no implementada` con puntero a este RFC. Ninguna macro se expande en esta fase.

## Introspección comparada (Ley 21)

- C#: tuplas `(int, string)` con paréntesis como Varn viejo, pero con `ValueTuple` nominal por debajo y `record struct` / `record class` separados de objetos. Varn adopta la separación nominal de C# (struct vs record) pero rechaza el paréntesis por ambiguo.
- Python: `(1, 2)` tupla, `()` vacía, `(x,)` de uno. Es la fuente de la ambigüedad que Varn hereda. Varn la abandona: el corchete con `#` hace innecesaria la coma mágica.
- Dart: records `(int, String)` con paréntesis desde 3.0, misma ambigüedad que Varn viejo. Varn decide lo contrario: paréntesis solo agrupan.
- Swift: `(Int, String)` tuplas con paréntesis, structs con `{ }`. Swift convive porque sus paréntesis de llamada y de tupla comparten gramática con etiquetas. Varn prioriza resolución estática sin etiquetas.
- Kotlin: `Pair<A, B>` / `Triple` nominales, `data class` para records. Varn mantiene tuplas estructurales pero con sintaxis propia en vez de nominalizarlas.
- Rust: `[T; N]` arrays, `(A, B)` tuplas, `struct` con `{ }`, macros `nombre!(...)` / `#[derive]`. Varn toma de Rust el `#` como marca de forma especial, pero con sintaxis uniforme `#forma` en vez de dos sintaxis (`!` y `#[...]`).

Puente TypeScript: la superficie se mantiene lo más cerca posible de TS. `[T]` sigue siendo array, `(x: T) => U` sigue siendo función, `{ }` sigue siendo bloque y literal de struct. La única distancia es el `#`, justificada porque TS resuelve tupla vs array y objeto vs bloque por contexto e inferencia, mientras Varn exige resolución estática sin contexto (Leyes 17, 20, 23). Donde TS tiene grieta por JS (`==`, `any`, tipos borrados), Varn pone cimiento: la forma sintáctica dice el tipo.

## Fases del compilador

- `vn-lexer`: sin cambios, `Hash`, `LBracket`, `LBrace` ya existen.
- `vn-parser`: `primary/mod.rs` (`Hash` dispatch, `LParen` solo `Paren`), `patterns.rs` (`#[...]` tupla, `#{...}` objeto), `types/primary.rs` (`#[...]` tupla, `#{...}` record), `match_expr.rs` (sujeto simple, patrones con `#`).
- `vn-core`: `ForeignForm::TupleBrackets` ahora sugiere `#[A, B]`, `TypeDesc` display a `#[...]` / `#{...}`.
- `vn-sem`: `types/display.rs` a `#[...]` / `#{...}`.
- `vn-vm`: `heap/display.rs` a `#[...]` / `#{...}`.
- `vn-checker`, `vn-emit`, `vn-compiler`: sin cambios, operan sobre el mismo AST.

## Ganancia y verificación

- Determinismo de parsing: `(` y `{` dejan de necesitar lookahead por coma para decidir tupla vs agrupación.
- Mensajes accionables: cada forma vieja sugiere la nueva exacta.
- Verificación: `cargo build --profile quick --bin vn`, `vn run -e` para tupla, unidad, record y destructuring, std migrada (`msgpack`, `toml`, `http`, `net`, `semver`, `mime`, `response`, `server`, `udp`), suite completa pendiente de migración del corpus.

## Qué se borra

- `ExprKind::Tuple` desde `(...)`: solo desde `#[...]`.
- `TypeKind::Tuple` desde `(...)`: solo desde `#[...]`, salvo `() => T` que es función.
- `Pattern::Tuple` desde `(...)`: solo desde `#[...]`.
- `MatchPattern::Tuple` desde `(...)`: solo desde `#[...]`.
- `TypeKind::Record` desde `{...}` en tipo: solo desde `#{...}`.
- Tests `hash-tuple-is-gone.vn` y `hash-brace-is-gone.vn`: ahora son sintaxis válida, se borran.
- `ForeignForm::TupleBrackets`: su reemplazo pasa de `(A, B)` a `#[A, B]`.
- Migración: `(a, b)` → `#[a, b]`, `()` unidad → `#[]`, `(A, B)` tipo → `#[A, B]`, `{x: T}` tipo → `#{x: T}`, `for (const (k, v) in m)` → `for (const #[k, v] in m)`, `new Map([("k", v)])` → `new Map([#["k", v]])`.

## Alternativas

- Convivencia con warning: se rechaza por Ley 8, dos caminos de tupla duplican cada `match` del checker y del emit.
- Usar `[A, B]` para tuplas como TS: se rechaza, choca con arrays y con `T[]`, y rompe `entries(): (int, T)[]` que pasaría a ser ambiguo.
- Macros con `!` estilo Rust (`sql!(...)`): se rechaza, introduce un segundo canal de formas especiales cuando `#` ya existe en el lexer.

## Preguntas abiertas

- Endurecer `{...}` en valor para que solo valga con tipo struct esperado y `#{...}` sea el único record.
- Sintaxis de macro con bloques y tipos como args (`#sql(...)`, `#derive(...)` con `#{...}`).
- Código de error definitivo para macro reservada y su fase de expansión con higiene.
