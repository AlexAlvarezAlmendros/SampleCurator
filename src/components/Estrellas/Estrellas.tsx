import styles from "./Estrellas.module.css";

interface EstrellasProps {
  valor: number;
  /** Sin `onChange` son solo de lectura: es lo que se pinta en cada fila de la lista. */
  onChange?: ((valor: number) => void) | undefined;
  tamano?: "fila" | "grande" | undefined;
}

/**
 * Valoración de 0 a 5.
 *
 * Pulsar la estrella que ya está puesta la quita: sin eso, bajar de una estrella a ninguna
 * obligaría a buscar otro control, y valorar rápido es justo el punto.
 */
export function Estrellas({ valor, onChange, tamano = "fila" }: EstrellasProps) {
  const editable = onChange !== undefined;

  if (!editable && valor === 0) return null;

  return (
    <span className={styles.estrellas} data-tamano={tamano} data-editable={editable || undefined}>
      {[1, 2, 3, 4, 5].map((n) =>
        editable ? (
          <button
            key={n}
            type="button"
            className={styles.estrella}
            data-puesta={n <= valor || undefined}
            onClick={() => onChange(n === valor ? 0 : n)}
            aria-label={`${n} ${n === 1 ? "estrella" : "estrellas"}`}
            aria-pressed={n <= valor}
          >
            ★
          </button>
        ) : (
          <span key={n} className={styles.estrella} data-puesta={n <= valor || undefined}>
            ★
          </span>
        ),
      )}
    </span>
  );
}
