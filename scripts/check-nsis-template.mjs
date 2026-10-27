// Fails when the installed Tauri CLI embeds a different NSIS template than the one our copy in
// app/src-tauri/windows/installer.nsi was made from (ADR-036). The bundler fills the template
// with variables of its own version, so a stale copy can build a broken installer silently.
//
// The check undoes our marked changes and looks for the result, byte for byte, inside the CLI's
// native module. When it fails: take installer.nsi from the matching `tauri-cli-v*` tag, apply
// the "TokenRadar:" changes again and update UPSTREAM_TAG and the header.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const ours = readFileSync(join(root, "app/src-tauri/windows/installer.nsi"), "utf8");

// Our changes, as [ours, upstream]. The header is removed first.
const CHANGES = [
  [
    `      ; TokenRadar: an uninstall started here is always followed by an install, so the old
      ; uninstaller runs in update mode and keeps the statusline bridge, autostart and data.
      StrCpy $R1 "$R1 /UPDATE"
`,
    `      \${IfThen} $UpdateMode = 1 \${|} StrCpy $R1 "$R1 /UPDATE" \${|} ; append /UPDATE
`,
  ],
  [
    `      ; TokenRadar: %LOCALAPPDATA%\\TokenRadar is the data directory, so the app goes to Programs.
      StrCpy $INSTDIR "$LOCALAPPDATA\\Programs\\\${PRODUCTNAME}"
`,
    `      StrCpy $INSTDIR "$LOCALAPPDATA\\\${PRODUCTNAME}"
`,
  ],
];

let upstream = ours.slice(ours.indexOf("Unicode true"));
for (const [mine, theirs] of CHANGES) {
  if (!upstream.includes(mine)) {
    console.error(`installer.nsi: a marked change is missing or was edited:\n${mine}`);
    process.exit(1);
  }
  upstream = upstream.replace(mine, theirs);
}

const appRequire = createRequire(join(root, "app/package.json"));
const cliRequire = createRequire(appRequire.resolve("@tauri-apps/cli/package.json"));
const native = readFileSync(cliRequire.resolve(`@tauri-apps/cli-${process.platform}-x64-msvc`));
const cliVersion = cliRequire("@tauri-apps/cli/package.json").version;

const found = [upstream, upstream.replaceAll("\n", "\r\n")].some((text) =>
  native.includes(Buffer.from(text, "utf8")),
);
if (!found) {
  console.error(
    `installer.nsi no longer matches the NSIS template of @tauri-apps/cli ${cliVersion}.\n` +
      "Sync it from the matching tauri-cli tag and re-apply the TokenRadar changes.",
  );
  process.exit(1);
}
console.log(`installer.nsi matches the template of @tauri-apps/cli ${cliVersion}.`);
