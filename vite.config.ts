import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri sirve el front desde un puerto fijo y necesita que el servidor no muera al fallar.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/spike/**"] },
  },
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: false,
  },
});
