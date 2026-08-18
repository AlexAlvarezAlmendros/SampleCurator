# ADR-0001 — Tauri 2 como shell de la aplicación

**Fecha:** 2026-08-18 · **Estado:** aceptada

## Contexto

Necesitamos una app de escritorio (no web) para Linux, con posibilidad de Windows y macOS más
adelante. Los requisitos duros: latencia de audio < 25 ms, listas de 100.000 elementos fluidas,
acceso completo al sistema de archivos, arranque rápido y una UI muy cuidada y muy personalizada
(waveforms, densidad alta, todo conducido por teclado).

## Opciones consideradas

| Opción | A favor | En contra |
|---|---|---|
| **Tauri 2** (Rust + WebView) | Núcleo Rust real para audio/E-S; binario ~10 MB; RAM ~100 MB; UI web con toda su expresividad; multiplataforma | Dos lenguajes; el puente IPC hay que diseñarlo bien |
| Electron | Un solo lenguaje; ecosistema enorme; Web Audio a mano | 150-250 MB de RAM en reposo, ~120 MB de binario; el trabajo pesado cae en JS o en un addon nativo igualmente |
| Rust nativo (egui / iced) | Sin puente, máximo control | Construir el sistema de diseño y la accesibilidad cuesta 3-5× más; texto y layout complejos son dolorosos |
| Flutter desktop | Buen render, un lenguaje | El audio de baja latencia en Linux depende de plugins flojos; el binario es grande; menos control del sistema de archivos |
| Qt / C++ | Maduro para audio pro | Velocidad de desarrollo baja y ergonomía pobre para el estilo de UI que queremos |

## Decisión

**Tauri 2.**

El reparto encaja exactamente con el problema: la parte difícil (audio en tiempo real, escanear
decenas de miles de archivos, decodificar, mover archivos con seguridad) es justo donde Rust
brilla, y la parte cara en horas (una UI densa, con waveforms, atajos y estados finos) es justo
donde el stack web brilla. Electron nos daría la misma UI pero nos obligaría a escribir el núcleo
en un addon nativo de todas formas — con el coste de RAM y arranque encima.

## Consecuencias

- Hay un contrato IPC que mantener. Se mitiga generando los tipos TS desde Rust (`tauri-specta`)
  y prohibiendo `invoke()` fuera de `src/lib/ipc.ts`.
- Todo lo caro debe empujarse a Rust conscientemente: si empieza a aparecer lógica pesada en el
  WebView, es una señal de que el diseño se está torciendo.
- En Linux dependemos de WebKitGTK (ya presente en Ubuntu 24.04). Su rendimiento es inferior al de
  Chromium en animaciones complejas — otra razón para que la UI sea sobria y sin animaciones caras.
