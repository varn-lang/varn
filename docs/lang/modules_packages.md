# Varn Language — Módulos y Paquetes

> Fuentes: `tests/async.test.vn`, `tests/decorators.test.vn`, `tests/stdlib-comprehensive-test.test.vn`, `tests/isolates-multithread.test.vn`, `tests/channels.test.vn`, `tests/module-locals.test.vn`, `tests/capabilities-security.test.vn`, `tests/std-csv.test.vn`, `tests/canonical-types-regex-uuid-time.test.vn`, `tests/stdlib-process-compress-cli-env.test.vn`, `tests/stdlib-sqlite-ws-archive.test.vn`, `tests/tagged-templates.test.vn`.

---

## 1. Declaraciones `import`

### Importación nombrada desde stdlib

```vn
import { sleep, TaskGroup, spawn, parallel } from "std:task"   // tests/async.test.vn
import { channel, Sender, Receiver, ChannelClosed, spawnIsolate } from "std:task"   // tests/isolates-multithread.test.vn
import { describe, it, expect } from "std:test"   // tests/async.test.vn
import { MetaKey, ClassRef } from "std:reflect"   // tests/decorators.test.vn
import { WebSocket } from "std:ws"   // tests/stdlib-sqlite-ws-archive.test.vn
import { readText, writeText, remove, exists } from "std:fs"   // tests/capabilities-security.test.vn
import { Env } from "std:env"
import { platform, cwd } from "std:sys"
```

Resto de exports por paquete: ver `standard_library.md` (solo lo citado
con `tests/*.vn`).

### Importación desde módulo local

```vn
import { workerMain } from "./some-module"
```

---

## 2. Declaraciones `export`

```vn
export async function workerMain(rx: Receiver<ValueMsg>, tx: Sender<int>) {
    // ...
}
// tests/isolates-multithread.test.vn
```

Solo las funciones exportadas (`export`) de nivel superior pueden usarse con `spawnIsolate`.

---

## 3. Módulos Locales y Variables Globales del Módulo

Las variables declaradas en el nivel superior del módulo son accesibles desde todos los ámbitos del archivo, incluyendo dentro de generadores y funciones:

```vn
let moduleCounter = 0
function bumpCounter(): int {
    moduleCounter = moduleCounter + 1
    return moduleCounter
}
function* touchesModuleState() {
    yield bumpCounter()    // accede a moduleCounter del módulo
    yield bumpCounter()
}
// tests/generator-values.test.vn
```

---

## 4. Paquetes de la Stdlib

Ver `standard_library.md` (mapa: `../STDLIB_ARCHITECTURE.md`). Aquí solo los
imports ejercitados:

```vn
import { sleep, TaskGroup, spawn, parallel } from "std:task"   // tests/async.test.vn
import { channel, Sender, Receiver, ChannelClosed, spawnIsolate } from "std:task"   // tests/isolates-multithread.test.vn
import { describe, it, expect } from "std:test"   // tests/async.test.vn
import { MetaKey, ClassRef } from "std:reflect"   // tests/decorators.test.vn
import { readText, writeText, remove, exists } from "std:fs"   // tests/capabilities-security.test.vn
import { Env } from "std:env"
import { platform, cwd } from "std:sys"
```

APIs por paquete (`CSV`, `Regex`, `UUID`, `DateTime`, `Process`, `Compress`,
`CLI`, `Env`, `Path`, `Database`, `WebSocket`): solo lo citado en
`standard_library.md` con `tests/*.vn`.

