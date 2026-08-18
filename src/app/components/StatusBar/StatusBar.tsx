import { useEffect } from "react";
import { useUiStore } from "../../uiStore";
import styles from "./StatusBar.module.css";

/**
 * Avisos que NO bloquean. Cuando algo falla, el usuario ya va tres samples más abajo: la
 * información aparece aquí y se va sola, sin robarle el foco ni el ritmo.
 */
export function StatusBar() {
  const aviso = useUiStore((s) => s.aviso);

  useEffect(() => {
    if (!aviso) return;
    const t = setTimeout(() => useUiStore.getState().limpiarAviso(), 4500);
    return () => clearTimeout(t);
  }, [aviso]);

  if (!aviso) return null;

  return (
    <output className={styles.barra} data-tipo={aviso.tipo} aria-live="polite">
      {aviso.texto}
    </output>
  );
}
