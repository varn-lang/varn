# RFC 0004: contrato semántico de decoradores `@` y frontera con macros `#`

- Estado: draft
- Fecha: 2026-11-26
- Continúa a RFC 0003 (macros). Los decoradores no tenían RFC: solo tests (`decorators.test.vn`, `decorator-builtins.test.vn`) y código (`vn-core/src/ast/decorators.rs`, `vn-checker/src/checker/decorator_*.rs`, `vn-compiler/src/from_tir/build/classdef_methods.rs`).

## Resumen

`@nombre` anota una declaración existente y nunca genera sintaxis. Hay dos familias disjuntas: **builtins** (solo estáticos, nunca envuelven en runtime) y **de usuario** (envoltorios en runtime aplicados al evaluar la definición, siempre con pérdida de receptor y de tipado estático: el resultado es dinámico). La regla de oro: **lo que el checker no puede probar, no se promete en silencio**. Este RFC fija el modelo de evaluación, los contextos exactos, la matriz de colocación, dos errores nuevos que hoy son agujeros silenciosos y la frontera con `#` (incluida la resolución de la colisión `@test` / `#test`).

## Motivación

Verificado contra el binario actual:

1. Un envoltorio síncrono sobre un método `async` compila y corre: `await t.hacer(21)` devuelve `0` en vez de `Task<int>`, sin ningún diagnóstico. El `Task` se rompe en silencio.
2. Un decorador de usuario sobre un miembro `abstract` no se aplica nunca (ni siquiera se evalúa) y no hay error ni aviso: no-op silencioso.
3. El RFC 0003 propone `#test` y `#bench` mientras `@test` ya existe, se descubre con `vn test` como `fichero::nombre` y su propio RFC dice que ningún nombre existe en ambos canales. Contradicción pendiente de resolver.

## Modelo de evaluación (fijado)

1. **Cuándo**: al evaluar la definición (clase o función), una sola vez, en orden de programa. Los decoradores no existen antes ni después: no hay fase separada.
2. **Orden verificado**: de abajo arriba, por decorador, **evaluación y aplicación entrelazadas**: con `@fab("externo")` encima de `@fab("interno")`, el orden real es `fabrica:interno, aplica:interno, fabrica:externo, aplica:externo`. Difiere de TypeScript (que evalúa todas las fábricas de arriba abajo y luego aplica de abajo arriba). Varn fija su orden y lo documenta; no lo imita.
3. **Aplicación**: `decorador(valor, ctx)` con el valor actual; si el resultado es `null` (o el decorador declara `void`), se conserva el valor anterior. Fábricas (`@f(x)`) devuelven el decorador; un `return` directo sin llamada también vale.
4. **Clases**: los miembros se decoran primero (cada uno con su regla), la clase después, sobre la clase ya construida. Un decorador de clase recibe `(ClassRef)` y puede devolver otra clase; devolver `null`/`void` conserva la original.
5. **Contextos** (tipos ambiente del compilador, no se importan; `FunctionRef`/`ClassRef` sí vienen de `std:reflect`):
   - método: `(fn, ctx)` con `ctx = #{ name: str, isStatic: bool, isPrivate: bool }`.
   - getter/setter: `(fn, ctx)` con el mismo `ctx` (`GetterContext`/`SetterContext`).
   - constructor: `(fn, ctx)` con `ConstructorContext`.
   - propiedad: `(objetivo, ctx)` con `PropertyContext`; el decorador **no devuelve nada** (solo observa o anota metadatos).
   - función libre: `(fn)`; clase: `(cls)`.
6. **Sin receptor**: el envoltorio pierde `this`/`super` por construcción (el `ctx` lleva los flags, no el receptor). El checker lo convierte en error `InvalidDecoratorTarget` si el cuerpo decorado los usa. Los decoradores solo tienen sentido sobre lógica sin receptor o que lo reinyecte explícitamente.
7. **Sin tipado estático del resultado**: el miembro decorado por un envoltorio de usuario pasa a ser dinámico en runtime. El checker valida la expresión del decorador (debe ser función con la aridad del objetivo; su retorno debe ser invocable/compatible o `void`/`null`), pero **no promete preservación de firma**: ese es el hueco que cierra la regla 8.

