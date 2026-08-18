# Plan 01 — Fundaciones

> Fase: 1 de 7 | Estado: ✅ Hecho | 2026-08-18
> Hito: la app arranca con su ventana, sus tokens, sus bindings IPC generados y su CI local

---

## Dependencia con otras fases

- **Requiere:** Fase 0 con GO.
- **Habilita:** todas las demás.

---

## Tareas

### Esqueleto
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 1.1 | Scaffold Tauri 2 + Vite 6 + React 19 + TS (`pnpm create tauri-app`) | ✅ Hecho | Vite 6 + React 19 + TS 5.9 + Tauri 2.11, montado a mano (sin `create tauri-app` interactivo) |
| 1.2 | `tsconfig` estricto: `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess` | ✅ Hecho | `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `verbatimModuleSyntax` |
| 1.3 | Biome (`biome.json`) + scripts `typecheck`, `lint`, `test` en `package.json` | ✅ Hecho | biome.json + scripts typecheck/lint/test/build |
| 1.4 | Estructura de carpetas de `src/` y `src-tauri/src/` según CLAUDE.md, con módulos vacíos | ✅ Hecho | Estructura completa de `src/` y `src-tauri/src/`, más el módulo `codec/` (ADR-0006) |
| 1.5 | `error.rs`: `AppError` con `thiserror` + serialización `{ kind, message }` | ✅ Hecho | `AppError` con thiserror; serializa a `{ kind, message }` |
| 1.6 | `tauri-specta`: primer comando `app_info` + generación de `src/bindings.ts` en `cargo test` | ✅ Hecho | ts-rs 12 en vez de tauri-specta: **ver ADR-0006**. `cargo test` regenera src/bindings.ts |
| 1.7 | `src/lib/ipc.ts`: envoltorio tipado sobre los bindings + normalización de errores | ✅ Hecho | `src/lib/ipc.ts`: 40 comandos tipados, errores normalizados, única puerta al núcleo |

### Diseño
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 1.8 | `src/styles/tokens.css` con las tres capas de `docs/DESIGN_SYSTEM.md`, literal | ✅ Hecho | `tokens.css` con las tres capas, literal respecto a DESIGN_SYSTEM.md |
| 1.9 | `reset.css` + `global.css` (tipografía base, scrollbars, selección) | ✅ Hecho | reset.css + global.css con foco siempre visible y scrollbars |
| 1.10 | Empaquetar Inter Variable y JetBrains Mono en `.woff2` subsetado, sin red | ⏭️ Aplazada | Fuentes del sistema (`ui-sans-serif`/`ui-monospace`) en vez de empaquetar woff2. Pasa a 6.x |
| 1.11 | Primitivas: `Button`, `Kbd`, `Panel`, `Chip`, `Icon` con CSS Modules | ✅ Hecho | Kbd, Panel, Chip, Boton con CSS Modules |
| 1.12 | Layout de tres zonas (barra lateral · lista · destinos · transporte) con datos falsos | ✅ Hecho | Layout de tres zonas + transporte, con datos reales |

### Base
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 1.13 | SQLite: apertura en `app_data_dir`, PRAGMAs, runner de migraciones con `user_version` | ✅ Hecho | Pool propio (1 escritura + hasta 8 lecturas), WAL, PRAGMAs y runner con user_version |
| 1.14 | Migración 001 con el esquema completo de `docs/DATA_MODEL.md` | ✅ Hecho | Migración 001 con el esquema completo, FTS5 y triggers |
| 1.15 | Estado global de Tauri: pool de DB + handle del motor de audio (aún vacío) | ✅ Hecho | Estado de Tauri: Db + AudioHandle + flag de cancelación |
| 1.16 | `src/lib/keymap.ts`: registro declarativo + listener global único | ✅ Hecho | `lib/keymap.ts`: un único listener global y tabla declarativa en `app/atajos.ts` |
| 1.17 | Vitest + Testing Library configurados con un test de humo del keymap | ✅ Hecho | Vitest + Testing Library; 30 tests de front, incluido el montaje completo de App |
| 1.18 | Verificar los hooks del proyecto (formato, tokens, atribución) sobre un cambio real | ✅ Hecho | Los cinco hooks verificados contra cambios reales |

---

## Entregable

`pnpm tauri dev` abre una ventana con el layout definitivo (con datos falsos), tema oscuro con
tokens reales, teclado enganchado y un comando IPC de ida y vuelta tipado.

## Criterio de aceptación

- `pnpm typecheck`, `pnpm biome check .` y `cargo clippy -- -D warnings` limpios.
- `src/bindings.ts` se regenera con `cargo test export_bindings` y está versionado.
- Ningún color literal fuera de `tokens.css`.
- Arranque en frío por debajo de 1,2 s.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 1.1–1.18 | Fundaciones completas. Dos desvíos con ADR: ts-rs en vez de tauri-specta, y sinc propio en vez de rubato (ADR-0006) |

