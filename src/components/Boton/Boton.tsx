import type { ReactNode } from "react";
import { Kbd } from "../Kbd";
import styles from "./Boton.module.css";

interface BotonProps {
  children: ReactNode;
  onClick: () => void;
  /** Atajo mostrado a la derecha: si una acción no lo tiene, es que le falta la tecla. */
  atajo?: string;
  variante?: "normal" | "principal" | "peligro";
  deshabilitado?: boolean;
  titulo?: string;
}

export function Boton({
  children,
  onClick,
  atajo,
  variante = "normal",
  deshabilitado,
  titulo,
}: BotonProps) {
  return (
    <button
      type="button"
      className={styles.boton}
      data-variante={variante}
      onClick={onClick}
      disabled={deshabilitado}
      title={titulo}
    >
      <span className={styles.texto}>{children}</span>
      {atajo !== undefined && <Kbd apagada={deshabilitado}>{atajo}</Kbd>}
    </button>
  );
}
