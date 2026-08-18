import styles from "./Kbd.module.css";

interface KbdProps {
  children: string;
  /** Atenuada cuando la acción no está disponible ahora mismo. */
  apagada?: boolean | undefined;
}

/**
 * La tecla, escrita en pantalla. Es el mecanismo por el que el usuario aprende a no usar el
 * ratón: si una acción de la interfaz no lleva `Kbd`, es que le falta la tecla.
 */
export function Kbd({ children, apagada }: KbdProps) {
  return (
    <kbd className={styles.kbd} data-apagada={apagada || undefined}>
      {children}
    </kbd>
  );
}
