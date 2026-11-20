<!--
  The session detail's cost chart (Step 16a): cost per bucket as bars stacked by model family,
  over the limit windows the session touched (faint bands, a window that hit its limit tinted).

  - Canvas, because a long session has up to 1,500 buckets. Colors come from the design tokens
    through a probe (`lib/chart/theme.ts`), read again on a theme change.
  - Family colors are fixed (`STACKS`): Opus is the same color on every session. A 1 px surface
    gap separates stacked segments and neighbouring bars once bars are wide enough to show it.
  - Hover shows the bucket's time and each family's cost; the legend under the chart names the
    families with their totals, so identity never rests on color alone.
-->
<script lang="ts">
import { onMount } from "svelte";
import ChartTooltip, { type TooltipRow } from "../../../lib/chart/ChartTooltip.svelte";
import type { ChartBand } from "../../../lib/chart/model";
import { tickLabel, tooltipTime } from "../../../lib/chart/model";
import { resolver } from "../../../lib/chart/theme";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import { usd } from "../../panel/detail";
import { type CostChart, costAxis, costTicks, modelLabel, type StackKey, stackMax } from "./view";

interface Props {
  chart: CostChart;
  bands: readonly ChartBand[];
  height?: number;
}

let { chart, bands, height = 180 }: Props = $props();

const AXIS_W = 52;
const AXIS_H = 20;
const PAD_TOP = 8;
const FONT = '11px "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif';
const TIP_WIDTH = 200;

let host: HTMLDivElement;
let canvas: HTMLCanvasElement;
let width = $state(0);
let themeEpoch = $state(0);
let hover = $state<number | null>(null);
let tip = $state<{ title: string; rows: TooltipRow[]; x: number; y: number } | null>(null);

const from = $derived(chart.xs[0] ?? 0);
const to = $derived((chart.xs[chart.xs.length - 1] ?? 0) + chart.bucketMs);
const axis = $derived(costAxis(stackMax(chart)));
const ticks = $derived(costTicks(from, to));
const plotW = $derived(Math.max(0, width - AXIS_W));
const plotH = $derived(height - AXIS_H - PAD_TOP);

const totals = $derived(
  chart.stacks.map((s) => ({ ...s, total: s.values.reduce((a, b) => a + b, 0) })),
);
const total = $derived(totals.reduce((a, s) => a + s.total, 0));

function familyLabel(key: StackKey): string {
  return key === "other" ? t("session-chart-other") : key.charAt(0).toUpperCase() + key.slice(1);
}

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

const xOf = (at: number) => AXIS_W + ((at - from) / Math.max(1, to - from)) * plotW;
const yOf = (micro: number) => PAD_TOP + plotH - (micro / axis.top) * plotH;

function draw() {
  void themeEpoch;
  if (!canvas || width <= 0) return;
  const dpr = devicePixelRatio || 1;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const read = resolver(host);
  const color = {
    text: read("var(--text-3)"),
    grid: read("var(--stroke)"),
    surface: read("var(--surface-1)"),
    hover: read("color-mix(in oklch, var(--text-1) 6%, transparent)"),
    window: read("color-mix(in oklch, var(--text-1) 4%, transparent)"),
    hit: read("color-mix(in oklch, var(--signal-crit) 14%, transparent)"),
    estimated: read("color-mix(in oklch, var(--text-2) 8%, transparent)"),
    stacks: chart.stacks.map((s) => read(`var(${s.token})`)),
  };
  (read as unknown as { dispose: () => void }).dispose();

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);
  ctx.font = FONT;

  // Window bands behind everything, alternating so neighbouring windows stay apart.
  let alt = false;
  for (const band of bands) {
    const x0 = xOf(Math.max(band.from, from));
    const x1 = xOf(Math.min(band.to, to));
    if (x1 <= x0) continue;
    if (band.kind === "window") {
      alt = !alt;
      if (!alt) continue;
    }
    ctx.fillStyle =
      band.kind === "hit" ? color.hit : band.kind === "estimated" ? color.estimated : color.window;
    ctx.fillRect(x0, PAD_TOP, x1 - x0, plotH);
  }

  // Grid and the cost axis.
  ctx.strokeStyle = color.grid;
  ctx.lineWidth = 1;
  ctx.fillStyle = color.text;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let v = 0; v <= axis.top + axis.step / 2; v += axis.step) {
    const y = Math.round(yOf(v)) + 0.5;
    ctx.beginPath();
    ctx.moveTo(AXIS_W, y);
    ctx.lineTo(width, y);
    ctx.stroke();
    ctx.fillText(money(v), AXIS_W - 8, y);
  }

  // Bars: a slot per bucket, stacked bottom up in `STACKS` order.
  const slot = (chart.bucketMs / Math.max(1, to - from)) * plotW;
  const gap = slot >= 4 ? 1 : 0;
  const barW = Math.max(1, slot - gap);
  chart.xs.forEach((at, i) => {
    const x = xOf(at);
    if (hover === i) {
      ctx.fillStyle = color.hover;
      ctx.fillRect(x, PAD_TOP, slot, plotH);
    }
    let base = 0;
    chart.stacks.forEach((s, k) => {
      const v = s.values[i] ?? 0;
      if (v <= 0) return;
      const y0 = yOf(base);
      const y1 = yOf(base + v);
      base += v;
      const h = y0 - y1 - (gap && base > v ? gap : 0);
      if (h <= 0) return;
      ctx.fillStyle = color.stacks[k] ?? color.text;
      ctx.fillRect(x, y1, barW, Math.max(0.5, h));
    });
  });

  // Time labels, in text ink again (the bars changed the fill).
  ctx.fillStyle = color.text;
  ctx.textAlign = "center";
  ctx.textBaseline = "alphabetic";
  const span = to - from;
  for (const at of ticks) {
    const x = xOf(at);
    if (x < AXIS_W + 16 || x > width - 16) continue;
    ctx.fillText(tickLabel(locale(), hour12(), at, span), x, height - 4);
  }
}

