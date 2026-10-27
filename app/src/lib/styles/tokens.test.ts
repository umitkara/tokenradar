import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { contrast, hex, over, parseOklch, type Rgba } from "./color";

const css = readFileSync(resolve(import.meta.dirname, "tokens.css"), "utf8");

/** `--name: light-dark(light, dark)` → [light, dark]. */
function pair(name: string): [Rgba, Rgba] {
  const m = new RegExp(
    `${name}:\\s*light-dark\\((oklch\\([^)]*\\)),\\s*(oklch\\([^)]*\\))\\)`,
  ).exec(css);
  if (!m?.[1] || !m[2]) throw new Error(`${name} is not a light-dark() oklch pair`);
  return [parseOklch(m[1]), parseOklch(m[2])];
}

/*
 * What can sit behind a page. Mica and Acrylic tint the wallpaper heavily; these span a dark
 * and a bright wallpaper for each theme, plus the flat color Mica falls back to when the window
 * loses focus. Acrylic over a bright wallpaper in dark mode is the hardest case.
 */
const BACKDROPS = {
  light: ["oklch(0.84 0.01 250)", "oklch(0.93 0.005 250)", "oklch(1 0 0)"].map(parseOklch),
  dark: ["oklch(0.17 0.005 250)", "oklch(0.27 0.01 250)", "oklch(0.38 0.02 250)"].map(parseOklch),
};
const THEMES = [
  ["light", 0],
  ["dark", 1],
] as const;

function check(token: string, min: number, onBackdrop: boolean) {
  for (const [theme, i] of THEMES) {
    const color = pair(token)[i];
    const card = pair("--surface-1")[i];
    for (const backdrop of BACKDROPS[theme]) {
      const surfaces = [over(card, backdrop)];
      if (onBackdrop) surfaces.push(backdrop);
      for (const surface of surfaces) {
        const ratio = contrast(over(color, surface), surface);
        expect(ratio, `${token} (${theme}) on ${hex(surface)}`).toBeGreaterThanOrEqual(min);
      }
    }
  }
}

describe("design tokens on Mica and Acrylic", () => {
  it("primary and secondary text reach WCAG AA on cards and on the bare backdrop", () => {
    check("--text-1", 4.5, true);
    check("--text-2", 4.5, true);
  });

  it("dimmed text reaches AA on cards", () => {
    check("--text-3", 4.5, false);
  });

  it("signal colors reach 3:1 as data ink", () => {
    for (const s of ["ok", "warn", "crit", "other"]) check(`--signal-${s}`, 3, true);
  });

  it("text on the accent fill reaches AA", () => {
    for (const [theme, i] of THEMES) {
      const ratio = contrast(pair("--on-accent")[i], pair("--signal-ok")[i]);
      expect(ratio, theme).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("text on the opaque Windows 10 surface reaches AA", () => {
    for (const [, i] of THEMES) {
      const solid = pair("--surface-0-solid")[i];
      expect(contrast(pair("--text-2")[i], solid)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("the Rust window colors match --surface-0-solid", () => {
    const rust = readFileSync(
      resolve(import.meta.dirname, "../../../src-tauri/src/chrome.rs"),
      "utf8",
    );
    const color = (name: string) => {
      const m = new RegExp(`${name}: Color = Color\\((\\d+), (\\d+), (\\d+), 255\\)`).exec(rust);
      if (!m) throw new Error(`${name} not found in chrome.rs`);
      return hex({ r: Number(m[1]) / 255, g: Number(m[2]) / 255, b: Number(m[3]) / 255, a: 1 });
    };
    const [light, dark] = pair("--surface-0-solid");
    expect(color("SOLID_LIGHT")).toBe(hex(light));
    expect(color("SOLID_DARK")).toBe(hex(dark));
  });
});
