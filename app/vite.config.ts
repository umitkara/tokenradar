/// <reference types="vitest/config" />
import { resolve } from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

const root = import.meta.dirname;
const devHost = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: devHost || false,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // Multi-page build: each window loads only its own bundle.
    rolldownOptions: {
      input: {
        panel: resolve(root, "panel.html"),
        settings: resolve(root, "settings.html"),
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
