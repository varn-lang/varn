# TO-NATIVE: cada tipo con la representación que su tipo estático decide

Si este documento contradice al código, manda el código (Ley 12). Toda cifra
y toda afirmación se vuelve a comprobar antes de actuar.

- Punto de partida: `e66d721b` (char en registro nativo, campo de 4 bytes y
  `ArrayRepr::Char`).
- Principio: el tipo estático ya sabe cómo es su valor en ejecución. Ninguna
  fase lo re-deriva de los valores (Ley 6), y ningún valor de un tipo escalar
  o de heap pasa por la ruta genérica `Dynamic` si su tipo la hace innecesaria
  (Leyes 17 y 20).

## 0. Reglas de ejecución

- Un commit atómico por item (Ley 9), con la comparación de la Ley 21 en el mensaje.
- Cada item declara qué gana, cómo se verifica y qué borra (Ley 10).
- Toda `match` sobre `ScalarRepr`, `ArrayRepr`, `SlotKind`, `RuntimeKind` o
  los `KIND_*` coloca la variante nueva de forma explícita (Ley 7).
- Puerta por item: `tests/main.vn` en JIT, `VN_NO_JIT=1`, `VN_JIT_CACHE=verify`,
  `VN_GC_STRESS=256 VN_GC_VERIFY=1` y `VN_STD=@embedded`, más `error_corpus`,
  `debug_golden`, clippy y rustfmt. Todos los items tocan el JIT, así que antes
  de push va la matriz completa `.\scripts\verify.ps1` (Ley 19).
- Cada item añade casos a `tests/` que fallan antes del cambio.

## 1. Estado actual medido en el código

| Tipo | Campo (`TypeLayout::of_field`) | Registro (`slot_kind_of`) | Array (`ArrayRepr`) |
|---|---|---|---|
| `int` | `I64`, 8 B | `Int` | `I64` |
| `float` | `F64`, 8 B | `Float` | `F64` |
| `bool` | `Bool`, 1 B | `Bool` | **`Boxed`, 16 B** |
| `char` | `Char`, 4 B | `Char` | `Char`, 4 B |
| `str` | `Boxed`, 16 B (SSO inmediato) | `Str` | `Boxed` |
| clase, array, map, set, fn, bytes | `Ref`, 8 B | `Ref` | **`Boxed`, 16 B** |
| tupla, rango, opaco, task handle, función | **`Boxed`, 16 B** | `Ref`/`Dynamic` | `Boxed` |
| enum | **`Boxed`, 16 B** | `Ref` (objeto de heap) | `Boxed` |

`vn debug -p layout` muestra esta tabla para cada clase de un módulo.
| `BigInt`, `Decimal` | **`Boxed`, 16 B** | **`Dynamic`** | `Boxed` |
| `T?` escalar | **`Boxed`, 16 B** | `Dynamic` | `Boxed` |

Hechos que condicionan el plan:

- Un enum es siempre un `HeapObj::EnumVariant(Box<EnumVariantData>)`, incluso
  una variante sin datos (`exec/enum_variants.rs`, `vn-macros/src/vn_enum.rs`).
- `ArrayRepr::unboxed` elige la representación mirando los valores, y
  `migrate_to_boxed` la cambia en ejecución.
- Los genéricos se borran: un parámetro de tipo baja a `Dynamic(Generic)`
  (`vn-emit/src/ty/lower.rs`). Un `T[]` creado dentro de código genérico no
  conoce `T`.
- `BigInt` y `Decimal` siempre viven en el heap (`heap/intern.rs`).

## 2. Items

### N1. Los tipos que siempre viven en el heap son una referencia

- **Qué cambia:** tupla, enum, función, rango, task handle, `BigInt` y
  `Decimal` pasan a `ScalarRepr::Ref` (8 B, null por `COMPACT_REF_NULL`) en los
  campos: `BackendTy::field_kind` les da su `RuntimeKind` (antes devolvía `None`
  para tuplas, enums y funciones) y `of_field` los coloca en `Ref`. `BigInt`,
  `Decimal`, funciones, rangos y task handles pasan a `SlotKind::Ref` en los
  registros. `Opaque` no lo produce el heap y sigue en `Boxed`. Hasta N2 un
  enum es siempre un objeto de heap, así que también es `Ref`.
- **Gana:** 8 B por campo y quitar las comprobaciones de etiqueta en registros
  cuyo tipo ya garantiza un puntero de heap.
- **Verifica:** layout de una clase con esos campos (`vn debug`), suite completa,
  GC stress (las referencias compactas se trazan).
