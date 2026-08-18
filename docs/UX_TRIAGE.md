# Flujo de triaje y mapa de teclas

El producto entero es este bucle. Todo lo demás existe para que este bucle sea rápido.

---

## 1. El bucle

```
                    ┌──────────────────────────────┐
                    │  suena el sample enfocado    │  ← automático al enfocar
                    └──────────────┬───────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
        1…9  me gusta        X  no me gusta       ↓  todavía no sé
     va al destino N       va a la papelera        paso al siguiente
              │                    │                    │
              └────────────────────┴────────────────────┘
                                   │
                          avanza a la siguiente fila
```

Una decisión = **una tecla**. La mano izquierda vive en `1…9` y `X`; la derecha en las flechas.
No hay confirmaciones: `Ctrl+Z` es más rápido que cualquier diálogo.

**Lo que hace que se sienta bien:**

- Al enfocar una fila, suena. Sin pulsar nada más.
- Al decidir, avanza sola. La siguiente ya está sonando antes de que sueltes la tecla.
- El sample siguiente ya está decodificado en RAM (prefetch de ±3): nunca hay silencio de espera.
- El contador del destino parpadea al recibir: sabes que la tecla entró sin mirar dos veces.

---

## 2. Preparar una sesión (una vez, ~30 s)

1. `O` → elegir la carpeta de origen (la del desorden). Empieza a indexar y la lista aparece
   antes de terminar.
2. `D` → elegir la carpeta raíz de destino (la librería ordenada).
3. Definir los destinos: escribir un nombre y se asigna la siguiente tecla libre `1…9`.
   Si la carpeta ya tiene subcarpetas, se ofrecen como destinos automáticamente.
4. `Enter` → empieza el triaje.

Modo `mover` (por defecto) o `copiar`, por sesión. La sesión se guarda: al reabrir la app se
retoma en el mismo sample donde lo dejaste.

---

## 3. Mapa de teclas

### Navegación
| Tecla | Acción |
|---|---|
| `↓` / `J` | Siguiente sample (y suena) |
| `↑` / `K` | Sample anterior (y suena) |
| `PageDown` / `PageUp` | Saltar 10 |
| `Home` / `End` | Primero / último |
| `Tab` | Cambiar de panel (fuentes → lista → destinos) |

### Escucha
| Tecla | Acción |
|---|---|
| `Espacio` | Repetir desde el principio |
| `⇧ Espacio` | Loop on/off |
| `←` / `→` | Retroceder / avanzar 0,5 s |
| `S` | Silenciar / reanudar |
| `+` / `-` | Volumen |
| `N` | Normalizar volumen de escucha on/off (usa `loudness_db`) |
| `⇧ A` | Autoplay on/off (para navegar sin que suene) |

### Decisión
| Tecla | Acción |
|---|---|
| `1` … `9` | Enviar al destino N y avanzar |
| `X` / `Supr` | Rechazar → papelera y avanzar |
| `Enter` | Marcar como conservado *en su sitio* y avanzar |
| `F` | Favorito (⭐) sin mover |
| `1`…`5` con `⌥` | Valoración de 1 a 5 |
| `Ctrl+Z` | Deshacer la última acción (devuelve el archivo y el foco) |
| `Ctrl+⇧+Z` | Rehacer |

### Selección múltiple
| Tecla | Acción |
|---|---|
| `⇧ ↓` / `⇧ ↑` | Extender selección |
| `Ctrl+A` | Seleccionar todo lo filtrado |
| `1`…`9` con selección múltiple | Enviar el lote (un solo undo) |

### Buscar y filtrar
| Tecla | Acción |
|---|---|
| `/` | Buscar (incremental, sobre nombre y ruta) |
| `Esc` | Cerrar búsqueda / limpiar filtro |
| `Ctrl+K` | Paleta de comandos |
| `⇧ D` | Filtrar solo duplicados |
| `⇧ P` | Filtrar solo pendientes |
| `F2` | Renombrar el archivo (en la propia barra, sin diálogo) |
| `Ctrl+R` | Revelar en el explorador de archivos |
| `Ctrl+E` | Guardar las decisiones en `<destino>/library.json` |
| `T` | Cambiar entre tema oscuro y claro |
| `Ctrl+,` | Ajustes: carpetas, apariencia, escucha, papelera e información |
| `?` | Ver el mapa de teclas completo |

