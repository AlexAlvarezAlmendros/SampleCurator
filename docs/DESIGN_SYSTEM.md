# Sistema de diseño — SampleCurator

> Este documento es la fuente de verdad visual. `src/styles/tokens.css` es su implementación
> literal: si aquí cambia un valor, cambia allí, y en ningún otro sitio.

---

## 1. La idea

Una herramienta de estudio, no una web bonita. El usuario está escuchando: la vista tiene que
darle información sin pedirle atención. Piensa en la mesa de un ingeniero de sonido — superficie
oscura, etiquetas legibles a distancia, el vúmetro es lo único que brilla.

**Cinco principios, en orden de prioridad. Cuando dos choquen, gana el de arriba:**

1. **El sonido manda.** La UI nunca compite con lo que se está escuchando. Sin animaciones
   llamativas, sin colores que salten, sin nada que se mueva mientras suena un sample salvo el
   cabezal de reproducción.
2. **Teclado primero.** Cada acción del bucle de triaje tiene tecla, y esa tecla se ve escrita
   en la interfaz. El ratón es el camino lento y opcional.
3. **Densidad honesta.** Ver 30 samples a la vez es una funcionalidad. Se aprieta el espaciado
   hasta el límite de lo legible, no más.
4. **Todo es reversible y se nota.** Cada acción destructiva deja un rastro visible y un
   `Ctrl+Z` a mano. Cero diálogos de confirmación: confirmar es más lento que deshacer.
5. **Instantáneo o no existe.** Cualquier cosa que tarde más de 100 ms necesita feedback en el
   mismo frame de la pulsación, aunque el resultado llegue después.

---

## 2. Color

### Capa 1 — primitivos

Escala neutra fría de 12 pasos (modelo Radix: cada paso tiene un trabajo asignado) más las
familias de acento. **Solo existen en `tokens.css`.**

```
--gray-1   #0b0c0d   fondo de la app
--gray-2   #111315   fondo de superficie
--gray-3   #17191c   fondo de elemento (fila par, input)
--gray-4   #1d2024   fondo de elemento hover
--gray-5   #24272c   fondo de elemento activo
--gray-6   #2b2f35   borde sutil (separadores)
--gray-7   #363b42   borde de elemento
--gray-8   #454b54   borde de foco / hover fuerte
--gray-9   #6b7280   texto deshabilitado, iconos apagados
--gray-10  #8b9199   texto secundario
--gray-11  #b4bac1   texto de apoyo
--gray-12  #e8eaed   texto principal

--accent-9   #4dd4ac   acento (verde menta): selección, waveform, foco
--accent-10  #67e6c0   acento hover
--accent-4   #12332c   fondo de acento tenue (fila seleccionada)

--keep-9     #4dd4ac   conservar / enviado a destino
--reject-9   #e06c6c   rechazar
--warn-9     #e0b25c   aviso (duplicado, archivo dañado)
```

Colores de destino (para los cubos 1..9). Hues distinguibles a alta densidad y con daltonismo
protán/deután en mente — nunca son el único portador de información: siempre van con número y nombre.

```
--dest-1 #4dd4ac   --dest-2 #5aa9e6   --dest-3 #b58cf0
--dest-4 #e6a35a   --dest-5 #e06c9f   --dest-6 #7ed957
--dest-7 #e0d35c   --dest-8 #5ce0d3   --dest-9 #9aa4b0
```

### Capa 2 — semánticos

Es la capa que usan los componentes. Un componente **jamás** referencia `--gray-7`.

```
--color-bg-app          --gray-1
--color-bg-surface      --gray-2
--color-bg-element      --gray-3
--color-bg-hover        --gray-4
--color-bg-active       --gray-5
--color-bg-selected     --accent-4

--color-border-subtle   --gray-6
--color-border          --gray-7
--color-border-strong   --gray-8

--color-text            --gray-12
--color-text-muted      --gray-10
--color-text-subtle     --gray-9
--color-text-accent     --accent-9

--color-focus-ring      --accent-9
--color-waveform        --accent-9
--color-waveform-idle   --gray-8
--color-playhead        --gray-12
```

### Capa 3 — de componente

```
--row-height            28px    (densidad compacta: 24px · cómoda: 32px)
--waveform-height       72px
--sidebar-width         240px
--transport-height      88px
```

### Reglas

- **Prohibido** cualquier `#hex`, `rgb()`, `hsl()` u `oklch()` literal fuera de `tokens.css`.
  Hay un hook que lo detecta y lo devuelve para corregir.
- Modo claro: se define **la misma capa semántica** con los primitivos invertidos, bajo
  `[data-theme="light"]`. Ningún componente sabe en qué tema está. Es Fase 6, no MVP —
  pero la arquitectura de tokens ya lo permite desde el día uno.
