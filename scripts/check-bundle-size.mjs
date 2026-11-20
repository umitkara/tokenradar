#!/usr/bin/env node
// Checks the gzip size of each page entry against docs/performance-budget.md. Reads the Vite
// manifest of a production build (`pnpm -C app build`) and follows static imports, so shared
// chunks count towards every page that loads them. Also fails if the dev-only gallery leaked
// into the build. Exit code 1 on any violation.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const dist = resolve(dirname(fileURLToPath(import.meta.url)), "../app/dist");
const manifestPath = join(dist, ".vite/manifest.json");

/** Gzip budgets in bytes, per entry. */
const BUDGETS = {
  "panel.html": { js: 60 * 1024, css: 15 * 1024 },
  // The main window: 180 KB JS including uPlot. Its pages load on demand; the check counts the
  // entry with every page chunk it can import (worst case: all pages visited).
  "main.html": { js: 180 * 1024, dynamic: true },
};

if (!existsSync(manifestPath)) {
  console.error(`No Vite manifest at ${manifestPath}. Run \`pnpm -C app build\` first.`);
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

/** Static imports; with `dynamic`, also the chunks `import()` can load (lazy pages). */
function collect(key, dynamic, seen = new Set()) {
  if (seen.has(key)) return seen;
  seen.add(key);
  const chunk = manifest[key];
  const deps = [...(chunk?.imports ?? []), ...(dynamic ? (chunk?.dynamicImports ?? []) : [])];
  for (const dep of deps) collect(dep, dynamic, seen);
  return seen;
}

const gz = (file) => gzipSync(readFileSync(join(dist, file)), { level: 9 }).length;
const kb = (n) => `${(n / 1024).toFixed(1)} KB`;

let failed = false;

for (const [entry, budget] of Object.entries(BUDGETS)) {
  if (!manifest[entry]) {
    console.error(`${entry}: not in the manifest`);
    failed = true;
    continue;
  }
  const files = { js: new Set(), css: new Set(), other: new Set() };
  for (const key of collect(entry, budget.dynamic === true)) {
    const chunk = manifest[key];
    if (chunk.file.endsWith(".js")) files.js.add(chunk.file);
    for (const css of chunk.css ?? []) files.css.add(css);
    for (const asset of chunk.assets ?? []) files.other.add(asset);
  }
  const size = (set) => [...set].reduce((sum, f) => sum + gz(f), 0);
  const js = size(files.js);
  const css = size(files.css);
  const assets = [...files.other].reduce(
    (sum, f) => sum + readFileSync(join(dist, f)).length,
    0,
  );
  // Entries without a CSS budget report their CSS only (performance-budget.md has none).
  const ok = js <= budget.js && (budget.css === undefined || css <= budget.css);
  failed ||= !ok;
  console.log(
    `${ok ? "ok  " : "FAIL"} ${entry}: JS ${kb(js)} / ${kb(budget.js)}, ` +
      `CSS ${kb(css)}${budget.css === undefined ? "" : ` / ${kb(budget.css)}`} (gzip); ` +
      `other assets ${kb(assets)} raw`,
  );
}

if (existsSync(join(dist, "gallery.html"))) {
  console.error("FAIL gallery.html is in the production build (development only)");
  failed = true;
}

process.exit(failed ? 1 : 0);