- **Borra:** esos tipos salen del grupo `BOXED` de `of_field` y del grupo
  `Dynamic` de `slot_kind_of`.
- **Comparación:** en Rust, Swift, C# y Kotlin un tipo referencia ocupa un
  puntero en un campo.

### N2. Una variante sin datos es un valor inmediato

- **Qué cambia:** un nuevo `KIND_ENUM` (7) cuyo payload identifica la variante:
  un registro de enums en tiempo de ejecución (id del enum) y la etiqueta
  declarada. Las variantes sin datos de enums de usuario y de enums nativos
  (`vn_enum!`) se crean así. Las variantes con datos siguen siendo objetos de heap.
- **Afecta:** `GetEnumTag`, igualdad (`bits_eq` para inmediatos), `typeof`/`::type`,
  impresión y `toString` (nombre desde el registro), `is`, `match`, JSON,
  `conform`, inline caches de campos por nombre sobre un enum, y las globales
  de variantes importadas (`TirImportedVariant`).
- **Campos y registros:** un enum cuyas variantes no tienen datos se guarda en
  un campo de 8 B y en un registro nativo de palabra. Un enum mixto es `Ref`
  si todas sus variantes viven en el heap, o `Boxed` si mezcla.
- **Gana:** cero asignaciones por variante, `match` e igualdad sin leer memoria.
- **Verifica:** tests de enums (`tests/16`, websocket, enums nativos de std),
  `match` en bucle caliente compilado sin bails, `Json.stringify` de un enum.
- **Borra:** `EnumVariantData::unit` para variantes sin datos y la asignación
  de `make_enum_variant` y del builder de `vn_enum!`.
- **Comparación:** Rust, C#, Kotlin y Swift representan un enum sin datos como
  su discriminante entero; Rust y Swift guardan los enums con datos como una
  unión con etiqueta.

### N3. Los escalares nulables usan un nicho en los campos

- **Qué cambia:** `bool?` en 1 B (2 es null), `char?` en 4 B (0x110000 es null),
  enum inmediato nulable en 8 B (un payload reservado es null). `int?` y
  `float?` se quedan en 16 B, porque no tienen valor libre para el null.
- **Gana:** 15, 12 y 8 bytes por campo respectivamente.
- **Verifica:** lectura y escritura de null y no-null por slot, por nombre
  (inline cache) y con `with`, compilado e interpretado.
- **Borra:** esos nulables salen de `BOXED`.
- **Comparación:** Rust guarda `Option<bool>` en 1 B, `Option<char>` en 4 B
  (nicho por encima de 0x10FFFF) y `Option<i64>` en 16 B; Swift hace lo mismo
  con sus *extra inhabitants*.

### N4. Arrays de bool y de referencias sin caja

- **Qué cambia:** `ArrayRepr::Bool(Vec<bool>)` (1 B por elemento) y
  `ArrayRepr::Ref` (punteros de 8 B con nicho de null) para elementos que
  siempre son objetos de heap. El GC traza `Ref` y la verificación de GC lo cubre.
  El JIT añade `Elem::Bool` y `Elem::Ref` con su ancho.
- **Gana:** de 16 B a 1 B y a 8 B por elemento, y lectura nativa en el JIT.
- **Verifica:** arrays de bool y de instancias en bucles calientes, `push`,
  `pop`, migraciones, GC stress.
- **Comparación:** `Vec<bool>` y `Vec<Box<T>>` en Rust, `[Bool]` y `[T]` (de
  clase) en Swift, `bool[]` y `T[]` en C#.

### N5. La representación de un array la decide su tipo estático

- **Qué cambia:** la instrucción que crea un array lleva la representación de
  su tipo de elemento, y un array no cambia de representación en vida.
  `unboxed()`, `push_repr_change` y `migrate_to_boxed` desaparecen (Ley 8).
- **Bloqueo:** con genéricos borrados, el código genérico que crea un `T[]` no
  conoce `T`. Resolverlo de raíz exige que el código genérico reciba la
  representación de `T` (metadatos de tipo en la llamada, como Swift, o
  genéricos reificados, como C#) o que se monomorfice.
- **Requiere decisión del dueño** antes de empezar: es un cambio del modelo de
  genéricos, no solo de arrays.

### N6. Una tupla tiene un layout compacto

- **Qué cambia:** una tupla se guarda con el layout de sus tipos de elemento,
  como una instancia, no como un `VmArray` de valores.
- **Bloqueo:** depende del orden canónico de campos de los records (pendiente
  por decisión del dueño). No empieza sin él.

## 3. Orden

N1 → N2 → N3 → N4. N3 usa el enum inmediato de N2. N5 y N6 esperan sus decisiones.
