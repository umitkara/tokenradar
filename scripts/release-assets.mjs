// Prepares the release assets after `pnpm tauri build` (with updater artifacts) and
// scripts/package-portable.ps1 (Step 12):
//
// - verifies the installer's updater signature against the public key in tauri.conf.json, so a
//   signing secret that does not match the shipped key fails the release instead of producing
//   updates no installed copy accepts;
// - writes latest.json (the updater endpoint), SHA256SUMS.txt and notes.md (this version's
//   CHANGELOG section) to target/release/bundle/release/.
//
// Usage: node scripts/release-assets.mjs [--base-url <url>]
//   --base-url  where the assets are downloaded from; default the GitHub release of the tag.
//               A local URL tests the update flow against a fake endpoint.

import { execFileSync } from "node:child_process";
import { createHash, createPublicKey, verify } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const REPO = "https://github.com/umitkara/tokenradar";

const { values } = parseArgs({ options: { "base-url": { type: "string" } } });

const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps", "--locked"], {
    cwd: root,
    encoding: "utf8",
  }),
);
const version = metadata.packages.find((p) => p.name === "tokenradar-app").version;
const baseUrl = (values["base-url"] ?? `${REPO}/releases/download/v${version}`).replace(/\/$/, "");

const bundle = join(root, "target", "release", "bundle");
const installer = join(bundle, "nsis", `TokenRadar_${version}_x64-setup.exe`);
const portable = join(bundle, "portable", `TokenRadar_${version}_x64-portable.zip`);
for (const file of [installer, `${installer}.sig`, portable]) {
  if (!existsSync(file)) throw new Error(`${file} is missing`);
}

// Minisign -----------------------------------------------------------------------------------
// tauri.conf.json holds the .pub file and the .sig file holds the signature file, both base64.
// Public key line: "Ed" + key id (8) + Ed25519 key (32). Signature line: algorithm ("ED":
// BLAKE2b-512 of the file is signed, "Ed": the file itself) + key id (8) + signature (64). The
// global signature covers the signature and the trusted comment.

const lines = (b64) => Buffer.from(b64.trim(), "base64").toString("utf8").split(/\r?\n/);
const SPKI_ED25519 = Buffer.from("302a300506032b6570032100", "hex");

function publicKey() {
  const conf = JSON.parse(readFileSync(join(root, "app", "src-tauri", "tauri.conf.json"), "utf8"));
  const raw = Buffer.from(lines(conf.plugins.updater.pubkey)[1] ?? "", "base64");
  if (raw.length !== 42 || raw.subarray(0, 2).toString() !== "Ed") {
    throw new Error("tauri.conf.json: plugins.updater.pubkey is not a minisign public key");
  }
  const key = createPublicKey({
    key: Buffer.concat([SPKI_ED25519, raw.subarray(10)]),
    format: "der",
    type: "spki",
  });
  return { id: raw.subarray(2, 10), key };
}

function verifySignature(file, signatureB64) {
  const [, sigLine, trustedLine, globalLine] = lines(signatureB64);
  const sig = Buffer.from(sigLine, "base64");
  const alg = sig.subarray(0, 2).toString();
  const pk = publicKey();
  const keyId = (b) => Buffer.from(b).reverse().toString("hex").toUpperCase();
  if (!sig.subarray(2, 10).equals(pk.id)) {
    throw new Error(
      `${basename(file)} is signed with key ${keyId(sig.subarray(2, 10))}, ` +
        `but tauri.conf.json ships key ${keyId(pk.id)}: TAURI_SIGNING_PRIVATE_KEY does not match`,
    );
  }
  const data = readFileSync(file);
  const message = alg === "ED" ? createHash("blake2b512").update(data).digest() : data;
  if (!verify(null, message, pk.key, sig.subarray(10))) {
    throw new Error(`${basename(file)}: the updater signature does not verify`);
  }
  const trusted = trustedLine.replace(/^trusted comment: /, "");
  const global = Buffer.from(globalLine, "base64");
  if (!verify(null, Buffer.concat([sig.subarray(10), Buffer.from(trusted)]), pk.key, global)) {
    throw new Error(`${basename(file)}: the trusted comment's signature does not verify`);
  }
  return keyId(pk.id);
}

const signature = readFileSync(`${installer}.sig`, "utf8").trim();
const keyId = verifySignature(installer, signature);

// Notes ----------------------------------------------------------------------------------------
const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8").replaceAll("\r\n", "\n");
const section = new RegExp(`^## \\[${version.replaceAll(".", "\\.")}\\][^\\n]*\\n([\\s\\S]*?)(?=^## \\[|(?![\\s\\S]))`, "m").exec(
  changelog,
);
if (!section) throw new Error(`CHANGELOG.md has no "## [${version}]" section`);
const notes = section[1].replace(/^\[[^\]]+\]: .*$/gm, "").trim();

