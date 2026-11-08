import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import type {
  FontError,
  NerdFontView,
  NerdTier,
  TargetId,
  TargetProblem,
  TargetState,
  TerminalKind,
} from "../../../lib/ipc/bindings";
import {
  canSelect,
  fontErrorKey,
  fontStatusKey,
  megabytes,
  percent,
  previewFont,
  targetKey,
  targetTerminal,
  terminalKey,
} from "./font";

const FONT: NerdFontView = { family: "CaskaydiaMono Nerd Font Mono", tier: "full", mono: true };

function target(patch: Partial<TargetState> = {}): TargetState {
  return {
    id: "windows_terminal",
    path: "C:/x/settings.json",
    problem: null,
    font: "Cascadia Mono",
    written: false,
    tier: "none",
    overrides: 0,
    can_undo: false,
    ...patch,
  };
}

describe("terminal font helper", () => {
  it("offers selecting only where another font is in effect", () => {
    expect(canSelect(target(), FONT)).toBe(true);
    expect(canSelect(target({ font: "caskaydiamono nerd font mono" }), FONT)).toBe(false);
    expect(canSelect(target(), null)).toBe(false);
    expect(canSelect(target({ problem: "legacy" }), FONT)).toBe(false);
  });

  it("formats size and progress", () => {
    expect(megabytes(3_577_856, "en")).toBe("3.4 MB");
    expect(megabytes(3_577_856, "tr")).toBe("3,4 MB");
    expect(percent(0, 0)).toBe(0);
    expect(percent(1_788_928, 3_577_856)).toBe(50);
    expect(percent(5, 4)).toBe(100);
  });

  it("puts the installed font first in the preview", () => {
    expect(previewFont(null)).toBeNull();
    expect(previewFont(FONT)).toBe(
      '"CaskaydiaMono Nerd Font Mono", "Cascadia Mono", Consolas, monospace',
    );
    expect(previewFont({ ...FONT, family: 'A"B' })).toMatch(/^"AB", /);
  });

  it("maps settings files to their terminal", () => {
    expect(targetTerminal("vscode_insiders")).toBe("vscode");
    expect(targetTerminal("windows_terminal_unpackaged")).toBe("windows_terminal");
  });

  it("has a message for every key it builds", () => {
    const ftl = (lang: string) =>
      readFileSync(
        resolve(import.meta.dirname, `../../../../../locales/${lang}/settings.ftl`),
        "utf8",
      );
    const errors: FontError["error"][] = [
      "busy",
      "cancelled",
      "connect",
      "timeout",
      "status",
      "request",
      "checksum",
      "archive",
      "unsupported",
      "no_font",
      "invalid_settings",
      "legacy_settings",
      "changed_meanwhile",
      "stale_plan",
      "io",
    ];
    const targets: TargetId[] = [
      "windows_terminal",
      "windows_terminal_preview",
      "windows_terminal_unpackaged",
      "vscode",
      "vscode_insiders",
    ];
    const kinds: TerminalKind[] = ["windows_terminal", "vscode", "other"];
    const tiers: NerdTier[] = ["none", "old", "full"];
    const problems: TargetProblem[] = ["unreadable", "invalid", "legacy"];
    const keys = [
      ...errors.map((error) => fontErrorKey({ error } as FontError)),
      ...targets.map(targetKey),
      ...kinds.map(terminalKey),
      ...tiers.map((tier) => fontStatusKey(tier === "none" ? null : { ...FONT, tier })),
      ...tiers.map((tier) => `font-tier-${tier}`),
      ...problems.map((p) => `font-problem-${p}`),
    ];
    for (const lang of ["en", "tr"]) {
      const text = ftl(lang);
      for (const key of keys) expect(text, `${lang}: ${key}`).toMatch(new RegExp(`^${key} =`, "m"));
    }
  });
});
