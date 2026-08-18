/** Piezas del navegador que jsdom no trae y que la app sí usa. */
import { vi } from "vitest";

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
