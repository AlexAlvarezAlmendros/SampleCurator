# ADR-0006 — Cómo se genera el contrato IPC y quién remuestrea

**Fecha:** 2026-08-18 · **Estado:** aceptada · **Sustituye a:** las menciones a `tauri-specta` y
`rubato` de ADR-0001, ADR-0002 y del CLAUDE.md inicial.

## Contexto

Al empezar la Fase 1 se dieron dos sorpresas al mirar el registro de crates de verdad en vez de
fiarse del plan escrito antes de tocar código.

## 1 · Los tipos del backend se generan con `ts-rs`, no con `tauri-specta`

**Lo que se planeó:** `tauri-specta` genera desde Rust tanto los tipos TypeScript como los
envoltorios de cada comando. Es la herramienta canónica para esto en Tauri.

**Lo que hay:** en el registro solo existen `specta` 1.0.5 y `tauri-specta` 1.0.2, que son de la
línea para **Tauri 1**. La versión 2 (la que soporta Tauri 2) sigue sin publicarse como estable:
`cargo add specta@2` y `cargo add tauri-specta@2` no encuentran nada.

**Decisión:** generar los **tipos** con `ts-rs` 12 (maduro, estable, sin sorpresas) y escribir a
mano la capa fina de envoltorios en `src/lib/ipc.ts`, que de todas formas el CLAUDE.md ya exigía
como única puerta de entrada al núcleo.

```rust
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SampleRow { … }
```

`cargo test` regenera `src/bindings.ts`, que se versiona: el diff del contrato se revisa en cada
cambio.

**Consecuencia con nombre propio:** ts-rs mapea `i64` a `bigint` porque en el caso general no
cabe en un `number`. Pero el puente IPC de Tauri serializa a JSON, donde eso llega como `number`:
el tipo generado mentiría y todo el frontend petaría. Cada `i64` del dominio lleva por eso
`#[ts(type = "number")]`, con un comentario explicando por qué. Ningún valor de este dominio
(ids, tamaños, milisegundos) se acerca a 2⁵³.

**Lo que se pierde:** los envoltorios de comandos no se generan; si alguien cambia la firma de un
comando en Rust y no toca `ipc.ts`, el compilador de Rust no se entera. Lo compensan los tipos
generados (que sí rompen la compilación de TypeScript si el tipo cambia) y el hook que impide
llamar a `invoke` fuera de `ipc.ts`. Si `tauri-specta` v2 llega a estable, migrar es un cambio
contenido en dos archivos.

## 2 · El remuestreo lo hace un sinc propio, no `rubato`

**Lo que se planeó:** `rubato` para remuestrear con calidad cuando el archivo y el dispositivo no
coinciden (el caso real: 44.100 ↔ 48.000).

**Lo que hay:** `rubato` 1.0 rediseñó su API sobre el crate `audioadapter`, con tipos `Async` /
`FixedAsync` / `Fft` y adaptadores de buffer. Es una buena API para lo que resuelve —remuestreo
*en streaming*, con ratio variable y sin reservar memoria— pero nuestro caso es el contrario:
una conversión **offline, de ratio fijo y sobre el buffer entero**, una sola vez al cargar el
sample y siempre en el hilo de control.

**Decisión:** `src/codec/resample.rs`, un sinc enventanado de 32 taps con ventana de Blackman y
tabla polifásica de 512 fases (~130 líneas). La tabla evita llamar a `sin()` millones de veces:
se precalcula una vez y el bucle interior es solo multiplicar y sumar.

**Por qué se puede defender:** la calidad se mide, no se supone. Los tests comparan la salida
contra el seno generado analíticamente a la frecuencia destino y exigen un error RMS por debajo
de 0,01 en ambos sentidos (48k→44,1k y 44,1k→48k), comprueban que una continua sale sin
oscilación de nivel entre fases y que el estéreo no se mezcla. La interpolación lineal —lo que
usaba el spike— no pasa ninguno de esos tests con material brillante.

**Cuándo revisarlo:** si aparece remuestreo en tiempo real (cambiar la velocidad de reproducción,
sincronizar a un BPM), el problema pasa a ser el que `rubato` resuelve bien y hay que volver a
él. Mientras la conversión sea offline y de ratio fijo, 130 líneas testeadas pesan menos que una
dependencia con un modelo de buffers propio.

## 3 · Dos reglas de accesibilidad de Biome desactivadas

`useKeyWithClickEvents` y `useSemanticElements`, ambas en `biome.json` con su motivo:

- **`useKeyWithClickEvents`**: toda acción de la app tiene atajo global en `src/lib/keymap.ts`.
  Añadir además un `onKeyDown` por elemento crearía una segunda fuente de verdad del teclado,
  que es justo lo que prohíbe el CLAUDE.md.
- **`useSemanticElements`**: la lista es un `listbox` ARIA virtualizado —un `<select>` no puede
  pintar 100.000 filas— con el patrón `aria-activedescendant` correctamente implementado. Y los
  overlays usan `role="dialog"` en vez de `<dialog>` porque el elemento nativo se queda con la
  tecla Esc, que gestiona el keymap global.

El resto de reglas de accesibilidad siguen activas y en verde.