- El color nunca es el único portador de significado: destino = color **+** número **+** nombre.

---

## 3. Tipografía

| Uso | Familia | Tamaño / interlineado | Peso |
|---|---|---|---|
| Nombres de archivo, rutas, tiempos, cifras | **JetBrains Mono** | 12 / 16 | 400 |
| Etiquetas de UI, botones, cabeceras | **Inter Variable** | 12 / 16 | 500 |
| Títulos de panel | Inter Variable | 13 / 20 | 600 |
| Texto de ayuda y estados vacíos | Inter Variable | 12 / 18 | 400 |
| Cifras grandes (contadores de sesión) | JetBrains Mono | 18 / 24 | 500 |

Escala completa: `11 · 12 · 13 · 15 · 18 · 24`. Seis pasos, ni uno más.

**Por qué monoespaciada para los nombres de archivo:** en una lista de 30 filas con nombres como
`KICK_808_LONG_02.wav`, la anchura constante alinea prefijos y sufijos verticalmente y el ojo
escanea la columna en un barrido. Es la diferencia entre leer y buscar.

Las dos fuentes se empaquetan con la app (`.woff2` subsetado). Sin fuentes de red: la app arranca
sin conexión y sin salto de layout.

---

## 4. Espaciado y rejilla

Base **4 px**. Escala: `2 · 4 · 6 · 8 · 12 · 16 · 24 · 32 · 48`.

- Padding horizontal de fila: 8 px. Separación entre columnas de fila: 12 px.
- Padding de panel: 12 px. Separación entre paneles: 1 px de borde, sin hueco.
- La app es una rejilla fija de tres zonas, sin márgenes exteriores: la ventana se llena entera.

```
┌────────────┬──────────────────────────────────────┬──────────────┐
│  Fuentes   │  Lista de samples (virtualizada)     │   Destinos   │
│  Filtros   │                                      │   1 Kicks 42 │
│  240 px    │  ← el 100 % del espacio sobrante →   │   2 Snares 8 │
├────────────┴──────────────────────────────────────┴──────────────┤
│  Transporte + waveform + progreso de sesión         88 px mínimo  │
└───────────────────────────────────────────────────────────────────┘
```

> **Los altos son mínimos Y el transporte no se encoge.** Hacen falta las dos cosas, y saberlo
> costó dos intentos:
>
> 1. `min-height` en vez de `height`, porque un alto fijo con contenido variable recorta siempre.
> 2. `flex: 0 0 auto`, que es lo que faltaba. En una columna flex el espacio que falta se
>    reparte **entre los hijos** en proporción a su tamaño: el transporte también se encogía
>    —hasta su mínimo— mientras su contenido pedía más, y el sobrante lo cortaba el
>    `overflow: hidden` del body. Con `flex-shrink: 0` el alto lo manda su contenido y es la
>    lista, que sí puede, la que cede el espacio.
>
> Para comprobarlo sin abrir la app: `./scripts/captura.sh 1362 861` y mirar la imagen. Es la
> forma de no volver a descubrir un recorte porque alguien mande una captura.
>
> La única altura de verdad fija es `--row-height`, y lo es porque el virtualizador necesita
> ese número exacto para saltar a cualquier posición sin medir. Por eso la densidad configurable
> vive en el estado de la app y no en una media consulta al CSS: si el token cambiara por su
> cuenta, la lista mediría mal.

---

## 5. Forma y profundidad

- Radios: `4px` (chips, teclas), `6px` (botones, inputs), `10px` (paneles flotantes). Nada más.
- **Sin sombras difusas.** La jerarquía se construye con fondo + 1 px de borde. Solo los overlays
  (paleta de comandos) llevan sombra: `0 8px 32px rgba(0,0,0,.5)`.
- Sin gradientes, salvo el degradado vertical sutil de la waveform.
- Bordes de 1 px siempre; el foco es el único borde de 2 px.

---

## 6. Movimiento

| Qué | Duración | Curva |
|---|---|---|
| Cambio de estado de fila (hover, selección) | **0 ms** | ninguna — la selección es instantánea |
| Micro-interacción (botón, chip, contador) | 80 ms | `cubic-bezier(.2, .8, .2, 1)` |
| Panel que aparece / desaparece | 140 ms | `cubic-bezier(.2, .8, .2, 1)` |
| Cabezal de reproducción | continuo | `requestAnimationFrame`, lineal |

Solo se animan `opacity` y `transform`. Nunca `height`, `width`, `top` ni nada que provoque
reflow: rompería el presupuesto de 16 ms de la lista.

La selección **no se anima**. Al pulsar `↓` la fila nueva está seleccionada en el mismo frame;
una transición de 80 ms aquí se percibe como lentitud, no como suavidad.

