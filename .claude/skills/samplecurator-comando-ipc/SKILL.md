---
name: samplecurator-comando-ipc
description: "Use this skill when adding, changing or removing a Tauri IPC command in the SampleCurator project (/home/poio/Documentos/GIT/SampleCurator). Triggers on 'nuevo comando', 'añade un comando Tauri', 'expón esto al frontend', 'nuevo endpoint IPC', 'add a Tauri command', 'llamar a Rust desde React'. Encodes the ipc/ layering, AppError, tauri-specta bindings generation, the src/lib/ipc.ts wrapper, progress channels and the raw-bytes rule for binary payloads."
metadata:
  version: 1.0.0
---

# Comando IPC en SampleCurator

Un comando nuevo toca **cuatro sitios**. Si falta uno, el contrato queda roto.

```
1. src-tauri/src/ipc/<area>.rs      el comando (capa fina, sin lógica)
2. src-tauri/src/lib.rs             registro en el invoke_handler y en specta
3. src/bindings.ts                  GENERADO — nunca a mano
4. src/lib/ipc.ts                   envoltorio tipado que usa el frontend
```

Áreas existentes: `library`, `player`, `triage`, `settings`. Si el comando no encaja en ninguna,
pregunta antes de crear una nueva.

---

## 1. El comando en Rust

La capa `ipc` **no contiene lógica**: valida la entrada, llama al módulo que corresponde
(`db`, `audio`, `scan`, `fileops`) y traduce el resultado.

```rust
// src-tauri/src/ipc/library.rs
use tauri::State;
use crate::{db::Db, domain::SampleSummary, error::AppError};

#[tauri::command]
#[specta::specta]
pub async fn library_page(
    db: State<'_, Db>,
    offset: u32,
    limit: u32,
    query: Option<String>,
) -> Result<Vec<SampleSummary>, AppError> {
    if limit > 500 {
        return Err(AppError::InvalidInput("limit máximo 500".into()));
    }
    db.samples_page(offset, limit, query.as_deref()).await
}
```

Reglas:

- Nombre `area_accion` en snake_case: `library_page`, `player_play`, `triage_undo`.
- Devuelve siempre `Result<T, AppError>`. Nunca `String` como error, nunca `unwrap()`.
- Los tipos de entrada y salida son de `domain/` y derivan
  `#[derive(Serialize, Deserialize, specta::Type)]`.
- **Trabajo pesado nunca en el hilo del comando**: mándalo a `rayon`, a un hilo dedicado o al
  motor de audio, y devuelve en cuanto esté encolado.
- Si el comando puede tardar más de ~100 ms, no devuelvas el resultado completo: acepta un
  `Channel<Progress>` y emite lotes (throttle ~10 msg/s, nunca un mensaje por elemento).
- Si la carga útil es binaria y grande (picos de waveform), devuelve
  `tauri::ipc::Response::new(bytes)`. Nunca un array JSON ni base64.

## 2. Registro

```rust
// src-tauri/src/lib.rs
let builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
        ipc::library::library_page,
        ipc::library::library_add_source,
        ipc::player::player_play,
        // ← añade aquí, en el orden del módulo
    ]);
```

## 3. Regenerar los bindings

```bash
cargo test --manifest-path src-tauri/Cargo.toml export_bindings
```

`src/bindings.ts` **se versiona**: su diff es la revisión del cambio de contrato. Nunca lo edites
a mano ni dupliques sus tipos en el frontend.

## 4. El envoltorio del frontend

```ts
// src/lib/ipc.ts
import { commands } from "../bindings";
import type { SampleSummary } from "../bindings";

export async function libraryPage(
  offset: number,
  limit: number,
  query?: string,
): Promise<SampleSummary[]> {
  const res = await commands.libraryPage(offset, limit, query ?? null);
  if (res.status === "error") throw toAppError(res.error);
  return res.data;
}
```

- **Ningún componente llama a `invoke()` ni importa `bindings.ts` directamente.** Hay un hook que
  lo bloquea.
- Los errores se normalizan aquí a un tipo único con mensaje ya listo para la barra de avisos.
- Los `camelCase` del frontend se traducen aquí; en Rust todo sigue en snake_case.

## 5. Tests

- Rust: test de integración del módulo al que llama el comando (no del comando en sí, que es una
  capa fina). Si el comando toca archivos → `TempDir` obligatorio.
- Frontend: los tests mockean `src/lib/ipc.ts` entero. Nunca se mockea `@tauri-apps/api`.

## Checklist antes de darlo por hecho

- [ ] `#[specta::specta]` presente y comando registrado en `collect_commands!`
- [ ] `Result<T, AppError>`, sin `unwrap()`/`expect()`
- [ ] Bindings regenerados y versionados
- [ ] Envoltorio en `src/lib/ipc.ts`, ningún `invoke()` suelto
- [ ] Nada pesado bloqueando el hilo del comando
- [ ] Progreso por `Channel` con throttle si dura más de 100 ms
- [ ] Binario grande enviado como bytes crudos
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` limpio