## Reglas nuevas (cierran los agujeros)

8. **Envoltorio de `async`**: si el objetivo es `async` (devuelve `Task<T>`), el valor que devuelve el envoltorio debe ser `Task`. Un envoltorio síncrono sobre `async` es error `InvalidDecoratorSignature` (hoy compila y rompe el `await` en silencio; verificado).
9. **Miembro `abstract`**: un decorador de usuario (que envuelve) sobre un miembro sin cuerpo es error `InvalidDecoratorTarget`: no hay nada que envolver y hoy se ignora en silencio (verificado: ni siquiera se evalúa). Los builtins de metadatos (`@deprecated`, `@capability`, `@throws`) siguen valiendo sobre abstractos porque no envuelven.
10. **Colisión builtin**: en posición de decorador, un nombre builtin siempre gana al del ámbito (verificado: `function pure` local se ignora ante `@pure`). Se mantiene por predictibilidad, pero con diagnóstico informativo cuando el ámbito contiene un nombre que queda oculto (hoy es totalmente silencioso).

## Interacciones fijadas

- `override` + decorador: permitido; el decorador se aplica al miembro que sobrescribe, las reglas de `override` no cambian.
- `static` + decorador: permitido; `ctx.isStatic` lo refleja; `this`/`super` siguen prohibidos en el cuerpo envuelto.
- `private`:可见 en `ctx.isPrivate`; sin cambios.
- getter/setter/ctor/property: según contextos del punto 5; propiedad siempre observadora.
- `async` + `@pure`: permitido si el cuerpo cumple pureza; `@pure` + envoltorio de usuario: el envoltorio corre en runtime y su pureza no se verifica — se documenta, no se promete.
- `@test`: solo en funciones sin parámetros requeridos (el runner las llama sin args; verificado `VN...`: `InvalidDecoratorTarget` si los tienen). Descubrimiento: `vn test` las lista como `fichero::nombre`.
- `@inline`: pista al compilador (`force_inline`); los miembros decorados por envoltorios de usuario nunca se inlinean y rompen la cadena de herencia local para inlining (verificado en `inline_method.rs`, `construct.rs`).

## Builtins: semántica exacta

- `@deprecated("mensaje?")`: aviso `VN3025`/`DeprecatedUse` en cada uso (una vez por sitio), nunca error. Sin args o con un `str`; otra cosa es error de firma.
- `@pure`: contrato estático verificado: sin asignación a no-locales (solo parámetros y locales) y solo llamadas a callees `@pure`. Violarlo es error `ImpureOperation`. No es una promesa de runtime: es una prueba del checker.
- `@inline`: solo funciones; con forma no-inlineable el checker la rechaza (`check_inline_shape`).
- `@capability dominios...`: propaga estáticamente los dominios al símbolo; llamar sin tenerlos es error `CapabilityViolation`/`VN4302` (fail-closed: lo desconocido se deniega). Solo en función/método/getter/setter/ctor.
- `@test`: registra `is_test`; `vn test` lo ejecuta.
- `@throws(Clase, ...)`: contrato declarado sobre el símbolo. Hoy **no se verifica en los `throw`** (verificado: el checker no lo lee): queda definido como contrato declarado visible en LSP/documentación hasta que se implemente la verificación, sin pretender enforcement.
- `@decodes("op")`: pista de lowering del JIT; `op` debe existir en `RuntimeOp` o es error con la lista válida.
- Colocación: matriz `supports()` vigente (p. ej. `@test`/`@inline`/`@decodes` solo en funciones; `@capability`/`@throws`/`@pure` también en método/getter/setter/ctor). Fuera de matriz: error `InvalidDecoratorTarget`.
- `@deprecated` vale en todas partes (incluidos abstractos) porque no envuelve.

