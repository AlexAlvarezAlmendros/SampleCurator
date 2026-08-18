import styles from "./Chip.module.css";

export type TonoChip = "neutro" | "keep" | "reject" | "warn" | "acento";

interface ChipProps {
  children: string;
  tono?: TonoChip;
  /** Color de destino (`dest-1`…`dest-9`) para los chips de cubo. */
  destino?: string;
}

export function Chip({ children, tono = "neutro", destino }: ChipProps) {
  return (
    <span
      className={styles.chip}
      data-tono={tono}
      style={destino ? { color: `var(--${destino})`, borderColor: `var(--${destino})` } : undefined}
    >
      {children}
    </span>
  );
}
