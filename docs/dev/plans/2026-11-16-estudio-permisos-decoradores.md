# Estudio previo: permisos y decoradores (2026-11-16)

## 1. Auditoría de permisos (origen: 4 subagentes + revisión directa)

- Default abierto: `crates/vn-cli/src/policy.rs:18-19` sin flags → `allow_all()`. Bypass: `standalone.rs:63`, `dap/driver.rs:47`, `Vm::new` (`vn-vm/src/exec/ctx.rs:138`), `compare-tiers` pierde flags.
- Sin gate: `compress` tar/zip (`compress.rs:92-118`), `net udpSendTo` (`net.rs:126`), `process exec/spawn` sin `check_exec` (solo gate de contrato), `sys platform/cwd/args/exit/now`, `fs tempDir`, `process currentExe`, `io readLine/write`, `ui setDark`.
- Handles globales adivinables: `fs/descriptors.rs:14`, `sqlite/handles.rs:9`, `process.rs:14`, `net/driver/mod.rs:15`.
- `check_exec`/`check_ui` muertos (cero llamadas salvo `ffi.rs:21`). Granular nunca otorga `SYS_EXEC` (sin `--allow-exec`).
- `path_matches_any` (`capabilities.rs:153`): falla si el destino no existe, TOCTOU check→open, `net.connect` ignora puerto.
- Estático eludible (alias/HOF, métodos sin caps en `ClassMemberInfo`, computadas, `new`/`super`, pipeline, shadowing que neutraliza en `decorator_attrs.rs:58-60`): lo estático es lint, la frontera es runtime.

## 2. Dirección acordada (sin archivos externos)

El código es el manifiesto: capabilities como valores + efectos, sin `varn.json` obligatorio.

- Importar no otorga; el poder entra como parámetro (`main(fs: Fs)`) o atenuación (`sub()`). Sin token no compila/no ejecuta.
- `@capability(C)` = "ejecutar este cuerpo necesita C". Checker infiere y verifica `inferido ⊆ declarado`; la VM inyecta tokens por sesión; CLI liga objetos (`--dir ./d:ro`), default cero.
- Handles por sesión (adiós `static` globales), resolución anclada (adiós TOCTOU/traversal).

## 3. Primer paso: unificar la semántica de decoradores

Fragmentación actual: `@permission` (contratos, 1 dominio, sin validar) vs `@capability` (usuario, N dominios free-form como `"fs"`/`"net"` que ni parsean contra `Capability`), `ContractDecorator` vs `BuiltinDecoratorKind`, 5 copias del extractor de head, shadowing que neutraliza en silencio (test `116` loconsagra), `user_decorators` regado por binder/checker/emit, `check_native_fn_capability` que deja pasar lo desconocido (`dispatch/mod.rs:95-97`).

Unificación (un commit, sin pasarelas):

- Un solo registro en `vn-core`: `Deprecated Pure Inline Capability Test Throws Decodes`. `@permission` se elimina; contratos migran a `@capability`. Tabla de colocación única (`supports()`).
- Dominios validados y canónicos en el parse (`Capability::parse`, `ErrorClass::of_name`); ordenados+dedup; `BuiltinMatch.result: String` para nombrar lo inválido.
- Head extractor único; `has_builtin` lo reutiliza.
- Builtin gana en posición de decorador: se elimina `user_decorators` y `is_active_builtin` (pasa a `is_builtin`); test `116` y `decorator-capability-missing` se reescriben (`"fs.read"`, `"net.client"`, builtin-gana).
- `NativeOpEntry.capability` lleva el conjunto unido (`", "`); el gate exige TODO y deniega lo desconocido (fail-closed).
- `sqlite open` declara `fs.read, fs.write` (refleja `allow_path`; sin cambio de conducta).
- Colocación de `Throws`/`Decodes` también se valida en `vn-contract`.
- Puerta dejada para el endurecimiento: declarar `@capability` en los nativos con checks manuales (fs/net/io/sys/compress) y luego generar guards desde el tipo.
