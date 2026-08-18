import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    css: false,
    /*
     * Orden aleatorio a propósito. Los stores de zustand y los mocks viven en el módulo y
     * sobreviven de un test a otro; con el orden fijo, una fuga se esconde durante semanas y
     * aparece el día que alguien añade un test en medio. Vitest imprime la semilla, así que
     * un fallo se puede reproducir tal cual.
     */
    sequence: { shuffle: true },
  },
});
