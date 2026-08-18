import { memo } from "react";
import { Chip } from "../../../../components/Chip";
import { canales, duracion, hz, truncarCentro } from "../../../../lib/format";
import { usePlayerStore } from "../../../player/store";
import { filaEn, useLibraryStore } from "../../store";
import styles from "./Row.module.css";

interface RowProps {
  indice: number;
  /** Posición absoluta que le asigna el virtualizador. */
  desplazamiento: number;
}

/**
 * Una fila de la lista.
 *
 * Recibe SOLO props primitivas y se suscribe ella misma a su dato, a si tiene el foco y a si
 * está sonando. Así, mover la selección repinta dos filas en vez de las treinta visibles.
 */
function RowImpl({ indice, desplazamiento }: RowProps) {
  const fila = useLibraryStore((s) => filaEn(s, indice));
  const enfocada = useLibraryStore((s) => s.foco === indice);
  const seleccionada = useLibraryStore((s) => s.seleccion.has(indice));
  const sonando = usePlayerStore((s) => s.sonando !== null && s.sonando === fila?.id);

  if (!fila) {
    return (
      <div
        className={styles.fila}
        style={{ transform: `translateY(${desplazamiento}px)` }}
        data-cargando="true"
      >
        <span />
        <span className={styles.hueco} />
      </div>
    );
  }

  const decidida = fila.status !== "pending";

  return (
    <div
      className={styles.fila}
      style={{ transform: `translateY(${desplazamiento}px)` }}
      id={`fila-${indice}`}
      tabIndex={-1}
      data-indice={indice}
      data-enfocada={enfocada || undefined}
      data-seleccionada={seleccionada || undefined}
      data-sonando={sonando || undefined}
      data-decidida={decidida || undefined}
      role="option"
      aria-selected={enfocada}
    >
      <span className={styles.indicador} />
      <span className={styles.nombre} title={fila.relPath}>
        {truncarCentro(fila.filename, 56)}
      </span>
      <span className={styles.duracion}>{duracion(fila.durationMs)}</span>
      <span className={styles.formato}>
        {fila.analyzed ? `${hz(fila.sampleRate)} ${canales(fila.channels)}` : "…"}
      </span>
      <span className={styles.estado}>
        {fila.rating >= 5 && <Chip tono="acento">★</Chip>}
        {fila.duplicate && <Chip tono="warn">dup</Chip>}
        {fila.status === "rejected" && <Chip tono="reject">papelera</Chip>}
        {fila.destination !== null && <Chip tono="keep">{fila.destination}</Chip>}
        {fila.status === "kept" && fila.destination === null && <Chip tono="keep">✓</Chip>}
      </span>
    </div>
  );
}

export const Row = memo(RowImpl);
