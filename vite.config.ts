import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Port 1420 (8080 is used by other apps on the dev machine). Tauri expects a fixed port.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    // The Rust workspace lives in the same folder; never watch build output or Rust sources.
    watch: { ignored: ["**/target/**", "**/src-tauri/**", "**/crates/**", "**/docs/**", "**/release/**"] },
  },
  build: { target: "es2022", sourcemap: false, chunkSizeWarningLimit: 800 },
  test: { environment: "node", include: ["src/**/*.test.ts"] },
} as never);