`@media (prefers-reduced-motion: reduce)` → todas las duraciones a 0.

---

## 7. Foco y estado

La app se conduce con el teclado, así que **el foco siempre es visible**. No hay `:focus-visible`
condicional que lo esconda con el ratón.

```css
outline: 2px solid var(--color-focus-ring);
outline-offset: -2px;   /* hacia dentro: en una lista densa, el offset positivo se solapa */
```

Estados de una fila, en orden de precedencia visual:

1. **Reproduciendo** — barra de 2 px en color acento en el borde izquierdo + nombre en `--color-text`.
2. **Seleccionada** — fondo `--color-bg-selected`.
3. **Hover** — fondo `--color-bg-hover`.
4. **Ya decidida** — nombre en `--color-text-subtle` + chip del destino a la derecha.
5. **Normal**.

Seleccionada y reproduciendo casi siempre coinciden (autoplay); el diseño debe seguir siendo
legible cuando **no** coinciden (el usuario navega mientras suena otra cosa).

---

## 8. Los componentes que importan

### `Row` — la fila de sample
Altura fija `--row-height`. Cinco columnas: indicador de reproducción (4 px) · nombre (mono,
truncado por el **centro**, no por el final: `KICK_808…_02.wav` conserva la extensión) · duración
· formato (frecuencia y canales) · chip de estado.

> La mini-onda de 64×16 px por fila está **aplazada a la Fase 6**: pintar 35 por pantalla exige
> una columna de picos reducidos (~32 buckets, 64 bytes) y un canal binario para la página
> entera; hacerlo con el endpoint actual serían 35 llamadas IPC por scroll. Mientras tanto esa
> columna la ocupa el formato, que es lo segundo que se mira al triar. Sin bordes horizontales: la alternancia de fondo
(`--gray-2` / `--gray-1`) basta y pesa menos.

### `Waveform` — canvas
Se pinta desde el BLOB de picos: una barra vertical por columna de píxel, `min`→`max`.
Reproducida en `--color-waveform`, pendiente en `--color-waveform-idle`, cabezal 1 px en
`--color-playhead`. Redibujado completo solo al cambiar de sample; el cabezal va en un canvas
superpuesto para no repintar la onda 60 veces por segundo.

### `Kbd` — la tecla
Cada acción de la UI muestra su atajo con este componente: fondo `--color-bg-element`, borde
`--color-border`, radio 4 px, mono 11 px. Es el mecanismo por el que el usuario aprende a no
usar el ratón. Si una acción no tiene `Kbd`, es que falta la tecla.

### `DestinationBucket` — el cubo de destino
Número grande a la izquierda (mono 18 px, en `--dest-N`), nombre, contador. Al recibir un sample
parpadea 80 ms en su color y el contador incrementa. Es el único feedback de que la tecla funcionó
— y por eso tiene que ser inmediato y estar siempre visible en pantalla.

### `SessionProgress`
`428 / 3.211` en mono, barra de 2 px. Sin porcentajes redondeados: el número exacto es la
recompensa del triaje.

---

## 9. Voz y microcopy

- Español, tuteo, frases cortas. `Sin analizar`, no `Este archivo aún no ha sido analizado`.
- Los errores dicen qué pasó y qué hacer: `No se pudo mover: el destino está lleno. Libera espacio o cambia de carpeta.`
- Cero signos de exclamación. Cero emojis en la UI (sí en la documentación).
- Los estados vacíos proponen la acción siguiente con su tecla: `Arrastra una carpeta aquí o pulsa O para abrir una`.

---

## 10. Accesibilidad

- Contraste mínimo AA (4,5:1) para texto; los `--gray-10` sobre `--gray-1` y superiores cumplen.
- Todo lo accionable es alcanzable por teclado, y el orden de tabulación sigue al visual.
- Los colores de destino se acompañan siempre de número y nombre.
- Objetivos de ratón ≥ 24 px de alto aunque la fila mida 28 px (el hit area cubre la fila entera).
- `prefers-reduced-motion` respetado; nada parpadea más de 3 veces por segundo.

---

## 11. Checklist antes de dar por buena una pantalla

- [ ] ¿Ningún color literal fuera de `tokens.css`?
- [ ] ¿Toda acción tiene atajo y el atajo se ve en pantalla?
- [ ] ¿El foco es visible en cada elemento navegable?
- [ ] ¿La selección responde en el mismo frame, sin transición?
- [ ] ¿Se ven ≥ 25 filas en una ventana de 900 px de alto?
- [ ] ¿Funciona sin ratón, de principio a fin?
- [ ] ¿Cada acción destructiva es deshacible y lo indica?
