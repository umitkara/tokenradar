/// <reference types="vitest/config" />
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig, type Plugin } from "vite";

const root = import.meta.dirname;
const devHost = process.env.TAURI_DEV_HOST;

/** The npm packages whose code ends up in the bundle, for scripts/third-party-notices.mjs. */
function bundledPackages(): Plugin {
  return {
    name: "tokenradar-bundled-packages",
    apply: "build",
    generateBundle(_options, bundle) {
      const dirs = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const id of output.moduleIds) {
          // `…/node_modules/.pnpm/x@1/node_modules/@scope/name/…` → the package directory.
          const dir = /^(.*[\\/]node_modules[\\/](?:@[^\\/]+[\\/])?[^\\/]+)[\\/]/.exec(id)?.[1];
          if (dir) dirs.add(dir);
        }
      }
      const out = resolve(root, "../target/third-party");
      mkdirSync(out, { recursive: true });
      writeFileSync(resolve(out, "js-packages.json"), JSON.stringify([...dirs].sort(), null, 2));
    },
  };
}

export default defineConfig(({ mode }) => ({
  plugins: [svelte(), bundledPackages()],
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
