import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { Kbd } from "../../../../components/Kbd";
import { cifra } from "../../../../lib/format";
import { useLibraryStore } from "../../store";
import { Row } from "../Row";
import styles from "./SampleList.module.css";

/** La altura de fila sale del token CSS: el layout y el diseño no pueden desincronizarse. */
function alturaFila(): number {
  const valor = getComputedStyle(document.documentElement).getPropertyValue("--row-height");
  const n = Number.parseInt(valor, 10);
  return Number.isFinite(n) && n > 0 ? n : 28;
}

export function SampleList() {
  const contenedor = useRef<HTMLDivElement>(null);
  const total = useLibraryStore((s) => s.total);
  const foco = useLibraryStore((s) => s.foco);
  const hayFuentes = useLibraryStore((s) => s.fuentes.length > 0);
  const asegurarRango = useLibraryStore((s) => s.asegurarRango);
  const irA = useLibraryStore((s) => s.irA);

  const altura = useMemo(alturaFila, []);

  const virtualizador = useVirtualizer({
    count: total,
    getScrollElement: () => contenedor.current,
    estimateSize: () => altura,
    overscan: 12,
  });

  const visibles = virtualizador.getVirtualItems();

  // Carga por ventanas: solo se piden las páginas que se están mirando.
  useEffect(() => {
    const primera = visibles[0]?.index ?? 0;
    const ultima = visibles[visibles.length - 1]?.index ?? 0;
    asegurarRango(primera, ultima);
  }, [visibles, asegurarRango]);

  // El foco lo mueve el teclado; la lista solo lo sigue.
  useEffect(() => {
    if (total > 0) virtualizador.scrollToIndex(foco, { align: "auto" });
  }, [foco, total, virtualizador]);

  /** Un solo manejador para toda la lista: nada de un callback por fila. */
  const alPulsar = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const objetivo = (e.target as HTMLElement).closest("[data-indice]");
      if (!(objetivo instanceof HTMLElement)) return;
      const indice = Number.parseInt(objetivo.dataset.indice ?? "", 10);
      if (Number.isFinite(indice)) irA(indice, e.shiftKey);
    },
    [irA],
  );

  if (total === 0) {
    return (
      <div className={styles.vacio}>
        {hayFuentes ? (
          <p>
            Nada coincide con el filtro. <Kbd>Esc</Kbd> para limpiarlo.
          </p>
        ) : (
          <p>
            Arrastra aquí una carpeta de samples, o pulsa <Kbd>O</Kbd> para abrirla.
          </p>
        )}
      </div>
    );
  }

  return (
    <div
      className={styles.contenedor}
      ref={contenedor}
      onClick={alPulsar}
      role="listbox"
      aria-label={`${cifra(total)} samples`}
      aria-activedescendant={`fila-${foco}`}
      tabIndex={0}
    >
      <div className={styles.lienzo} style={{ height: `${virtualizador.getTotalSize()}px` }}>
        {visibles.map((v) => (
          <Row key={v.key} indice={v.index} desplazamiento={v.start} />
        ))}
      </div>
    </div>
  );
}
