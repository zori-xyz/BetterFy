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
  // The catalog data and third-party libraries change on their own schedules,
  // so they ship as separate chunks instead of one oversized studio bundle.
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (
            /[\\/]src[\\/](webCatalog\.json|catalogPacks\.json|modCatalog\.ts|minifyCatalog\.ts)$/.test(
              id,
            )
          )
            return "catalog";
          if (id.includes("node_modules")) return "vendor";
          return undefined;
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