$effect(draw);

function onpointermove(e: PointerEvent) {
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  if (x < AXIS_W || chart.xs.length === 0) {
    hover = null;
    tip = null;
    return;
  }
  const at = from + ((x - AXIS_W) / Math.max(1, plotW)) * (to - from);
  // The bucket under the pointer, or none in a gap between buckets.
  let found: number | null = null;
  for (let i = 0; i < chart.xs.length; i++) {
    const start = chart.xs[i] ?? 0;
    if (start > at) break;
    if (at < start + chart.bucketMs) found = i;
  }
  hover = found;
  if (found == null) {
    tip = null;
    return;
  }
  const rows: TooltipRow[] = [];
  const read = resolver(host);
  for (const s of chart.stacks) {
    const v = s.values[found] ?? 0;
    if (v <= 0) continue;
    rows.push({ label: familyLabel(s.key), value: money(v), color: read(`var(${s.token})`) });
  }
  (read as unknown as { dispose: () => void }).dispose();
  const start = chart.xs[found] ?? 0;
  if (bands.some((b) => b.kind === "hit" && b.from <= start && start < b.to)) {
    rows.push({ label: t("chart-hit-limit"), value: "", color: "", note: true });
  }
  const left = Math.min(Math.max(0, x + 12), Math.max(0, width - TIP_WIDTH));
  tip = { title: tooltipTime(locale(), hour12(), start), rows, x: left, y: PAD_TOP };
}

function onpointerleave() {
  hover = null;
  tip = null;
}

onMount(() => {
  const sizer = new ResizeObserver(([entry]) => {
    width = Math.floor(entry?.contentRect.width ?? 0);
  });
  sizer.observe(host);
  const theme = new MutationObserver(() => themeEpoch++);
  theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  const scheme = matchMedia("(prefers-color-scheme: dark)");
  const onScheme = () => themeEpoch++;
  scheme.addEventListener("change", onScheme);
  return () => {
    sizer.disconnect();
    theme.disconnect();
    scheme.removeEventListener("change", onScheme);
  };
});
</script>

<div class="cost">
  <div class="plot" bind:this={host} style:height="{height}px">
    <div
      class="surface"
      role="img"
      aria-label={t("session-chart-alt", { total: money(total) })}
      {onpointermove}
      {onpointerleave}
    >
      <canvas bind:this={canvas} style:width="{width}px" style:height="{height}px"></canvas>
    </div>
    {#if tip}
      <ChartTooltip {...tip} />
    {/if}
  </div>
  <ul class="legend">
    {#each totals as s (s.key)}
      <li title={s.models.map(modelLabel).join(", ")}>
        <span class="swatch" style:--swatch="var({s.token})" aria-hidden="true"></span>
        <span>{familyLabel(s.key)}</span>
        <span class="readout">{money(s.total)}</span>
      </li>
    {/each}
    {#if bands.some((b) => b.kind === "hit")}
      <li>
        <span class="swatch hit" aria-hidden="true"></span>
        <span>{t("chart-hit-limit")}</span>
      </li>
    {/if}
  </ul>
</div>

<style>
  .cost {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .plot {
    position: relative;
  }
  canvas {
    display: block;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-4);
    margin: 0;
    padding: 0 0 0 52px;
    list-style: none;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .readout {
    color: var(--text-1);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--swatch);
  }
  .swatch.hit {
    background: color-mix(in oklch, var(--signal-crit) 30%, transparent);
  }
  @media (forced-colors: active) {
    .swatch {
      forced-color-adjust: none;
    }
  }
</style>
