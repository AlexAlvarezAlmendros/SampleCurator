import { useEffect, useRef, useState } from "react";
import { useUiStore } from "../../../../app/uiStore";
import { Kbd } from "../../../../components/Kbd";
import { cifra, duracion } from "../../../../lib/format";
import * as ipc from "../../../../lib/ipc";
import { filaEn, useLibraryStore } from "../../../library/store";
import { useTriageStore } from "../../../triage/store";
import { usePlayerStore } from "../../store";
import { Waveform } from "../Waveform";
import styles from "./Transport.module.css";

export function Transport() {
  const fila = useLibraryStore((s) => filaEn(s, s.foco));
  const foco = useLibraryStore((s) => s.foco);
  const total = useLibraryStore((s) => s.total);
  const bucle = usePlayerStore((s) => s.bucle);
  const silenciado = usePlayerStore((s) => s.silenciado);
  const volumen = usePlayerStore((s) => s.volumen);
  const autoplay = usePlayerStore((s) => s.autoplay);
  const progreso = useTriageStore((s) => s.progreso);
  const renombrando = useUiStore((s) => s.renombrando);
  const [nombreNuevo, setNombreNuevo] = useState("");
  const campo = useRef<HTMLInputElement>(null);

  // El renombrado ocurre EN SU SITIO, en la barra de transporte: abrir un diálogo para
  // cambiar un nombre rompería el ritmo del triaje.
  useEffect(() => {
    if (renombrando && fila) {
      setNombreNuevo(fila.filename);
      requestAnimationFrame(() => {
        campo.current?.focus();
        const punto = fila.filename.lastIndexOf(".");
        campo.current?.setSelectionRange(0, punto > 0 ? punto : fila.filename.length);
      });
    }
  }, [renombrando, fila]);

  const confirmarNombre = async () => {
    const ui = useUiStore.getState();
    if (!fila || nombreNuevo.trim() === "" || nombreNuevo === fila.filename) {
      ui.setRenombrando(false);
      return;
    }
    try {
      const nuevo = await ipc.renombrar(
        fila.id,
        nombreNuevo,
        useTriageStore.getState().proyecto?.id ?? null,
      );
      useLibraryStore.getState().parchear(fila.id, { filename: nuevo });
      ui.avisar("exito", `Renombrado a ${nuevo}`);
    } catch (e) {
      ui.avisar("error", ipc.esAppError(e) ? e.message : "No se pudo renombrar");
    }
    ui.setRenombrando(false);
  };

  const hechos = progreso?.done ?? 0;
  const totalSesion = progreso?.total ?? 0;
  const porcentaje = totalSesion > 0 ? (hechos / totalSesion) * 100 : 0;

  return (
    <footer className={styles.transporte}>
      <div className={styles.onda}>
        <Waveform />
      </div>

      <div className={styles.info}>
        {renombrando && fila ? (
          <input
            ref={campo}
            className={styles.campoNombre}
            value={nombreNuevo}
            onChange={(e) => setNombreNuevo(e.target.value)}
            onBlur={() => void confirmarNombre()}
            onKeyDown={(e) => {
              if (e.key === "Enter") void confirmarNombre();
              if (e.key === "Escape") useUiStore.getState().setRenombrando(false);
            }}
            aria-label="Nuevo nombre del archivo"
          />
        ) : (
          <div className={styles.nombre} title={fila?.relPath ?? ""}>
            {fila?.filename ?? "—"}
          </div>
        )}
        <div className={styles.meta}>
          <span>{duracion(fila?.durationMs ?? null)}</span>
          <span className={styles.separador}>·</span>
          <span>{total > 0 ? `${cifra(foco + 1)} / ${cifra(total)}` : "sin samples"}</span>
        </div>

        <div className={styles.controles}>
          <button
            type="button"
            className={styles.interruptor}
            data-activo={bucle || undefined}
            onClick={() => void usePlayerStore.getState().alternarBucle()}
          >
            bucle <Kbd>⇧ Espacio</Kbd>
          </button>
          <button
            type="button"
            className={styles.interruptor}
            data-activo={autoplay || undefined}
            onClick={() => usePlayerStore.getState().alternarAutoplay()}
          >
            autoplay <Kbd>⇧ A</Kbd>
          </button>
          <button
            type="button"
            className={styles.interruptor}
            data-activo={silenciado || undefined}
            onClick={() => void usePlayerStore.getState().alternarSilencio()}
          >
            {silenciado ? "silenciado" : `vol ${Math.round(volumen * 100)}%`} <Kbd>S</Kbd>
          </button>
        </div>
      </div>

      <div className={styles.sesion}>
        <div className={styles.cifra}>
          {cifra(hechos)} <span className={styles.de}>/</span> {cifra(totalSesion)}
        </div>
        <div className={styles.barra}>
          <div className={styles.relleno} style={{ width: `${porcentaje}%` }} />
        </div>
        <div className={styles.etiquetaSesion}>revisados</div>
      </div>
    </footer>
  );
}
