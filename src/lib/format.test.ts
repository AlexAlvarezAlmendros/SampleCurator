import { describe, expect, it } from "vitest";
import { canales, cifra, duracion, hz, tamano, truncarCentro } from "./format";

describe("duracion", () => {
  it("usa dos decimales por debajo de 10 s y uno hasta el minuto", () => {
    // Se evitan valores justo en el borde del redondeo binario (0,345 → 0,34): lo que se
    // fija aquí es el FORMATO, no el modo en que el IEEE 754 parte un empate.
    expect(duracion(1_500)).toBe("1.50s");
    expect(duracion(12_400)).toBe("12.4s");
  });
  it("pasa a minutos:segundos a partir del minuto", () => {
    expect(duracion(65_000)).toBe("1:05");
    expect(duracion(600_000)).toBe("10:00");
  });
  it("no inventa nada cuando aún no hay análisis", () => {
    expect(duracion(null)).toBe("—");
    expect(duracion(0)).toBe("—");
  });
});

describe("truncarCentro", () => {
  it("deja el nombre intacto si cabe", () => {
    expect(truncarCentro("kick.wav", 20)).toBe("kick.wav");
  });
  it("recorta por el centro para conservar principio y extensión", () => {
    const r = truncarCentro("KICK_808_LONG_MASTER_FINAL_02.wav", 20);
    expect(r).toHaveLength(20);
    expect(r.startsWith("KICK_")).toBe(true);
    expect(r.endsWith(".wav")).toBe(true);
    expect(r).toContain("…");
  });
});

describe("tamano", () => {
  it("cambia de unidad donde toca", () => {
    expect(tamano(512)).toBe("512 B");
    expect(tamano(2048)).toBe("2 KB");
    expect(tamano(5 * 1024 * 1024)).toBe("5.0 MB");
  });
});

describe("otros formatos", () => {
  it("abrevia frecuencias y canales", () => {
    expect(hz(44_100)).toBe("44.1k");
    expect(hz(48_000)).toBe("48k");
    expect(hz(null)).toBe("—");
    expect(canales(1)).toBe("mono");
    expect(canales(2)).toBe("st");
    expect(canales(6)).toBe("6ch");
  });
  it("separa los miles en español", () => {
    expect(cifra(50_000)).toBe("50.000");
  });
});
