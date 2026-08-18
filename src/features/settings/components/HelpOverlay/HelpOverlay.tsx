import { useMemo } from "react";
import { construirAtajos } from "../../../../app/atajos";
import { useUiStore } from "../../../../app/uiStore";
import { Kbd } from "../../../../components/Kbd";
import styles from "./HelpOverlay.module.css";

/**
 * La ayuda se genera desde la MISMA tabla que ejecuta los atajos: si se añade una acción,
 * aparece aquí sola, y si se cambia una tecla, aquí ya está cambiada.
 */
export function HelpOverlay() {
  const grupos = useMemo(() => {
    const mapa = new Map<string, Array<{ etiqueta: string; descripcion: string }>>();
    for (const a of construirAtajos()) {
      const lista = mapa.get(a.grupo) ?? [];
      lista.push({ etiqueta: a.etiqueta, descripcion: a.descripcion });
      mapa.set(a.grupo, lista);
    }
    return [...mapa.entries()];
  }, []);

  return (
    <div
      className={styles.fondo}
      onClick={() => useUiStore.getState().alternarAyuda()}
      role="presentation"
    >
      <div
        className={styles.hoja}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Atajos de teclado"
      >
        <header className={styles.cabecera}>
          <h2 className={styles.titulo}>Atajos</h2>
          <span className={styles.cerrar}>
            <Kbd>Esc</Kbd> para cerrar
          </span>
        </header>
        <div className={styles.columnas}>
          {grupos.map(([grupo, atajos]) => (
            <section key={grupo} className={styles.grupo}>
              <h3 className={styles.nombreGrupo}>{grupo}</h3>
              {atajos.map((a) => (
                <div key={a.etiqueta + a.descripcion} className={styles.linea}>
                  <Kbd>{a.etiqueta}</Kbd>
                  <span className={styles.descripcion}>{a.descripcion}</span>
                </div>
              ))}
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
