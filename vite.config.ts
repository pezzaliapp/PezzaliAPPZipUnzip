import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Configurazione per Tauri: porta fissa, nessuna risorsa esterna.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "es2022", sourcemap: false },
});
