import { useEffect } from "react";
import { SampleList } from "../features/library/components/SampleList";
import { Sidebar } from "../features/library/components/Sidebar";
import { consultaActual, useLibraryStore } from "../features/library/store";
import { Transport } from "../features/player/components/Transport";
import { HelpOverlay } from "../features/settings/components/HelpOverlay";
import { DestinationsPanel } from "../features/triage/components/DestinationsPanel";
import { useTriageStore } from "../features/triage/store";
import * as ipc from "../lib/ipc";
import { registrarKeymap } from "../lib/keymap";
import { log } from "../lib/log";
import styles from "./App.module.css";
import { construirAtajos } from "./atajos";
import { AutoPlay } from "./components/AutoPlay";
import { SetupWizard } from "./components/SetupWizard";
import { StatusBar } from "./components/StatusBar";
import { useUiStore } from "./uiStore";

export function App() {
  const ayudaAbierta = useUiStore((s) => s.ayudaAbierta);
  const asistenteAbierto = useUiStore((s) => s.asistenteAbierto);

  // ── arranque ────────────────────────────────────────────────
  useEffect(() => {
    void (async () => {
      const lib = useLibraryStore.getState();
      await lib.cargarFuentes();
      await lib.refrescar();
      await useTriageStore.getState().cargar();

      // Retomar el triaje donde se dejó: un triaje que te devuelve al principio cada vez
      // que abres la app no se termina nunca.
      const proyecto = useTriageStore.getState().proyecto;
      if (proyecto) {
        try {
          const ultimo = await ipc.ultimaPosicion(proyecto.id);
          if (ultimo !== null) {
            const s = useLibraryStore.getState();
            const pos = await ipc.posicionDe(consultaActual(s, 0, 1), ultimo);
            if (pos !== null) useLibraryStore.getState().irA(pos);
          }
        } catch (e) {
          log.warn("no se pudo retomar la última posición", e);
        }
      }

      // El tema se restaura antes de nada para que no haya un parpadeo de oscuro a claro.
      const tema = await ipc.settingsGet("tema").catch(() => null);
      if (tema === "light" || tema === "dark") useUiStore.getState().aplicarTema(tema);

      const info = await ipc.appInfo().catch(() => null);
      if (info?.audioError) {
        useUiStore
          .getState()
          .avisar("error", `Sin audio: ${info.audioError}. Puedes ordenar, pero no escuchar.`);
      }
      if (useLibraryStore.getState().fuentes.length === 0) {
        useUiStore.getState().setAsistente(true);
      }
    })();
  }, []);

  // ── teclado: un único listener para toda la app ─────────────
  useEffect(() => {
    return registrarKeymap(() => {
      const ui = useUiStore.getState();
      const todos = construirAtajos();
      // Con un panel abierto solo quedan vivos Esc y la ayuda: nada de disparar decisiones
      // de triaje contra una lista que no se está viendo.
      if (ui.asistenteAbierto || ui.ayudaAbierta) {
        return todos.filter((a) => a.id === "escape" || a.id === "ayuda");
      }
      return todos;
    });
  }, []);

  // ── soltar carpetas sobre la ventana ────────────────────────
  useEffect(() => {
    let soltar: (() => void) | null = null;
    void ipc
      .alSoltarCarpetas(async (rutas) => {
        const primera = rutas[0];
        if (primera === undefined) return;
        try {
          await ipc.anadirFuente(primera, (p) => useLibraryStore.getState().setProgreso(p));
          await useLibraryStore.getState().cargarFuentes();
          await useLibraryStore.getState().refrescar();
          useUiStore.getState().avisar("exito", "Carpeta añadida");
        } catch (e) {
          log.warn("no se pudo añadir la carpeta soltada", e);
          useUiStore.getState().avisar("error", "No se pudo añadir esa carpeta");
        }
      })
      .then((f) => {
        soltar = f;
      });
    return () => soltar?.();
  }, []);

  // ── el análisis en segundo plano refresca la lista al terminar ──
  useEffect(() => {
    const t = window.setInterval(() => {
      void ipc.analisisPendiente().then((pendientes) => {
        const previo = useLibraryStore.getState().progreso;
        if (previo && previo.pendingAnalysis > 0 && pendientes === 0) {
          void useLibraryStore.getState().refrescar(true);
        }
        useLibraryStore.getState().setProgreso(
          previo
            ? { ...previo, pendingAnalysis: pendientes, done: pendientes === 0 }
            : {
                found: 0,
                indexed: 0,
                analyzed: 0,
                pendingAnalysis: pendientes,
                done: pendientes === 0,
              },
        );
      });
    }, 1500);
    return () => window.clearInterval(t);
  }, []);

  return (
    <div className={styles.app}>
      <div className={styles.centro}>
        <Sidebar />
        <main className={styles.lista}>
          <SampleList />
        </main>
        <DestinationsPanel />
      </div>
      <Transport />
      <StatusBar />
      {ayudaAbierta && <HelpOverlay />}
      {asistenteAbierto && <SetupWizard />}
      <AutoPlay />
    </div>
  );
}
