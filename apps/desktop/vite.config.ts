import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Cấu hình Vite cho Tauri: cố định port 1420 (khớp devUrl trong tauri.conf.json).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "esnext",
    outDir: "dist",
  },
});
