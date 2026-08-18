import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef } from "react";
import { ALTURA_FILA, useUiStore } from "../../../../app/uiStore";
import { Kbd } from "../../../../components/Kbd";
import { cifra } from "../../../../lib/format";
import { useLibraryStore } from "../../store";
import { Row } from "../Row";
import styles from "./SampleList.module.css";

export function SampleList() {
  const contenedor = useRef<HTMLDivElement>(null);
  const total = useLibraryStore((s) => s.total);
  const foco = useLibraryStore((s) => s.foco);
  const hayFuentes = useLibraryStore((s) => s.fuentes.length > 0);
  const asegurarRango = useLibraryStore((s) => s.asegurarRango);
  const irA = useLibraryStore((s) => s.irA);

  // El alto sale de la preferencia del usuario, no de leer el CSS: el virtualizador necesita
  // el número exacto, y si lo leyera del CSS y el CSS cambiara, mediría mal.
  const altura = ALTURA_FILA[useUiStore((s) => s.densidad)];

  const virtualizador = useVirtualizer({
    count: total,
    getScrollElement: () => contenedor.current,
    estimateSize: () => altura,
    overscan: 12,
  });

  const visibles = virtualizador.getVirtualItems();

  // Cambiar la densidad invalida lo ya medido. Se compara contra el valor anterior en vez de
  // remedir en cada render: `measure()` tira la caché de medidas entera.
  const alturaAnterior = useRef(altura);
  useEffect(() => {
    if (alturaAnterior.current === altura) return;
    alturaAnterior.current = altura;
    virtualizador.measure();
  }, [altura, virtualizador]);

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
