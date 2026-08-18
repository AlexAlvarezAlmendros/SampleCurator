/** Piezas del navegador que jsdom no trae y que la app sí usa. */
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Desmontar entre tests, explícitamente. Sin esto los árboles se acumulan en el DOM y las
// consultas encuentran el mismo texto varias veces — con el orden por defecto casi nunca se
// nota, y con `--sequence.shuffle` fallan a pares.
afterEach(() => {
  cleanup();
});

class ResizeObserverFalso {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

globalThis.ResizeObserver = ResizeObserverFalso as unknown as typeof ResizeObserver;

if (!window.matchMedia) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })) as unknown as typeof window.matchMedia;
}

if (!window.confirm) {
  window.confirm = vi.fn(() => false);
}
