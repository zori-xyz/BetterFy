import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "website",
  base: "/BetterFy/",
  plugins: [react()],
  publicDir: "public",
  build: {
    outDir: "../dist-site",
    emptyOutDir: true,
    sourcemap: true,
  },
  server: {
    port: 4174,
    strictPort: true,
  },
});
