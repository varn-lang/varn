# ADR-0031: Puntos pendientes (post-cierre estático)

## Estado
Propuesta (2026-11-18). Continúa el trabajo cerrado y archivado en git
(`docs/decisions/ADR-0030`, eliminado al completarse su checklist C0-C8:
forwarder exacto, Layer pura + bypass relativo cerrado, corpus de errores
reenganchado como script, `runtime:buffer` borrado, capabilities `fs/net`
estáticas punta a punta, `required_caps` checker→TIR→proto, deuda
verificada, 9 slots JIT podados, matriz completa verde).

## Contexto
ADR-0030 deja una sola fuente de verdad estática para capabilities
(declaración en contrato → re-declaración en wrapper → chequeo en uso →
dato en `proto.required_caps`), el corpus `tests/errors` con gate, y la
matriz completa en verde. Lo que sigue necesita diseño propio o RFC y no
cabe en un commit mecánico. Cada punto cita su puerta; sin puerta no se
implementa (Ley 10).

## Descartados (incompatibles con la dirección decidida; no se retoman)
- Herencia automática de capabilities vía `Forward`: la re-declaración
  manual es el contrato visible (puente TS, Ley 21).
- Quitar el sufijo `$`: churn en 150+ ficheros sin ganancia medible.
- Opcodes tipados para `GetProperty`: medido en HEAD, el gap no sobrevive
  en benchmarks (test-code sin anotar); Ley 10 lo prohíbe.
- Enforcement nuevo donde el runtime no deniega (p.ej. gates UDP):
  contradice el principio "reflejar, no inventar".
- Threading de `Roots` por todo `ModuleLoader` / renombres `Stdlib`vs`Std`:
  churn desproporcionado frente a lo ya logrado.
- Auditoría de bails JIT: todos van por `CompileOutcome::Bailed` con
  motivo (`vn-jit/src/lib.rs:238-252`), visibles en `tiers`/`bails`.
  Sin hallazgo; no hay nada que implementar.

## Pendientes (un único ADR; cada uno su RFC o diseño + commit)
- P1. Tablas totales: `CheckerTyTable::get` y `AtomInterner::resolve`
  a tipos que no puedan panicar (diseño de propiedad de tablas;
  cientos de sitios: migración por bordes). Puerta: suite + corpus.
- P2. Enum `RuntimeOp`: las cadenas wire (`@decodes`, ops) a IDs
  tipados; `op_id(&str)` solo para constantes internas hoy. Puerta:
  compatibilidad byte a byte + suite.
- P3. Enforcement de capabilities por paquete (+ manifiesto).
  Requiere DECISIÓN previa (roadmap item 3: primero decidir si los
  contratos pasan a `@capability` y cómo se cierra la frontera).
  Puerta: paquete que toca red sin declarar es rechazado; suite verde.
- P4. Inlining ADR-0028 restante: métodos/clases, estado de módulo no
  constante, `NativeOp` sin args en paquetes, `import *`, enlace std-std
  en imagen. Puerta: ratio + `xtask compare`.
- P5. Semántica gradual de `infer/ops` (retorno `Dynamic` por defecto):
  cambio de semántica, RFC. Puerta: corpus de errores actualizado.

## Migración
Un punto por esfuerzo, un cambio por commit (Ley 9); `-Min` por commit,
`-Fast` completo antes de push.
