import { useCallback, useEffect, useRef } from "react";
import { posicionCabezal, usePlayerStore } from "../../store";
import styles from "./Waveform.module.css";

/**
 * La onda, en canvas.
 *
 * Dos capas: la de abajo pinta los picos apagados y solo se redibuja al cambiar de sample;
 * la de arriba pinta la parte ya reproducida y el cabezal, y se anima con
 * requestAnimationFrame FUERA de React. Ni un re-render ni un evento IPC por frame.
 */
export function Waveform() {
  const picos = usePlayerStore((s) => s.picos);
  const contenedor = useRef<HTMLDivElement>(null);
  const lienzoBase = useRef<HTMLCanvasElement>(null);
  const lienzoCabezal = useRef<HTMLCanvasElement>(null);
  const picosRef = useRef<Int8Array | null>(null);
  picosRef.current = picos;

  const dimensionar = useCallback(() => {
    const caja = contenedor.current;
    const base = lienzoBase.current;
    const cabezal = lienzoCabezal.current;
    if (!caja || !base || !cabezal) return { ancho: 0, alto: 0, dpr: 1 };
    const dpr = window.devicePixelRatio || 1;
    const ancho = Math.max(1, Math.floor(caja.clientWidth));
    const alto = Math.max(1, Math.floor(caja.clientHeight));
    for (const c of [base, cabezal]) {
      c.width = Math.floor(ancho * dpr);
      c.height = Math.floor(alto * dpr);
      c.style.width = `${ancho}px`;
      c.style.height = `${alto}px`;
    }
    return { ancho, alto, dpr };
  }, []);

  const pintarBase = useCallback(() => {
    const { ancho, alto, dpr } = dimensionar();
    const ctx = lienzoBase.current?.getContext("2d");
    if (!ctx || ancho === 0) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, ancho, alto);

    const datos = picosRef.current;
    const estilo = getComputedStyle(document.documentElement);
    ctx.strokeStyle = estilo.getPropertyValue("--color-waveform-idle").trim();
    ctx.lineWidth = 1;

    if (!datos || datos.length < 2) {
      // Sin picos todavía: una línea central en vez de un hueco negro.
      ctx.beginPath();
      ctx.moveTo(0, alto / 2 + 0.5);
      ctx.lineTo(ancho, alto / 2 + 0.5);
      ctx.stroke();
      return;
    }
    dibujarPicos(ctx, datos, ancho, alto, 0, ancho);
  }, [dimensionar]);

  // Redibujado completo solo al cambiar de sample o de tamaño.
  useEffect(() => {
    pintarBase();
    const obs = new ResizeObserver(() => pintarBase());
    if (contenedor.current) obs.observe(contenedor.current);
    return () => obs.disconnect();
  }, [pintarBase]);

  // Bucle del cabezal: vive fuera de React y no provoca ni un render.
  useEffect(() => {
    let vivo = true;
    let anterior = -1;

    const frame = () => {
      if (!vivo) return;
      const caja = contenedor.current;
      const ctx = lienzoCabezal.current?.getContext("2d");
      if (caja && ctx) {
        const dpr = window.devicePixelRatio || 1;
        const ancho = caja.clientWidth;
        const alto = caja.clientHeight;
        const s = usePlayerStore.getState();
        const x = s.duracionMs > 0 ? (posicionCabezal() / s.duracionMs) * ancho : 0;

        if (Math.abs(x - anterior) > 0.4 || anterior < 0) {
          anterior = x;
          ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
          ctx.clearRect(0, 0, ancho, alto);
          const estilo = getComputedStyle(document.documentElement);
          const datos = picosRef.current;
          if (datos && datos.length >= 2 && s.sonando !== null) {
            ctx.strokeStyle = estilo.getPropertyValue("--color-waveform").trim();
            ctx.lineWidth = 1;
            dibujarPicos(ctx, datos, ancho, alto, 0, x);
          }
          if (s.sonando !== null) {
            ctx.strokeStyle = estilo.getPropertyValue("--color-playhead").trim();
            ctx.beginPath();
            ctx.moveTo(Math.floor(x) + 0.5, 0);
            ctx.lineTo(Math.floor(x) + 0.5, alto);
            ctx.stroke();
          }
        }
      }
      requestAnimationFrame(frame);
    };
    const id = requestAnimationFrame(frame);
    return () => {
      vivo = false;
      cancelAnimationFrame(id);
    };
  }, []);

  const alPulsar = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const caja = e.currentTarget.getBoundingClientRect();
    const fraccion = (e.clientX - caja.left) / Math.max(1, caja.width);
    void usePlayerStore.getState().saltarA(fraccion);
  }, []);

  return (
    <div className={styles.contenedor} ref={contenedor} onClick={alPulsar}>
      <canvas className={styles.lienzo} ref={lienzoBase} />
      <canvas className={styles.lienzo} ref={lienzoCabezal} />
    </div>
  );
}

/** Pinta una barra vertical por columna de píxel, de mínimo a máximo. */
function dibujarPicos(
  ctx: CanvasRenderingContext2D,
  datos: Int8Array,
  ancho: number,
  alto: number,
  desde: number,
  hasta: number,
): void {
  const buckets = Math.floor(datos.length / 2);
  if (buckets === 0) return;
  const medio = alto / 2;
  ctx.beginPath();
  for (let x = Math.floor(desde); x < Math.min(ancho, Math.ceil(hasta)); x++) {
    const bucket = Math.min(buckets - 1, Math.floor((x / ancho) * buckets));
    const mn = datos[bucket * 2] ?? 0;
    const mx = datos[bucket * 2 + 1] ?? 0;
    const y1 = medio - (mx / 127) * (medio - 1);
    const y2 = medio - (mn / 127) * (medio - 1);
    ctx.moveTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, Math.max(y2, y1 + 1));
  }
  ctx.stroke();
}
