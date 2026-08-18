import { useEffect, useRef } from "react";
import { useLabelsStore } from "../../../features/labels/store";
import { filaEn, useLibraryStore } from "../../../features/library/store";
import { useMetaStore } from "../../../features/meta/store";
import { usePlayerStore } from "../../../features/player/store";
import { useTriageStore } from "../../../features/triage/store";
import * as ipc from "../../../lib/ipc";

const VECINOS = 3;

/**
 * Componente sin interfaz: reacciona al cambio de foco.
 *
 * Está aislado a propósito. Si esta lógica viviera en `App`, cada movimiento del foco
 * re-renderizaría el árbol entero; aquí solo se re-renderiza un componente que no pinta nada.
 */
export function AutoPlay() {
  const idEnFoco = useLibraryStore((s) => filaEn(s, s.foco)?.id ?? null);
  const temporizador = useRef<number | null>(null);

  useEffect(() => {
    if (idEnFoco === null) return;

    const player = usePlayerStore.getState();
    if (player.autoplay) void player.reproducir(idEnFoco);

    if (useLabelsStore.getState().modo) {
      void useLabelsStore.getState().cargarDe(idEnFoco);
    }
    if (useMetaStore.getState().modo) {
      void useMetaStore.getState().cargarDe(idEnFoco);
    }

    // El prefetch y el "recuérdame dónde estaba" van con freno: en un barrido rápido de
    // flechas no tiene sentido pedirlos veinte veces por segundo.
    if (temporizador.current !== null) window.clearTimeout(temporizador.current);
    temporizador.current = window.setTimeout(() => {
      const lib = useLibraryStore.getState();
      const vecinos: number[] = [];
      for (let d = -VECINOS; d <= VECINOS; d++) {
        if (d === 0) continue;
        const fila = filaEn(lib, lib.foco + d);
        if (fila) vecinos.push(fila.id);
      }
      usePlayerStore.getState().prefetch(vecinos);

      const proyecto = useTriageStore.getState().proyecto;
      if (proyecto) void ipc.recordarPosicion(proyecto.id, idEnFoco).catch(() => {});
    }, 120);

    return () => {
      if (temporizador.current !== null) window.clearTimeout(temporizador.current);
    };
  }, [idEnFoco]);

  return null;
}