// Output ---------------------------------------------------------------------------------------
const out = join(bundle, "release");
mkdirSync(out, { recursive: true });

const latest = {
  version,
  notes,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms: {
    "windows-x86_64": { signature, url: `${baseUrl}/${basename(installer)}` },
  },
};
writeFileSync(join(out, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);
writeFileSync(join(out, "notes.md"), `${notes}\n`);

const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");
const assets = [installer, `${installer}.sig`, portable];
writeFileSync(
  join(out, "SHA256SUMS.txt"),
  assets.map((file) => `${sha256(file)}  ${basename(file)}\n`).join(""),
);
for (const file of assets) copyFileSync(file, join(out, basename(file)));

// Package manifests -----------------------------------------------------------------------------
// Written next to the assets (not uploaded). After the draft is built, bucket/tokenradar.json is
// replaced with the Scoop one and the winget ones go to a winget-pkgs PR (CONTRIBUTING.md).
const manifests = join(out, "manifests");
const releaseUrl = `${REPO}/releases/download/v${version}`;
const date = new Date().toISOString().slice(0, 10);

const scoop = {
  version,
  description: "Tray radar for the usage limits of AI coding agents (Claude Code). Local only.",
  homepage: REPO,
  license: "MIT",
  notes: [
    "TokenRadar runs in portable mode: its data is kept in $persist_dir\\data.",
    "Run 'tokenradar statusline install' to connect Claude Code's status line.",
  ],
  url: `${releaseUrl}/${basename(portable)}`,
  hash: sha256(portable),
  bin: "bin\\tokenradar.exe",
  shortcuts: [["tokenradar-app.exe", "TokenRadar"]],
  persist: "data",
  checkver: { github: REPO },
  autoupdate: {
    url: `${REPO}/releases/download/v$version/TokenRadar_$version_x64-portable.zip`,
    hash: { url: "$baseurl/SHA256SUMS.txt" },
  },
};
mkdirSync(join(manifests, "scoop"), { recursive: true });
writeFileSync(join(manifests, "scoop", "tokenradar.json"), `${JSON.stringify(scoop, null, 4)}\n`);

const ID = "umitkara.TokenRadar";
const SCHEMA = "1.12.0";
const header = (type) =>
  `# yaml-language-server: $schema=https://aka.ms/winget-manifest.${type}.${SCHEMA}.schema.json\n\n`;
const winget = {
  [`${ID}.yaml`]: `${header("version")}PackageIdentifier: ${ID}
PackageVersion: ${version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: ${SCHEMA}
`,
  [`${ID}.installer.yaml`]: `${header("installer")}PackageIdentifier: ${ID}
PackageVersion: ${version}
InstallerType: nullsoft
Scope: user
InstallModes:
- interactive
- silent
- silentWithProgress
UpgradeBehavior: install
Commands:
- tokenradar
ReleaseDate: ${date}
Installers:
- Architecture: x64
  InstallerUrl: ${releaseUrl}/${basename(installer)}
  InstallerSha256: ${sha256(installer).toUpperCase()}
  ProductCode: TokenRadar
ManifestType: installer
ManifestVersion: ${SCHEMA}
`,
  [`${ID}.locale.en-US.yaml`]: `${header("defaultLocale")}PackageIdentifier: ${ID}
PackageVersion: ${version}
PackageLocale: en-US
Publisher: Ümit Kara
PublisherUrl: https://github.com/umitkara
PublisherSupportUrl: ${REPO}/issues
PackageName: TokenRadar
PackageUrl: ${REPO}
License: MIT
LicenseUrl: ${REPO}/blob/main/LICENSE
ShortDescription: Tray radar for the usage limits of AI coding agents.
Description: TokenRadar shows the 5-hour and weekly usage limits of Claude Code in the Windows tray, with sessions, projects and how each session affected each window. Local only, no telemetry.
Moniker: tokenradar
Tags:
- claude-code
- usage-tracker
- rate-limits
- tray
ReleaseNotesUrl: ${REPO}/releases/tag/v${version}
ManifestType: defaultLocale
ManifestVersion: ${SCHEMA}
`,
};
const wingetDir = join(manifests, "winget", "manifests", "u", "umitkara", "TokenRadar", version);
mkdirSync(wingetDir, { recursive: true });
for (const [name, text] of Object.entries(winget)) writeFileSync(join(wingetDir, name), text);

console.log(`release ${version}: signature verified with key ${keyId}; assets in ${out}`);
