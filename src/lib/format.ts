/** Formateo para la interfaz. Todo lo que se ve en pantalla pasa por aquí. */

export function duracion(ms: number | null): string {
  if (ms === null || ms <= 0) return "—";
  const total = ms / 1000;
  if (total < 60) return `${total.toFixed(total < 10 ? 2 : 1)}s`;
  const min = Math.floor(total / 60);
  const seg = Math.floor(total % 60);
  return `${min}:${seg.toString().padStart(2, "0")}`;
}

export function tamano(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function cifra(n: number): string {
  return n.toLocaleString("es-ES");
}

/**
 * Trunca por el CENTRO, no por el final: en `KICK_808_LONG_MASTER_02.wav` lo que identifica
 * el archivo está repartido entre el principio y el final, y perder la extensión es perder
 * información útil.
 */
export function truncarCentro(texto: string, max: number): string {
  if (texto.length <= max) return texto;
  const mitad = Math.floor((max - 1) / 2);
  return `${texto.slice(0, mitad)}…${texto.slice(texto.length - (max - 1 - mitad))}`;
}

export function hz(n: number | null): string {
  if (!n) return "—";
  return n % 1000 === 0 ? `${n / 1000}k` : `${(n / 1000).toFixed(1)}k`;
}

export function canales(n: number | null): string {
  if (!n) return "—";
  if (n === 1) return "mono";
  if (n === 2) return "st";
  return `${n}ch`;
}
