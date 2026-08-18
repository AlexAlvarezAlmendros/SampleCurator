/**
 * Mecanismo de atajos: UN solo listener global y una tabla declarativa.
 *
 * Nada de `addEventListener` repartidos por componentes ni librerías de hotkeys: con un
 * único punto de entrada el mapa de teclas se puede listar, mostrar en pantalla y (Fase 5)
 * reconfigurar sin tocar ningún componente.
 */

export interface Atajo {
  id: string;
  /** Cómo se escribe la tecla en la interfaz, p. ej. "⇧ Espacio". */
  etiqueta: string;
  descripcion: string;
  grupo: string;
  test: (e: KeyboardEvent) => boolean;
  /** Recibe el evento: los atajos de rango (1…9) necesitan saber qué tecla fue. */
  ejecutar: (e: KeyboardEvent) => void | Promise<void>;
  /** Si es true, funciona incluso con el foco dentro de un campo de texto. */
  enTexto?: boolean;
}

interface Mods {
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
}

/** Comprueba tecla + modificadores exactos: `ctrl+z` no dispara con `ctrl+shift+z`. */
export function tecla(nombre: string, mods: Mods = {}) {
  const objetivo = nombre.toLowerCase();
  return (e: KeyboardEvent): boolean =>
    e.key.toLowerCase() === objetivo &&
    (e.ctrlKey || e.metaKey) === Boolean(mods.ctrl) &&
    e.shiftKey === Boolean(mods.shift) &&
    e.altKey === Boolean(mods.alt);
}

/** Cualquiera de varias teclas con los mismos modificadores. */
export function algunaTecla(nombres: string[], mods: Mods = {}) {
  const tests = nombres.map((n) => tecla(n, mods));
  return (e: KeyboardEvent): boolean => tests.some((t) => t(e));
}

/**
 * Como `algunaTecla`, pero sin mirar el Shift.
 *
 * Hace falta para el volumen: el `+` se escribe con Shift en muchas distribuciones, así que
 * exigir `shiftKey === false` hacía que el atajo no disparara nunca. Lo que importa es el
 * carácter que llega, no cómo lo produzca tu teclado.
 */
export function teclaIgnorandoShift(nombres: string[]) {
  const objetivos = nombres.map((n) => n.toLowerCase());
  return (e: KeyboardEvent): boolean =>
    objetivos.includes(e.key.toLowerCase()) && !e.ctrlKey && !e.metaKey && !e.altKey;
}

/** Dígito con Alt: se usa para la valoración (Alt+3 = tres estrellas). */
export function digitoConAlt(e: KeyboardEvent): boolean {
  return e.key >= "0" && e.key <= "5" && e.altKey && !e.ctrlKey && !e.metaKey;
}

export function esDigito1a9(e: KeyboardEvent): boolean {
  return e.key >= "1" && e.key <= "9" && !e.ctrlKey && !e.metaKey && !e.shiftKey && !e.altKey;
}

function enCampoDeTexto(destino: EventTarget | null): boolean {
  if (!(destino instanceof HTMLElement)) return false;
  const etiqueta = destino.tagName;
  return etiqueta === "INPUT" || etiqueta === "TEXTAREA" || destino.isContentEditable;
}

/**
 * Engancha el listener global. Devuelve la función para soltarlo.
 * `obtenerAtajos` se llama en cada pulsación para que la tabla siempre sea la actual sin
 * tener que re-registrar el listener.
 */
export function registrarKeymap(obtenerAtajos: () => Atajo[]): () => void {
  const alPulsar = (e: KeyboardEvent) => {
    if (e.repeat && e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const enTexto = enCampoDeTexto(e.target);
    for (const atajo of obtenerAtajos()) {
      if (enTexto && !atajo.enTexto) continue;
      if (!atajo.test(e)) continue;
      e.preventDefault();
      void atajo.ejecutar(e);
      return;
    }
  };
  window.addEventListener("keydown", alPulsar);
  return () => window.removeEventListener("keydown", alPulsar);
}
