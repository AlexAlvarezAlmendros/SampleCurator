import type { ReactNode } from "react";
import styles from "./Panel.module.css";

interface PanelProps {
  titulo?: string;
  accion?: ReactNode;
  children: ReactNode;
  /** El panel ocupa el alto disponible y su contenido hace scroll. */
  desplazable?: boolean;
}

export function Panel({ titulo, accion, children, desplazable }: PanelProps) {
  return (
    <section className={styles.panel}>
      {titulo !== undefined && (
        <header className={styles.cabecera}>
          <h2 className={styles.titulo}>{titulo}</h2>
          {accion}
        </header>
      )}
      <div className={desplazable ? styles.cuerpoDesplazable : styles.cuerpo}>{children}</div>
    </section>
  );
}
