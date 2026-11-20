// Chart colors from the design tokens. uPlot draws on a canvas, which cannot read CSS variables,
// and the tokens are `light-dark()` pairs whose computed custom-property value is the unresolved
// text. A probe element inside the chart resolves each one through `color`, so the browser picks
// the active side of `color-scheme`. Read again on every theme change (the chart is rebuilt).

import type { Tone } from "./model";

export interface ChartColors {
  tone: Record<Tone, string>;
  /** Tone colors at low alpha, for the area under a line. */
  fill: Record<Tone, string>;
  text: string;
  muted: string;
  grid: string;
  warn: string;
  crit: string;
  /** Faint bands: alternating windows, a hit-limit window, an estimated window. */
  bandWindow: string;
  bandHit: string;
  bandEstimated: string;
}

const TONE_TOKENS: Record<Tone, string> = {
  primary: "--signal-ok",
  secondary: "--series-2",
  other: "--signal-other",
};

/** Resolves CSS color expressions to computed colors inside `host`. */
export function resolver(host: HTMLElement): (css: string) => string {
  const probe = document.createElement("span");
  probe.style.display = "none";
  host.appendChild(probe);
  const cache = new Map<string, string>();
  const read = (css: string) => {
    const hit = cache.get(css);
    if (hit) return hit;
    probe.style.color = "";
    probe.style.color = css;
    const value = getComputedStyle(probe).color;
    cache.set(css, value);
    return value;
  };
  return Object.assign(read, { dispose: () => probe.remove() });
}

const mix = (token: string, pct: number) =>
  `color-mix(in oklch, var(${token}) ${pct}%, transparent)`;

export function chartColors(host: HTMLElement): ChartColors {
  const read = resolver(host);
  const tone = (pick: (token: string) => string) =>
    Object.fromEntries(
      (Object.keys(TONE_TOKENS) as Tone[]).map((t) => [t, pick(TONE_TOKENS[t])]),
    ) as Record<Tone, string>;
  const colors: ChartColors = {
    tone: tone((t) => read(`var(${t})`)),
    fill: tone((t) => read(mix(t, 14))),
    text: read("var(--text-2)"),
    muted: read("var(--text-3)"),
    grid: read("var(--stroke)"),
    warn: read(mix("--signal-warn", 60)),
    crit: read(mix("--signal-crit", 60)),
    bandWindow: read(mix("--text-1", 4)),
    bandHit: read(mix("--signal-crit", 14)),
    bandEstimated: read(mix("--text-2", 8)),
  };
  (read as unknown as { dispose: () => void }).dispose();
  return colors;
}
