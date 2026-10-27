// Writes target/third-party/THIRD-PARTY-NOTICES.txt: the license of every third-party component
// shipped in the installer and the portable zip (Step 12). The installer puts it next to the app
// and About opens it.
//
// - Rust: the normal (not dev, not build) dependency graph of tokenradar-app and tokenradar-cli
//   for the Windows target, from `cargo metadata`, with the license files each crate ships.
// - JavaScript: the packages Vite actually put into the bundle (target/third-party/
//   js-packages.json, written by app/vite.config.ts), so svelte's runtime is included even
//   though it is a dev dependency.
// - The embedded font (OFL).
//
// Identical license texts are printed once with the list of components that use them.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "target", "third-party");
const SHIPPED = ["tokenradar-app", "tokenradar-cli"];
const TARGET = "x86_64-pc-windows-msvc";
const LICENSE_FILE = /^(licen[cs]e|copying|notice|copyright)([-._].*)?$/i;

function licenseFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && LICENSE_FILE.test(e.name))
    .map((e) => e.name)
    .sort()
    .map((name) => readFileSync(join(dir, name), "utf8").replaceAll("\r\n", "\n").trim());
}

/** One entry per component: `name version`, its SPDX expression and its license texts. */
const components = [];

// Rust ------------------------------------------------------------------------------------------
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET],
    { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  ),
);
const packages = new Map(metadata.packages.map((p) => [p.id, p]));
const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
const workspace = new Set(metadata.workspace_members);
const seen = new Set();
const queue = metadata.packages.filter((p) => SHIPPED.includes(p.name)).map((p) => p.id);
if (queue.length !== SHIPPED.length) throw new Error("shipped packages not found in metadata");
while (queue.length > 0) {
  const id = queue.pop();
  if (seen.has(id)) continue;
  seen.add(id);
  for (const dep of nodes.get(id)?.deps ?? []) {
    // `kind: null` is a normal dependency; dev and build dependencies are not shipped.
    if (dep.dep_kinds.some((k) => k.kind === null)) queue.push(dep.pkg);
  }
}
for (const id of seen) {
  if (workspace.has(id)) continue;
  const p = packages.get(id);
  components.push({
    name: `${p.name} ${p.version}`,
    license: p.license ?? (p.license_file ? "see text" : "unknown"),
    url: p.repository ?? `https://crates.io/crates/${p.name}`,
    texts: licenseFiles(dirname(p.manifest_path)),
  });
}

// JavaScript ------------------------------------------------------------------------------------
const jsList = join(outDir, "js-packages.json");
if (!existsSync(jsList)) throw new Error(`${jsList} is missing; run the production build first`);
for (const dir of JSON.parse(readFileSync(jsList, "utf8"))) {
  const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const repo = typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url;
  components.push({
    name: `${pkg.name} ${pkg.version} (npm)`,
    license: typeof pkg.license === "string" ? pkg.license : "unknown",
    url: repo ?? `https://www.npmjs.com/package/${pkg.name}`,
    texts: licenseFiles(dir),
  });
}

// Font ------------------------------------------------------------------------------------------
components.push({
  name: "Geist Mono (subset)",
  license: "OFL-1.1",
  url: "https://github.com/vercel/geist-font",
  texts: [readFileSync(join(root, "app/src/lib/fonts/OFL.txt"), "utf8").replaceAll("\r\n", "\n").trim()],
});

// Output ----------------------------------------------------------------------------------------
components.sort((a, b) => a.name.localeCompare(b.name));
const byText = new Map();
for (const c of components) {
  for (const text of c.texts) {
    const key = createHash("sha256").update(text).digest("hex");
    if (!byText.has(key)) byText.set(key, { text, users: [] });
    byText.get(key).users.push(c.name);
  }
}

const rule = "=".repeat(78);
const lines = [
  "TokenRadar third-party notices",
  "",
  "TokenRadar is MIT licensed (https://github.com/umitkara/tokenradar/blob/main/LICENSE).",
  `It includes the following ${components.length} third-party components.`,
  "",
  rule,
  "COMPONENTS",
  rule,
  "",
  ...components.map(
    (c) => `${c.name}\n    License: ${c.license}\n    Source: ${c.url}${c.texts.length ? "" : "\n    (no license file in the package; the license is the SPDX expression above)"}`,
  ),
  "",
  rule,
  "LICENSE TEXTS",
  rule,
];
for (const { text, users } of byText.values()) {
  lines.push("", `Used by: ${users.join(", ")}`, "-".repeat(78), "", text, "");
}

mkdirSync(outDir, { recursive: true });
const out = join(outDir, "THIRD-PARTY-NOTICES.txt");
writeFileSync(out, `${lines.join("\n")}\n`);
const missing = components.filter((c) => c.texts.length === 0).length;
console.log(
  `third-party notices: ${components.length} components, ${byText.size} distinct texts, ` +
    `${missing} without a license file -> ${out}`,
);
