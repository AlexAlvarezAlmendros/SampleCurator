/**
 * Logger de la app. Existe para que `console.log` no aparezca nunca en `src/`: los mensajes
 * llevan prefijo y se pueden silenciar de un sitio.
 */
const PREFIJO = "[sc]";

export const log = {
  warn(mensaje: string, ...datos: unknown[]): void {
    console.warn(PREFIJO, mensaje, ...datos);
  },
  error(mensaje: string, ...datos: unknown[]): void {
    console.error(PREFIJO, mensaje, ...datos);
  },
};
