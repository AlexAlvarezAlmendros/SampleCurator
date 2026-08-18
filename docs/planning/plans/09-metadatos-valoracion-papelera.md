# Plan 09 — Metadatos, valoración y papelera

> Fase: 9 | Estado: ✅ Hecho | 2026-08-18
> Hito: poder describir un sample (estrellas, etiquetas, notas), encontrarlo por cualquiera de
> esas cosas, y recuperar lo que se rechazó sin salir de la app.

---

## Dependencia con otras fases

- **Requiere:** Fase 4 (el triaje y su journal) y la Fase 8 gate (las etiquetas por origen).
- **Habilita:** nada. Es mejora sobre una app que ya funciona.

## Dos decisiones tomadas antes de escribir código

**1 · Los metadatos viven en la app, no dentro de los archivos.** Etiquetas, notas, valoración y
correcciones de BPM o tonalidad se guardan en el índice. Escribir dentro del `.wav` o del `.mp3`
—RIFF INFO, ID3— significaría reescribir archivos del usuario, con su copia de seguridad, su
verificación y su vuelta atrás; y los DAW mayormente ignoran esas etiquetas en samples. Si algún
día hace falta, se añade encima de esto sin tirar nada.

**2 · La papelera se ve y se restaura desde la app.** Ya existía por dentro desde la Fase 4
—lo rechazado va a `<destino>/.samplecurator-trash/` con su manifiesto— pero era invisible: la
única forma de recuperar algo era `Ctrl+Z` inmediatamente. Ahora hay una vista donde escuchar lo
rechazado antes de decidir y devolverlo a su carpeta original de uno en uno.

---

## Lo que ya existía y lo que faltaba de verdad

La auditoría antes de planificar, porque media petición ya estaba a medias:

| Pedido | Lo que había | Lo que falta |
|---|---|---|
| Valoración por estrellas | Columna `rating` 0-5, comando para ponerla y `F` para 5 estrellas | **Ninguna forma de poner 1-4**, y en la fila solo se ve un ★ cuando llega a 5 |
| Editar metadatos | Renombrar con `F2`; tablas `tags` y `sample_tags` creadas en la migración 001 | Nadie las usa. Sin etiquetas, sin notas |
| Control de volumen | `+` y `−` por teclado, y un botón que dice «vol 90 %» | **El botón silencia en vez de ajustar**, no hay deslizador, y el atajo falla según la distribución del teclado |
| Filtrar clasificados / sin clasificar | Filtros «Pendientes» y «Decididos» | Ya estaba; se renombra a «Sin clasificar» / «Clasificados», que es como lo llama el usuario |
| Filtrar por destino | Nada | Todo |
| Filtrar por valoración | Tres botones: sin filtro / ★3+ / ★5+ | Por estrella concreta |
| Papelera | Carpeta gestionada, manifiesto, resumen y vaciar | **No se puede ver ni restaurar** |

---

## Tareas

### Núcleo Rust

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 9.1 | Migración 003: `samples.notes`, índices para filtrar por destino y por valoración | ✅ Hecho | Migración 003: `notes` e índices parciales para destino y valoración |
| 9.2 | Consultas de etiquetas: crear, asignar, quitar, listar y contar usos | ✅ Hecho | `db/tags.rs` con normalización (« 808 » y «808» son la misma) y limpieza de huérfanas |
| 9.3 | `LibraryQuery` gana `destId`, `tag`, y la valoración pasa a rango exacto | ✅ Hecho | `destId`, `tag`, `unrated` y valoración mínima en la consulta |
| 9.4 | Notas por sample (texto libre) | ✅ Hecho | Notas por sample; vacías se guardan como NULL, no como cadena vacía |
| 9.5 | `trash_list`: leer el manifiesto y la carpeta, cruzarlo con el índice | ✅ Hecho | `trash_list` cruza manifiesto, carpeta e índice; lo huérfano también se lista |
| 9.6 | `trash_restore`: devolver un archivo a su ruta original, con journal y sin sobrescribir | ✅ Hecho | `trash_restore` con journal, sin sobrescribir y devolviendo el sample a la cola |
| 9.7 | Tests sobre `TempDir`: restaurar, colisión al restaurar, y entrada huérfana | ✅ Hecho | 4 tests sobre TempDir: restaurar, colisión, huérfano y limpieza del manifiesto |

