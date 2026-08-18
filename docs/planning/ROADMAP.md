# SampleCurator — Roadmap del proyecto

> Última actualización: 2026-08-18

## Fases

| # | Fase | Estado | Plan | Hito |
|---|------|--------|------|------|
| 0 | Spike de latencia y escaneo | ✅ Hecho | [00-spike-latencia.md](plans/00-spike-latencia.md) | Tecla → sonido < 25 ms y 50k archivos escaneados < 60 s (**GATE GO/NO-GO**) |
| 1 | Fundaciones | ✅ Hecho | [01-fundaciones.md](plans/01-fundaciones.md) | La app arranca, con tokens, bindings IPC y CI local |
| 2 | Índice de biblioteca | ✅ Hecho | [02-indice-biblioteca.md](plans/02-indice-biblioteca.md) | Elegir carpeta → lista virtualizada de 50k samples con búsqueda |
| 3 | Motor de audio | ✅ Hecho | [03-motor-audio.md](plans/03-motor-audio.md) | Navegar con flechas y oír cada sample al instante, con waveform |
| 4 | Triaje | ✅ Hecho | [04-triaje.md](plans/04-triaje.md) | **MVP usable**: 1…9 clasifica, X rechaza, Ctrl+Z deshace |
| 5 | Calidad de vida | 🔄 Parcial | [05-calidad-de-vida.md](plans/05-calidad-de-vida.md) | Duplicados, filtros, renombrar, atajos configurables, BPM/tonalidad |
| 6 | Rendimiento y pulido | ✅ Hecho | [06-rendimiento-pulido.md](plans/06-rendimiento-pulido.md) | Todos los presupuestos de `docs/PERFORMANCE.md` verdes |
| 7 | Empaquetado | ✅ Hecho | [07-empaquetado.md](plans/07-empaquetado.md) | AppImage + .deb instalables, con actualizador |
| 8 | Clasificación automática | ⬜ Listo | [08-clasificacion-automatica.md](plans/08-clasificacion-automatica.md) | Tipo, BPM y tonalidad **medidos** contra material real etiquetado (**GATE en 8.0**) |

## Estado

**La aplicación está terminada y empaquetada.** El bucle completo funciona: añadir carpeta →
escuchar con las flechas → clasificar con una tecla → deshacer si te equivocas, sobre 50.000
samples reales y con todos los presupuestos de rendimiento en verde.

- Paquetes: `.deb` (6,3 MB) y AppImage (78 MB), ambos verificados ejecutándose.
- 98 tests: 66 en Rust (incluidos 12 de integración sobre archivos reales) y 32 en el frontend.
- `clippy -D warnings`, `biome check` y `tsc` limpios.

**Siguiente paso planificado:** la [Fase 8](plans/08-clasificacion-automatica.md) añade tipo, BPM
y tonalidad como etiquetas y filtros. El clasificador **no mueve nada**: etiqueta, y el humano
sigue decidiendo con su tecla. Empieza por un gate (8.0) que construye el conjunto de evaluación
antes que el clasificador, porque sin verdad de referencia no hay forma de saber si acierta.

**Lo que queda abierto**, con su motivo, en cada plan:

| Qué | Dónde | Motivo |
|---|---|---|
| BPM, tonalidad y clasificación automática | [Fase 8](plans/08-clasificacion-automatica.md) | **Ya planificada.** Fase propia con gate: primero el conjunto de evaluación, después el DSP |
| Etiquetas, paleta de comandos, A/B, atajos configurables | Fase 5 | Mejoras; el bucle ya cierra sin ellas |
| Mini-onda por fila y benches en criterion | [Fase 6](plans/06-rendimiento-pulido.md) | Necesitan una columna de picos reducidos y un canal binario por página |
| Actualizador automático | [Fase 7](plans/07-empaquetado.md) | **Bloqueado**: hacen falta tus claves de firma y decidir dónde se publican las releases |
| Windows y macOS en CI | Fase 7 | Se puede compilar, pero no verificar desde aquí |

## Lo que el spike dejó calibrado y hay que respetar al tocar el motor:

| Constante | Valor | Por qué |
|---|---|---|
| `BufferSize` | `Fixed(256)` | Con el default del device el p95 sube de 2,6 ms a **42 ms** |
| Fade al cambiar de sample | 5 ms | Elimina la discontinuidad por completo (medido) |
| Caché LRU + prefetch | obligatorios | Ahorran 9,8 ms de p95 (12,35 → 2,59) |
| Arcs que dejan de sonar | ring de basura al hilo de control | Soltarlos en el callback libera memoria en tiempo real |

## Grafo de dependencias

```
        ┌─────────────────┐
        │ 0 · Spike GATE  │
        └────────┬────────┘
                 ▼
        ┌─────────────────┐
        │ 1 · Fundaciones │
        └────────┬────────┘
                 ▼
        ┌─────────────────┐
        │ 2 · Índice      │──────────┐
        └────────┬────────┘          │
                 ▼                   ▼
        ┌─────────────────┐   (la búsqueda y los filtros
        │ 3 · Audio       │    de la fase 5 dependen del
        └────────┬────────┘    índice, no del audio)
                 ▼
        ┌─────────────────┐
        │ 4 · Triaje  MVP │
        └────────┬────────┘
                 ▼
        ┌─────────────────┐   ┌──────────────────┐   ┌────────────────┐
        │ 5 · Calidad     │──►│ 6 · Rendimiento  │──►│ 7 · Empaquetado│
        └─────────────────┘   └──────────────────┘   └────────────────┘
```

## Momento de corte

Al terminar la **Fase 4** la aplicación ya resuelve el problema completo: se puede triar una
carpeta entera de samples con una tecla por decisión. Las fases 5-7 son mejora, no requisito.
Si hay que parar, se para ahí y se usa.

## Leyenda de estados

| Icono | Significado |
|-------|-------------|
| ⬜ Listo / Pendiente | Sin bloqueos |
| 🔄 En curso | Trabajando ahora |
| ✅ Hecho | Completado |
| 🔒 Bloqueado | Espera a otra tarea |
| ❌ Cancelado | Fuera de alcance |

## Registro de fases

| Fecha | Fase | Nota |
|-------|------|------|
| 2026-08-18 | — | Documentación inicial, CLAUDE.md, skills y hooks creados |
| 2026-08-18 | 0 | Arranca el spike de latencia y escaneo (gate GO/NO-GO) |
| 2026-08-18 | 0 | ✅ **GO**: latencia de software p95 2,59 ms · escaneo de 50.000 en 0,52 s en frío · sin clics con fade de 5 ms |
