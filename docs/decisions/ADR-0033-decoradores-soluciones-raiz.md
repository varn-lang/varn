# ADR-0033: Soluciones de raíz a los agujeros de decoradores `@`

## Estado

Propuesta (2026-11-26). Complementa RFC 0004 (contrato semántico). Criterio: lo correcto y absolutista, no lo más fácil. Cada solución elimina la causa; ninguna añade flags, modos ni rutas paralelas.

## Contexto

Verificado contra el binario actual (no por intuición):

1. Un envoltorio síncrono sobre un método `async` compila y corre: `await t.hacer(21)` devuelve `0` en vez de `Task<int>`, sin diagnóstico. Causa: la aplicación del decorador es dinámica (`BackendTy::Dynamic` en `classdef_methods.rs`) y el checker valida la aridad de la expresión decoradora pero jamás compara el tipo envuelto contra el tipo devuelto.
2. Un decorador de usuario sobre un miembro `abstract` no se evalúa ni se aplica nunca (log vacío), sin error ni aviso. Causa: el checker solo valida firmas con `body: Some` y el compilador solo aplica sobre cuerpos; nadie diagnostica el vacío.
3. En posición de decorador, un builtin siempre gana a un nombre del ámbito (`function pure` local ignorada ante `@pure`) sin dejar rastro. Causa: `match_builtin` corre antes que la resolución de ámbito.
4. `@throws` se registra en el símbolo (`decorator_attrs.rs`) y ningún checker lo lee jamás. Pretender verificación que no existe es la falla; no la falta de verificación.

## Decisión

### 1. Envoltorio de `async`: preservación de firma obligatoria

El checker exige que el tipo devuelto por el envoltorio acepte los mismos parámetros y devuelva un tipo compatible; si el objetivo es `async` (retorna `Task<T>`), el envoltorio debe devolver `Task<U>`. Violación = error duro `InvalidDecoratorSignature`.

Rechazadas:

- Prohibir envoltorios en `async`: mata casos legítimos (auditoría de tareas con `await` dentro).
- Warning: un warning que se ignora es deuda con intereses; lo silencioso ya demostró ser el bug.
- Chequeo en runtime: cuesta en caliente y llega cuando el daño está hecho.

Leyes: 17/20 (el tipo estático manda), 14 (el contrato de tipos es el cimiento; se repara ahí, no con parches en el runtime).

### 2. Decorador que envuelve `abstract`: error, no silencio

Un decorador de usuario (los que envuelven, es decir, todo lo que no sea builtin reconocido) sobre un miembro sin cuerpo es error `InvalidDecoratorTarget`: no hay nada que envolver. Los builtins de metadatos (`@deprecated`, `@capability`, `@throws`) siguen válidos sobre abstractos porque no envuelven; la distinción ya existe en el código (`wraps_at_runtime`), así que no hay maquinaria nueva, solo el diagnóstico que hoy falta.

Rechazadas:

- Warning: el no-op silencioso es la peor clase de bug (el programa parece hacer algo que no hace).
- Hacerlo funcionar envolviendo el override futuro: magia no local que viola una sola fuente de verdad (Ley 3) y repara el síntoma en el consumidor en vez del cimiento (Ley 14).

### 3. Shadowing de builtin: error, no victoria silenciosa

Si en el ámbito hay un valor con el mismo nombre que un builtin usado como decorador, es error con mensaje que nombra ambos y pide renombrar. Razonamiento: como el builtin siempre gana, ese valor homónimo es código muerto o confusión; en ambos casos el programa no significa lo que parece. El test `shadowScope` se reescribe para esperar el error (Ley 15: un test que valida el comportamiento erróneo es un test incorrecto).

Se acepta que es breaking para código que hoy compila en silencio (Ley 10: la ganancia es corrección semántica medible en diagnósticos, no compatibilidad con código que mentía).

### 4. `@throws`: contrato declarado, explícitamente no verificado

`@throws` queda definido como documentación formal del símbolo (visible en LSP hover y docs generadas), **no** como sistema de efectos. No se añade verificación en los `throw` porque duplicaría el sistema que ya existe para errores esperados (`Result`/`try`, Ley 8: una sola ruta por hecho). Excepciones sin chequeo + `Result` para lo esperado ya es la decisión del lenguaje; el enforcement estilo Java crearía el impuesto de anotación sin beneficio y una ruta paralela.

La honestidad es la fix: se documenta como declarado-y-no-verificado, con puerta de auditoría (`vn test` que liste throws declarados frente a lanzados observados) en lugar de verificación obligatoria que no existe.

## Consecuencias

- Dos errores nuevos del checker (casos 1 y 2), un error nuevo de shadowing (caso 3), cero cambios de runtime: la aplicación (`decorador(valor, ctx)`, `null` conserva) y el orden entrelazado de abajo arriba quedan como están, ya verificados.
- `@throws` no genera trabajo de checker; genera trabajo de LSP/docs (surfacing del contrato).
- RFC 0003: aplicar la enmienda ya decidida en RFC 0004 (`#test`/`#bench` fuera, sin `@` en salida de macros, `VN4xxx`).

## Verificación

- `vn test` con casos para cada regla: envoltorio síncrono sobre `async` (error), decorador que envuelve `abstract` (error), builtin con homónimo en ámbito (error con ambos nombres), matriz de colocación completa sin regresiones.
- `cargo build --profile quick --bin vn`, `clippy` sin `wildcard_enum_match_arm`.
- La suite existente se actualiza donde validaba lo erróneo (`shadowScope`); ningún otro test debe cambiar de resultado.
