/// <reference types="vitest/config" />
import { resolve } from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

const root = import.meta.dirname;
const devHost = process.env.TAURI_DEV_HOST;

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: devHost || false,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // Read by scripts/check-bundle-size.mjs.
    manifest: true,
    // Multi-page build: each window loads only its own bundle.
    rolldownOptions: {
      input: {
        panel: resolve(root, "panel.html"),
        settings: resolve(root, "settings.html"),
        onboarding: resolve(root, "onboarding.html"),
        // The component gallery ships only in development builds (`vite build --mode development`).
        ...(mode === "production" ? {} : { gallery: resolve(root, "gallery.html") }),
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
}));
