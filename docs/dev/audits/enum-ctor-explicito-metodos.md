# Auditoría: enums con constructor explícito + métodos o construcción explícita fallan

- Fecha: 2026-11-25. Binario: `target/quick/vn.exe` (parser nuevo `#[...]`).
- Estado: causa raíz localizada (dos defectos), sin fix aplicado.

## Síntoma

Cualquier combinación de **constructor explícito + (llamada a método | construcción explícita con args)** en un `enum` falla. Sin constructor explícito todo funciona (suite `Shape`, `CustomHttpStatus` en verde por eso: nunca tocan la combinación rota).

## Reproducción mínima

```vn
enum E {
    A(1),
    B(2);
    v: int
    constructor(v: int) { this.v = v }
    fijo(): int { return 42 }
}
const x = E.A
print(x.fijo())   // TypeError: expected 'int', got 'null' (atribuido a E.constructor)
print(E.A(5).v)   // null en campo non-nullable (sin error en la construcción)
```

## Bisección (todo ejecutado, no supuesto)

| Caso | Resultado |
|---|---|
| Sin ctor explícito + métodos + `Shape.Circle(10.0).area()` | OK (`12.56636`) |
| Ctor explícito + campos, sin llamadas a métodos, acceso desnudo `E.A.v` | OK (`1`) |
| Ctor explícito + método sin args, método declarado ANTES del ctor | OK el dispatch, `this` nulo si el valor vino de construcción explícita |
| Ctor explícito + método sin args, método declarado DESPUÉS del ctor | **Roto**: corre el constructor en lugar del método |
| Ctor explícito + 2 métodos, llamar al 2º (`suma`) | Corre el 1º (`cero`): devuelve `0` en vez de `42` |
| Construcción explícita `E.A(5)` | El constructor del usuario **no se ejecuta** (sin side effects); el valor queda con campos `null` |
| Bare `E.A` | Usa la plantilla pre-construida en la definición (el ctor corre 1 vez por variante con args) |
| Variante sin args + campo con default (`v: int = 7`), acceso desnudo | Roto: default ignorado, `null` |

## Causa raíz

**Defecto 1 — tablas de vtable en desacuerdo (despacho a función equivocada).**
La tabla de compilación (`vn-emit/src/tables/enums.rs::enum_members`, líneas 102–105) **excluye** `ClassMemberKind::Constructor` de `EnumInfo.members`. El runtime **sí** emite `DefineMember` para el constructor (`emit_enum` lo mete en `TirClassDef.methods` como el resto; verificado en bytecode: `Method "constructor"` en slot 0 y `Method "fijo"` en slot 1). Resultado: todo método declarado después del ctor despacha una posición corrida — al ctor mismo (si es el primero) o al vecino anterior (caso `suma`→`cero`, probado con side effects: imprime `"ctor corre"` al llamar a `uno()`). Las clases no sufren esto porque su tabla (`tables/classes.rs`) sí incluye el ctor en ambos lados.

**Defecto 2 — la construcción explícita no corre el constructor de usuario.**
`E.A(5)` compila a `GetProperty A` + `CallMethod "constructor"`, pero el ctor del usuario no se ejecuta (probado con side effects: cero impresiones) y el valor resultante tiene los campos en `null` aunque sean no-nulables. Candidatos exactos a inspeccionar: `call_method` en `vn-vm/src/exec/dispatch/reg_ops/method_calls/mod.rs:69-73` (atajo de template `enum_variant_template`/`construct_enum_variant` antes del dispatch normal) y el fallback silencioso de líneas 103–110 (`method_nv.is_null() + name == "constructor"` → devuelve `null` sin error). La variante desnuda `E.A` sí funciona porque usa la plantilla pre-construida en la definición de la clase (el ctor corre ahí, una vez por variante con args).

## Impacto en docs (verificado, nada roto publicado)

Todos los ejemplos de la guía evitan las combinaciones rotas: usan acceso desnudo + lectura de campos (`Urgencia.Alta.nivel`), payload sin ctor explícito (`Shape.Circle(10.0).area()`), o clases. Ningún ejemplo publicado llama a un método sobre un enum con ctor explícito ni construye explícitamente con args sobre ctor de usuario.

## Fix propuesto (absolutista, una sola fuente de verdad)

1. Unificar el orden: o bien el runtime deja de emitir `DefineMember` para el constructor de enums, o bien `EnumInfo.members` lo incluye (como hacen las clases). Cualquiera de las dos elimina la divergencia; la decisión es cuál es canónico. Recomendación: incluirlo en la tabla de compilación (espejo exacto de clases, un solo camino), **salvo** que se decida que `E.constructor` no debe ser invocable — en cuyo caso se excluye en ambos lados y llamarlo es error de compilación.
2. Construcción explícita: o corre el ctor de usuario (coherente con clases y con el `CallMethod "constructor"` ya emitido) o se rechaza en compilación. Lo inaceptable es el estado actual: compila, no corre, devuelve nulos.
3. Verificación: `vn test` con matriz explícito/implícito × antes/después del ctor × desnudo/explícito, más `vn debug -p tiers`/`--help` sin cambios. Sin rutas paralelas ni flags.

## Preguntas abiertas para el diseño

- ¿`E.A(args)` con args distintos a los declarados construye un valor nuevo (corre ctor) o está prohibido por ser las variantes singletons pre-construidos? Hoy compila y miente: hay que elegir.
- ¿Los defaults de campos (`v: int = 7`) aplican a variantes desnudas sin args? Hoy se ignoran.
