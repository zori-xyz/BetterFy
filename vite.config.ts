import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import packageJson from "./package.json";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // The root package.json version is the one tauri.conf.json and the
  // installer ship, so the About screen reads it instead of a literal.
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
