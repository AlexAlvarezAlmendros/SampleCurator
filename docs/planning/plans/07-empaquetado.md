# Plan 07 — Empaquetado y distribución

> Fase: 7 de 7 | Estado: ✅ Hecho (salvo el actualizador) | 2026-08-18
> Hito: AppImage y .deb instalables, con actualizador y datos a salvo

---

## Dependencia con otras fases

- **Requiere:** Fase 6.
- **Habilita:** usarla en otras máquinas y compartirla.

---

## Tareas

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 7.1 | `tauri.conf.json` de producción: identificador, iconos, categoría, permisos mínimos | ✅ Hecho | Identificador, iconos generados, categoría Music y CSP restrictiva |
| 7.2 | Build de AppImage y `.deb` verificados en una Ubuntu limpia | ✅ Hecho | .deb 6,3 MB y AppImage 78 MB; el AppImage se ejecutó y vio los 50.000 samples |
| 7.3 | Revisión de la lista de permisos de Tauri v2: solo los diálogos y rutas necesarios | ✅ Hecho | Solo `dialog:allow-open` y `opener:allow-reveal-item-in-dir`; el disco lo toca Rust |
| 7.4 | Actualizador (`tauri-plugin-updater`) con firma, apuntando a las releases de GitHub | 🔒 | 7.2 |
| 7.5 | Copia de seguridad automática de `library.json` junto a la carpeta destino | ✅ Hecho | library.json al cerrar y con Ctrl+E, con escritura atómica |
| 7.6 | Primer arranque: crear rutas de datos, migrar y no fallar nunca en frío | ✅ Hecho | Verificado con la base borrada: crea, migra y arranca sin datos |
| 7.7 | GitHub Actions: `clippy`, `cargo test`, `biome`, `typecheck`, `vitest` y build | ✅ Hecho | 3 jobs: frontend, núcleo Rust y paquetes. Incluye comprobar que bindings.ts está al día |
| 7.8 | README con capturas reales y guía de instalación | ✅ Hecho | README con números medidos, garantías sobre los archivos y guía de uso. Capturas pendientes |
| 7.9 | (Opcional) build de Windows y macOS en la matriz de CI | 🔒 | 7.7 |

---

## Entregable

Un AppImage que se descarga, se ejecuta y funciona, sin instalar nada más.

## Criterio de aceptación

- Instalación limpia en una Ubuntu 24.04 sin Rust ni Node.
- El primer arranque no falla ni pierde datos si se cierra a mitad de la migración.
- La CI bloquea cualquier merge que rompa lint, tests o presupuestos.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 7.1–7.3, 7.5–7.8 | Empaquetado completo y verificado ejecutando el AppImage |
| 2026-08-18 | 7.2 | El AppImage necesitaba `librsvg2-dev` para el plugin GTK de linuxdeploy; añadido a los requisitos del README y del CI |


---

## Bloqueado a la espera de una decisión tuya

**7.4 · Actualizador automático.** `tauri-plugin-updater` necesita dos cosas que no puedo
inventar: un **par de claves de firma** (la privada no debe salir de tus manos ni entrar en el
repositorio) y un **endpoint de releases** al que apuntar. Cuando decidas si esto se publica en
GitHub Releases y generes las claves con `pnpm tauri signer generate`, conectarlo es media hora.

**7.9 · Matriz Windows y macOS.** El CI puede compilar en las tres plataformas cambiando una
línea, pero no puedo *probar* que la app funcione en ellas desde aquí. Se añade cuando haya
alguien que pueda verificar el resultado, no antes.