## Frontera `@` frente a `#` (cierra RFC 0003)

- `@` nunca genera sintaxis; `#` nunca envuelve en runtime ni decide por tipos. La salida de una macro **no puede contener `@`** (el expansor la rechaza): evita la paradoja de orden entre expansión y decoración.
- **Resolución `@test` / `#test`**: se elimina `#test` (y no se crea `#bench`) del alcance de RFC 0003. Razones: `@test` ya está implementado, descubierto y documentado; la regla del propio RFC 0003 prohíbe un nombre en ambos canales; `#bench` no existe y la selección por fichero/filtro ya cubre el caso.
- `#derive` genera nominales (`impls` con los `Atom` de campos, rangos del sitio, sin nombres visibles nuevos). Nunca lee cuerpos de funciones ni infiere tipos (Ley 14).
- `#cfg` y atributos no generan nombres; `#sql`/`#regex`/`#html` devuelven valores tipados ya cerrados y validan en compilación.
- Orden global: primero `#` (AST→AST puro), después `@` (sobre el AST ya expandido).

## Introspección comparada (Ley 21)

- TypeScript 5: decoradores como funciones `(valor, contexto)` evaluados de arriba abajo y aplicados al revés, con tipado laxo. Varn adopta la forma pero fija el orden entrelazado de abajo arriba (verificado, no heredado por intuición) y exige aridad por tipo de objetivo, que TS no hace.
- Python: `@` envuelve en runtime sin red. Varn conserva la envoltura pero añade el contrato estático (aridad, retorno, receptor, pureza, capabilities), que Python delega a convención.
- Rust: `#[derive]` genera código con higiene. Varn toma la higiene y los roles cerrados de Rust para `#`, pero rechaza el texto arbitrario y los proc-macros con IO: args son AST tipado y la lista es cerrada.
- Dart `macro`: introspección de tipos en el expansor. Varn lo rechaza (Ley 14): la macro no ve tipos, solo sintaxis.
- Puente TS: la superficie se mantiene (`@nombre`, `@nombre(args)`, contextos con `name`). La distancia justificada: el orden entrelazado (sin fase separada de fábricas) y que el resultado envuelto sea dinámico. Donde TS promete tipos que borra, Varn exige verificación o lo declara dinámico.

## Ganancia y verificación

- Dos agujeros silenciosos pasan a error con mensaje accionable (envoltorio síncrono sobre `async`; decorador que envuelve `abstract`).
- Diagnóstico informativo para shadowing de builtins.
- Sin rutas paralelas: ninguna fase nueva de decoración, ningún flag; los cambios son validaciones del checker sobre el camino existente.
- Verificación: `vn test` con casos para cada regla (orden entrelazado, `async`, `abstract`, shadowing con aviso, matriz de colocación completa, `@test` sin args), `cargo build --profile quick`, `clippy` sin `wildcard_enum_match_arm`.

## Qué se borra y qué no se toca

- Se borra la pretensión de que `@throws` se verifica (pasa a contrato declarado hasta su enforcement).
- No se toca el runtime de aplicación (llamada `decorador(valor, ctx)`, `null` conserva) ni la lista cerrada de builtins.
- En RFC 0003: salen `#test` y `#bench` del alcance; entra la prohibición de `@` en salida de macros; los códigos de error quedan en `VN4xxx` (41xx expansión, 42xx validación por macro), a detallar cuando se implemente cada macro.

## Plan

1. Checker: errores de `async` y `abstract` + aviso de shadowing, con tests.
2. Docs de usuario: sección de decoradores con el modelo real (orden entrelazado, sin receptor, resultado dinámico).
3. RFC 0003: aplicar la enmienda (`#test`/`#bench` fuera, sin `@` en salida, `VN4xxx`).
4. `@throws`: decidir enforcement en `throw` o mantenerlo declarado (con este RFC como especificación del contrato).
