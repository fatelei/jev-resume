import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// port 必须与 tauri.conf.json 的 build.devUrl 一致
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: { target: "es2022", outDir: "dist" },
});
