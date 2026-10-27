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
};

if (!existsSync(manifestPath)) {
  console.error(`No Vite manifest at ${manifestPath}. Run \`pnpm -C app build\` first.`);
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

function collect(key, seen = new Set()) {
  if (seen.has(key)) return seen;
  seen.add(key);
  for (const dep of manifest[key]?.imports ?? []) collect(dep, seen);
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
  for (const key of collect(entry)) {
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
  const ok = js <= budget.js && css <= budget.css;
  failed ||= !ok;
  console.log(
    `${ok ? "ok  " : "FAIL"} ${entry}: JS ${kb(js)} / ${kb(budget.js)}, ` +
      `CSS ${kb(css)} / ${kb(budget.css)} (gzip); other assets ${kb(assets)} raw`,
  );
}

if (existsSync(join(dist, "gallery.html"))) {
  console.error("FAIL gallery.html is in the production build (development only)");
  failed = true;
}

process.exit(failed ? 1 : 0);
