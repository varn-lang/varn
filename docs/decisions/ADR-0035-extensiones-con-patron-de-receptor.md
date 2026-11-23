# ADR-0035 — Extensiones con patrón de receptor en lugar de tipos calculados

Estado: aceptada e implementada.
Fecha: 2026-11-22

## 1. Problema

TypeScript tipa `flat()` con `Flattened<T>`, un tipo condicional con `infer`. Lo necesita porque JavaScript solo tiene un método por nombre para cualquier array: el tipo de retorno se calcula, y el método existe incluso donde no tiene sentido (`["a"].flat()` devuelve una copia). Varn resuelve los miembros en compilación, así que puede declarar el método solo donde aplica. Retirar la maquinaria de tipos de TypeScript (ADR-0034 D9) exigía primero esa capacidad.

## 2. Cómo lo resuelven otros lenguajes

| Lenguaje | Forma | Idea |
|---|---|---|
| TypeScript | `flat(): Flattened<T>[]` | Calcula el tipo; el método existe en todo array |
| Swift | `extension Array where Element: Sequence` | Extensión condicionada por una restricción |
| Kotlin | `fun <T> Iterable<Iterable<T>>.flatten()` | Patrón genérico en el receptor |
| C# | `this IEnumerable<IEnumerable<T>>` | Patrón genérico en el receptor |
| Dart | `extension<T> on List<List<T>>` | Patrón genérico en el receptor |
| Rust | `impl<T> Vec<Vec<T>>` | Patrón en el receptor, con coherencia |

Conclusión: el receptor es un patrón con variables que se unifica con el tipo concreto en cada uso. Varn ya tenía la sintaxis de Dart y TypeScript (`extension<U> on …`); faltaba la semántica.

## 3. Decisión

1. **Patrón de receptor (`vn_sem::bind::ReceiverPattern`).** Cada candidato de extensión guarda el patrón de su receptor con nombres portables (Ley 2). Las extensiones se indexan por constructor: `Array`, `Map`, una clase, un primitivo, tupla o registro. Un receptor que es un parámetro desnudo (`on T`) es VN3010.
2. **Una sola resolución.** `Extensions::resolve` y `Extensions::members_for` unifican el patrón con el receptor y eligen el candidato más específico (`int[]` antes que `E[]`). El checker no tiene otra ruta: se borraron `extension_type_name` y las cuatro búsquedas por clave textual. La firma del miembro se instancia unificando su `this` declarado con el receptor, en vez de heredar por coincidencia de nombre la sustitución de la clase.
3. **Coherencia.** Dos candidatos igual de específicos cuyos patrones se solapan son VN4002 al declararlos, no en la llamada.
4. **Diagnóstico.** Si el miembro existe para el constructor pero el patrón no encaja, VN3004 dice para qué receptor es: "it is an extension for 'U[][]'".
5. **`flat()`** es `extension<U> on U[][]` y `extension<U> on ReadonlyArray<U[]>`; se borran el nativo y `Flattened`.
6. **D9.** Se retiran los tipos condicionales, `infer`, los tipos mapeados, `keyof`, `T[K]` y los tipos de plantilla, con los 12 alias del preludio que dependían de ellos. Cada forma tiene su pista en `ForeignForm`.

Distancia con TypeScript (Ley 23): la sintaxis de la extensión es la misma. La semántica es más estricta: un método existe solo para los receptores que lo declaran, y su tipo se lee en la firma.

## 4. Defectos que aparecieron en el camino

- El preludio importaba todos los exports de un módulo `core:` con código, incluidas sus `declare class`. Bastaba una extensión con cuerpo en `array.vn` para que `new Array<int>()` construyera una instancia vacía. Ahora el contrato separa `code_exports` (`8fdc7b56`).
- `TokenStream::restore` retrocedía la posición pero no los diagnósticos: un intento especulativo abandonado (`a < b[0]` leído como argumentos de tipo) dejaba errores sobre código correcto (`0b477048`).

## 5. Restricciones en las extensiones

`extension<T extends Sized> on T[]` existe solo para los arrays cuyo elemento cumple `Sized`, y dentro del cuerpo `x.size()` se resuelve por la restricción.

1. **Miembros a través de la restricción.** La búsqueda de miembros no consultaba la restricción de un parámetro de tipo, así que tampoco funcionaba `function g<T extends Sized>(x: T) { return x.size() }`. `member_input` resuelve ahora un parámetro acotado a su restricción, y las cachés de miembros usan como clave el receptor resuelto, porque dos funciones con `T` distintos comparten el mismo `Named("T")`.
2. **Selección por restricciones.** `Extensions::matching` devuelve los candidatos que encajan, del más específico al menos, y gana el primero cuyas restricciones se cumplen con los tipos ligados. Una clase con la forma de la restricción pero sin `implements` no la cumple (ADR-0034 D8). Dos candidatos igual de específicos siguen siendo VN4002 aunque sus restricciones difieran, como la coherencia de Rust sin especialización.

## 6. Commits

| Paso | Commit |
|---|---|
| Patrón de receptor y resolución única | `16fe9de5` |
| Prelude con `code_exports` | `8fdc7b56` |
| Diagnóstico con la forma del receptor | `dc47e934` |
| `flat()` como extensión | `1d023cac` |
| `restore` descarta diagnósticos especulativos | `0b477048` |
| Limpieza de `member_type.rs` | `24c2cbeb` |
| D9: retirada de la maquinaria de tipos | `9fbf3b68` |
| Miembros a través de la restricción | `778a47ab` |
| Selección de extensiones por restricciones | `922060d5` |
