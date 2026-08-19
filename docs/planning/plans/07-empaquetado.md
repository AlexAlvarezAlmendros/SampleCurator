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
| 7.4 | Actualizador (`tauri-plugin-updater`) con firma, apuntando a las releases de GitHub | ✅ Hecho | Aviso en la barra lateral, tecla `U`, instalación con progreso y reinicio. Workflow `release.yml` que firma y publica `latest.json` al empujar una etiqueta |
| 7.5 | Copia de seguridad automática de `library.json` junto a la carpeta destino | ✅ Hecho | library.json al cerrar y con Ctrl+E, con escritura atómica |
| 7.6 | Primer arranque: crear rutas de datos, migrar y no fallar nunca en frío | ✅ Hecho | Verificado con la base borrada: crea, migra y arranca sin datos |
| 7.7 | GitHub Actions: `clippy`, `cargo test`, `biome`, `typecheck`, `vitest` y build | ✅ Hecho | 3 jobs: frontend, núcleo Rust y paquetes. Incluye comprobar que bindings.ts está al día |
| 7.8 | README con capturas reales y guía de instalación | ✅ Hecho | README con números medidos, garantías sobre los archivos y guía de uso. Capturas pendientes |
| 7.9 | Build de Windows en la matriz de CI | 🔄 En curso | Compila, pasa clippy y los 110 tests en `windows-latest`, y construye `.msi` y `.exe`. macOS sigue fuera: no hay quien lo verifique |

---

## Entregable

Un AppImage que se descarga, se ejecuta y funciona, sin instalar nada más.

## Criterio de aceptación

- Instalación limpia en una Ubuntu 24.04 sin Rust ni Node.
- El primer arranque no falla ni pierde datos si se cierra a mitad de la migración.
- La CI bloquea cualquier merge que rompa lint, tests o presupuestos.

---

## Verificación del actualizador (2026-08-19)

El ciclo entero se probó contra la release real, no de memoria:

| Qué | Cómo | Resultado |
|---|---|---|
| Se generan las firmas | `pnpm tauri build` con la clave | `.sig` para deb, rpm y AppImage |
| El canal está bien montado | `cargo test --test actualizacion -- --ignored` | endpoint OK, entrada para la plataforma, y **la firma del paquete publicado casa con la clave pública que lleva la app** |
| La app lo detecta | AppImage 0.2.2 empaquetado, 18 s de vida | «hay 0.2.3 disponible (tienes la 0.2.2)» |
| Se instala y se reinicia | build 0.2.2 desechable que instala sin esperar la tecla | el AppImage se reemplazó a sí mismo y el resultado es **byte a byte** el 0.2.3 publicado (sha256 `eead8968…`) |

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-19 | 7.4 | Actualizador con firma, aviso en la barra lateral y tecla `U`. Ciclo completo verificado contra la release 0.2.3 |
| 2026-08-18 | 7.1–7.3, 7.5–7.8 | Empaquetado completo y verificado ejecutando el AppImage |
| 2026-08-18 | 7.2 | El AppImage necesitaba `librsvg2-dev` para el plugin GTK de linuxdeploy; añadido a los requisitos del README y del CI |


---

## Cómo se publica una versión

```bash
git tag -a v0.3.0 -m "SampleCurator 0.3.0" && git push origin v0.3.0
```

`release.yml` compila Linux y Windows, **firma** los paquetes con la clave del actualizador,
publica la release y sube el `latest.json` que la app consulta. La clave privada vive en los
secretos del repositorio (`TAURI_SIGNING_PRIVATE_KEY`) y una copia local en
`~/.samplecurator/updater.key`. **Si se pierde, nadie puede publicar actualizaciones que las
apps ya instaladas acepten**: la pública va compilada dentro de cada binario.

El `.deb` y el `.rpm` se firman también, pero la app no se actualiza sola desde ellos: reemplazar
sus archivos por detrás dejaría al gestor de paquetes mintiendo. Ahí el aviso lleva a la descarga.

**7.9 · Windows: compilado y empaquetado, pero sin ejecutar.** El CI compila el núcleo, pasa
clippy, ejecuta los 110 tests y construye el `.msi` y el `.exe` en `windows-latest`. Eso cubre
la portabilidad del código —rutas, renombrados, prioridades de hilo— pero **nadie ha abierto
todavía la app en Windows**. Lo que hay que comprobar cuando alguien pueda:

1. Que suena, y con qué latencia (WASAPI no deja fijar el buffer; se espera ~10 ms frente a los
   2,6 ms de Linux).
2. Que el triaje mueve archivos correctamente entre unidades distintas (`C:` → `D:`), que es
   donde entra la ruta de copiar-verificar-borrar.
3. Que los acentos y los espacios en las rutas sobreviven al viaje completo.

**macOS** sigue fuera por el mismo motivo, agravado: además de no poder verificarlo, la
distribución exige firma y notarización con una cuenta de desarrollador de Apple.