### Interfaz

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 9.8 | Componente de estrellas: se ve en la fila y se puede pulsar | ✅ Hecho | `components/Estrellas`, en la fila y en el inspector. Pulsar la puesta la quita |
| 9.9 | `Alt+1…5` pone la valoración, `Alt+0` la quita | ✅ Hecho | `Alt+1…5` y `Alt+0` |
| 9.10 | Inspector (`I`): etiquetas, notas, valoración y datos del archivo | ✅ Hecho | Inspector (`I`) como tercer modo del panel derecho: etiquetas, notas, valoración y datos |
| 9.11 | Deslizador de volumen en el transporte, y arreglo de `+` / `−` | ✅ Hecho | Deslizador hasta 150 %, y **el bug**: el atajo exigía que no hubiera Shift |
| 9.12 | Filtro por destino en la barra lateral | ✅ Hecho | Filtro por destino, con el color del cubo |
| 9.13 | Filtro por valoración estrella a estrella | ✅ Hecho | Filtro estrella a estrella, más «sin valorar» |
| 9.14 | Filtro por etiqueta | ✅ Hecho | Filtro por etiqueta con las doce más usadas |
| 9.15 | Vista de papelera con escuchar, restaurar y vaciar | ✅ Hecho | Panel de papelera (`⇧X`) con escuchar, restaurar y vaciar |
| 9.16 | Tests de todo lo anterior | ✅ Hecho | 49 tests en el frontend y 119 en Rust |

---

## El bug del volumen, con nombre

El atajo era `algunaTecla(["+", "="])`, y `tecla()` exige que **no** haya modificadores. En la
mayoría de distribuciones el `+` se escribe con `Shift`, así que `e.shiftKey` valía `true` y el
atajo no disparaba nunca. Encima, el único control visible («vol 90 %») era un botón que
silenciaba: quien lo pulsaba buscando subir el volumen se quedaba sin sonido.

Se arregla por los dos lados: los atajos de volumen ignoran el estado de `Shift` —es la tecla lo
que importa, no cómo la produzca tu teclado— y el transporte gana un deslizador de verdad.

## Entregable

Un sample se puede describir (estrellas, etiquetas, notas) y encontrar por cualquiera de esas
cosas, además de por destino. El volumen se ajusta con el ratón o con el teclado. Y lo rechazado
se puede escuchar y devolver a su sitio sin depender de haber pulsado `Ctrl+Z` a tiempo.

## Criterio de aceptación

- Poner 3 estrellas a un sample y encontrarlo filtrando por ★3, en menos de 50 ms sobre 50.000.
- Restaurar desde la papelera devuelve el archivo a su ruta original, sin sobrescribir nada, y
  con test sobre `TempDir` que lo demuestre.
- El volumen sube y baja con el deslizador y con las teclas, sea cual sea la distribución.
- Los archivos del usuario no se modifican nunca: los metadatos viven en el índice.

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | — | Plan escrito. Auditoría previa: media petición ya existía a medias y el control de volumen estaba roto por un atajo que exigía que no hubiera Shift |
| 2026-08-18 | 9.1–9.7 | Núcleo: etiquetas, notas, filtros y papelera restaurable. 4 tests nuevos sobre archivos reales |
| 2026-08-18 | 9.11 | Confirmado el bug del volumen: `tecla("+")` exigía `shiftKey === false`. Nuevo comparador que ignora Shift, porque lo que importa es el carácter, no cómo lo produzca tu teclado |
| 2026-08-18 | 9.8–9.16 | Interfaz completa. Dos tests fallaron por ambigüedad de texto («Kicks» está en el filtro y en el cubo): se resolvió dando etiqueta accesible al filtro, que además mejora la accesibilidad |
