import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  base: "/gigi/sheets/",
  resolve: {
    alias: {
      "@gigi-db/client": resolve(__dirname, "../sdk/js/src/index.ts"),
    },
  },
  build: {
    // Mirror the served sub-path on disk so `index.html` (which
    // references assets via the absolute `/gigi/sheets/...` URL set
    // by `base`) lines up with the file layout Vercel sees. With a
    // flat `dist/`, a request for `/gigi/sheets/assets/index.js`
    // would 404 because the file would live at `dist/assets/index.js`.
    // Nesting the output makes the build self-consistent whether
    // Vercel mounts this project at the root or at a path-rewritten
    // sub-route off the main davisgeometric.com domain.
    outDir: "dist/gigi/sheets",
    emptyOutDir: true,
  },
  server: {
    port: 5177,
    open: true,
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
    include: ["tests/**/*.test.ts", "tests/**/*.test.tsx"],
    css: false,
  },
});
