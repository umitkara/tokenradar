// Message keys and small formatting for the terminal font helper (Step 12o). The backend decides
// what is installed and selected; this only names it.

import type {
  FontError,
  NerdFontView,
  NerdTier,
  TargetId,
  TargetState,
  TerminalKind,
} from "../../../lib/ipc/bindings";

export function fontErrorKey(error: FontError): string {
  return `font-error-${error.error.replaceAll("_", "-")}`;
}

export function targetKey(id: TargetId): string {
  return `font-target-${id.replaceAll("_", "-")}`;
}

export function terminalKey(kind: TerminalKind): string {
  return `font-terminal-${kind.replaceAll("_", "-")}`;
}

/** The installed font's line: none, icons without the 3.4 progress glyphs, or complete. */
export function fontStatusKey(font: NerdFontView | null): string {
  const tier: NerdTier = font?.tier ?? "none";
  return `font-status-${tier}`;
}

/** Whether selecting the installed font in `target` would change anything worth offering. */
export function canSelect(target: TargetState, font: NerdFontView | null): boolean {
  if (!font || target.problem) return false;
  return target.font?.toLowerCase() !== font.family.toLowerCase();
}

/** `3.4 MB` in the UI language (`3,4 MB` in Turkish). */
export function megabytes(bytes: number, locale: string): string {
  const mb = bytes / (1024 * 1024);
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(mb)} MB`;
}

/** Download progress in whole percent. */
export function percent(done: number, total: number): number {
  return total > 0 ? Math.min(100, Math.floor((done * 100) / total)) : 0;
}

/**
 * The preview's font list with the installed Nerd Font first, so the preview draws the icons
 * the user's terminal will draw. `null` keeps the stylesheet's list.
 */
export function previewFont(font: NerdFontView | null): string | null {
  if (!font) return null;
  const quoted = `"${font.family.replaceAll('"', "")}"`;
  return `${quoted}, "Cascadia Mono", Consolas, monospace`;
}

/** The width of `text` drawn in a CSS font list. */
export type Measure = (fontList: string, text: string) => number;

const canvasMeasure: Measure = (fontList, text) => {
  const ctx = document.createElement("canvas").getContext("2d");
  if (!ctx) return 0;
  ctx.font = `20px ${fontList}`;
  return ctx.measureText(text).width;
};

/**
 * Whether this window can draw `family`. A browser sees only the fonts that were installed when
 * it started, so a font installed meanwhile needs TokenRadar restarted before the preview shows
 * it. Missing, both lists fall back and measure like the bare fallbacks.
 */
export function canDraw(family: string, measure: Measure = canvasMeasure): boolean {
  const probe = "";
  const quoted = `"${family.replaceAll('"', "")}"`;
  const mono = measure(`${quoted}, monospace`, probe) !== measure("monospace", probe);
  const serif = measure(`${quoted}, serif`, probe) !== measure("serif", probe);
  return mono || serif;
}

/** The terminal a settings file belongs to. */
export function targetTerminal(id: TargetId): TerminalKind {
  return id.startsWith("vscode") ? "vscode" : "windows_terminal";
}
