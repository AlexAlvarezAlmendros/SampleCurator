# SampleCurator

Aplicación de escritorio para **poner orden en una carpeta de samples**.

Tienes miles de sonidos sueltos, descargados de packs, sin nombre coherente y sin clasificar.
SampleCurator los recorre uno a uno, los reproduce **en el instante** en que los seleccionas y,
con **una sola tecla**, los manda a la carpeta que corresponda o a la papelera. Sin clics, sin
modales, sin esperas — y todo reversible.

```
 ┌──────────────────────────────────────────────────────────────┐
 │  ↓ / ↑      moverse (y escuchar al instante)                  │
 │  1 … 9      enviar al destino 1…9  y avanzar                  │
 │  X          rechazar (papelera)    y avanzar                  │
 │  Intro      conservar donde está   y avanzar                  │
 │  Espacio    repetir      ⇧Espacio  bucle                      │
 │  Ctrl+Z     deshacer     F2  renombrar     /  buscar          │
 │  ?          ver todos los atajos                              │
 └──────────────────────────────────────────────────────────────┘
```

## Qué lo hace rápido

| | |
|---|---|
| **2,6 ms** | de la tecla al sonido (p95, buffer de 256 frames) |
| **0,5 s** | escanear e indexar 50.000 archivos en frío |
| **16 ms** | cargar cualquier página de la lista, esté donde esté |
| **3 ms** | búsqueda incremental sobre 50.000 samples |
| **0** | clics necesarios en el bucle de triaje |

El motor de audio abre el dispositivo **una sola vez** al arrancar y mantiene en RAM los samples
vecinos al que estás mirando: cuando pulsas la flecha, el sonido ya está decodificado. Los
números salen de medir, no de suponer — ver [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md).

## Tus archivos, a salvo

Esta app mueve y aparta archivos de tu librería personal, así que:

- **Nunca borra nada.** Los rechazados van a `<destino>/.samplecurator-trash/` con un manifiesto
  que guarda de dónde venía cada uno. Vaciar esa papelera es la única acción irreversible de la
  app, y la única que pide confirmación.
- **Cada operación se apunta antes de tocar el disco.** Si se corta la luz a mitad, al arrancar
  se detecta y se repara; ante la duda, siempre se conserva el original.
- **Nunca sobrescribe.** Tres `kick.wav` distintos al mismo destino acaban como `kick.wav`,
  `kick (2).wav` y `kick (3).wav`.
- **Entre discos, copia, verifica por hash y solo entonces borra el origen.**
- **`Ctrl+Z` lo deshace todo**, incluidos lotes de 40 samples y renombrados, y te devuelve el
  foco al sample afectado.

## Instalar

Descarga el paquete de tu sistema de la última release, o constrúyelo tú.

**Linux** (`.deb` y `.AppImage`):

```bash
# Ubuntu 24.04
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
                    librsvg2-dev libasound2-dev build-essential curl file pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

pnpm install
pnpm tauri build     # → src-tauri/target/release/bundle/{deb,appimage}/
```

**Windows** (`.msi` y `.exe`):

```powershell
# Necesita Rust (rustup.rs), Node 22, pnpm 9 y las Build Tools de Visual Studio
# con la carga de trabajo «Desarrollo para el escritorio con C++».
# WebView2 ya viene con Windows 10 y 11; el instalador lo comprueba de todas formas.
pnpm install
pnpm tauri build     # → src-tauri\target\release\bundle\{msi,nsis}\
```

> **Estado de Windows:** el CI compila, pasa clippy y ejecuta los 110 tests del núcleo en
> `windows-latest`, y construye los dos instaladores. Lo que **no** está verificado es la
> ejecución real: nadie ha abierto todavía la app en Windows. La diferencia conocida es el
> audio: WASAPI no deja fijar el tamaño de buffer, así que allí manda el del sistema (~10 ms
> frente a los 2,6 ms medidos en Linux). Sigue siendo cómodamente imperceptible.

## Usar

1. `O` — elige la carpeta con tus samples. La lista aparece en segundo o dos; las duraciones y
   las ondas van llegando solas mientras ya estás triando.
2. `D` — elige la carpeta donde quieres construir tu librería ordenada.
3. Escribe los destinos (Kicks, Snares, FX…). Se les asigna la tecla `1`…`9` por orden, y si la
   carpeta de destino ya tiene subcarpetas, se te ofrecen directamente.
4. `↓` y a triar. Pulsa `?` en cualquier momento para ver el mapa de teclas completo.

Modo **mover** (por defecto) o **copiar**, por sesión. En modo copiar tu carpeta de origen no se
toca nunca, ni siquiera al rechazar.

## Desarrollo

```bash
pnpm tauri dev                                            # app con recarga en caliente
pnpm typecheck && pnpm biome check . && pnpm vitest run   # frontend
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml           # núcleo Rust
```

- Arquitectura → [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Sistema de diseño → [`docs/DESIGN_SYSTEM.md`](docs/DESIGN_SYSTEM.md)
- Flujo de triaje y atajos → [`docs/UX_TRIAGE.md`](docs/UX_TRIAGE.md)
- Modelo de datos → [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md)
- Rendimiento medido → [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md)
- Decisiones técnicas → [`docs/adr/`](docs/adr/)
- Plan de trabajo → [`docs/planning/ROADMAP.md`](docs/planning/ROADMAP.md)

## Stack

Tauri 2 (Rust) · React 19 + TypeScript · SQLite · `cpal` + `symphonia` para audio nativo.
El núcleo hace todo el trabajo pesado; el WebView solo pinta.

## Licencia

MIT — ver [LICENSE](LICENSE).