El mapa vive en un único sitio declarativo (`src/app/atajos.ts`), y la pantalla de ayuda (`?`)
se genera desde esa misma tabla: si se añade una acción, aparece documentada sola. Hacerlos
reconfigurables es Fase 5 y solo toca ese archivo. Existe un preset **una sola mano** que mueve `1…9` a `Q W E A S D Z X C`
para triar con la izquierda mientras la derecha sigue en el ratón o en el teclado del estudio.

---

## 4. Reglas del flujo

**Autoplay.** Activado por defecto. Al enfocar, reproduce desde 0. Si el sample dura más de 8 s,
empieza en el punto de mayor energía (se conoce por los picos) — en un loop de 4 compases lo
interesante nunca está en el primer milisegundo.

**Avance automático.** Después de `1…9`, `X` o `Enter`, el foco baja una fila. Si la fila era la
última del filtro, se queda ahí y avisa: `Fin de la lista — 3.211 revisados`.

**Nada bloquea.** Mover un archivo no congela la UI: la fila se marca en el acto (optimista) y si
la operación falla, vuelve a su sitio con un aviso en la barra inferior. El usuario ya está tres
samples más abajo.

**Deshacer de verdad.** `Ctrl+Z` devuelve el archivo a su ruta original, restaura el estado en el
índice, decrementa el contador del destino y **mueve el foco a esa fila**, reproduciéndola. Un
undo que no te devuelve a donde estabas no sirve de nada.

**Papelera, no borrado.** `X` mueve a `<destino>/.samplecurator-trash/`. Se vacía manualmente
desde ajustes, con un aviso que dice cuántos archivos y cuántos MB. Ese es el único diálogo de
confirmación de toda la app, y está justificado: es la única acción irreversible.

**Duplicados.** Al detectar que un sample tiene el mismo `content_hash` que otro ya conservado,
la fila muestra un chip `dup` en `--warn-9` y `⇧D` filtra solo esos. Nunca se borra un duplicado
automáticamente.

---

## 4.bis Las carpetas, sobre la marcha

La barra lateral no es solo una lista: cada carpeta lleva sus acciones, que aparecen al pasar
por encima.

| Acción | Dónde | Qué hace |
|---|---|---|
| **+** | Cabecera de «Carpetas» | Añade otra carpeta de samples sin pasar por el asistente |
| **↻** | En la fila de la carpeta | Vuelve a recorrerla: entra lo nuevo, se actualiza lo que cambió y se poda lo que ya no está en disco |
| **×** | En la fila de la carpeta | La quita del índice. **No borra ningún archivo** |

Quitar una carpeta pregunta **en la propia fila** («¿Quitar del índice? Sí / No»), no en un
diálogo: el flujo de esta app no abre modales. Lo que sí conviene saber es que se pierden las
decisiones tomadas sobre esos samples, porque el índice es donde viven.

Reescanear respeta el triaje: lo que hayas movido a un destino sigue donde lo pusiste y no
reaparece como pendiente.

## 5. Estados vacíos y errores

| Situación | Qué se ve |
|---|---|
| Sin carpetas | `Arrastra una carpeta de samples aquí, o pulsa O para abrirla` |
| Indexando | La lista ya poblada + barra fina de progreso arriba: `12.480 / ~50.000` |
| Sin resultados de búsqueda | `Nada coincide con "kick 808". Esc para limpiar` |
| Archivo ilegible | Fila en `--color-text-subtle` con chip `dañado`; se salta en el autoplay |
| Destino inaccesible | Aviso persistente en la barra inferior + las teclas de ese destino quedan inertes |

---

## 6. Qué queda fuera del MVP (y cuándo entra)

| Idea | Fase |
|---|---|
| Detección de BPM y tonalidad | 5 |
| Etiquetado automático por tipo (kick/snare/pad…) | 5+ (requiere modelo local) |
| Comparar dos samples A/B | 5 |
| Arrastrar desde la app al DAW | 6 |
| Sincronizar la librería entre equipos | fuera de alcance |
