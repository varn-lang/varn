# docs/

La documentación pública de Varn **ya no vive aquí**: se migró al sitio oficial,
`varn-lang/varn-docs` (<https://varn.twwk.cc/>). Este directorio
conserva solo el registro interno.

## Dónde fue cada cosa

| Antes (`varn-lang/docs/…`) | Ahora (ruta del sitio) |
|---|---|
| `GETTING_STARTED.md` | `/guias/primeros-pasos/` |
| `INSTALL.md` | `/guias/instalacion/` |
| `CLI_REFERENCE.md` | `/guias/cli/` |
| `CLI_INSPECT.md` | `/guias/inspeccion/` |
| `STABILITY.md` | `/guias/estabilidad/` |
| `ROADMAP.md` | `/guias/roadmap/` |
| `lang/` | `/lenguaje/` |
| `internals/` | `/arquitectura/` |

## Lo que queda aquí

- `decisions/` — ADRs, registro de decisiones de diseño (se enlaza desde el CHANGELOG).
- `dev/` — auditorías, notas y planes de trabajo. Registro interno, no es documentación.
- `superpowers/` — especificaciones de trabajo internas.

Regla: nada de `docs/` se publica salvo a través de `varn-docs`. Si un doc de aquí
vuelve a ser público, se mueve al sitio y se borra de aquí.
