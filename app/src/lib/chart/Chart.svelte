<!--
  The uPlot wrapper (Step 15): a time chart of percentage lines with window bands and faint
  75/90 thresholds, a crosshair and the shared tooltip.

  - Colors come from the design tokens (`theme.ts`); a theme or language change rebuilds the
    chart, since a canvas cannot follow CSS by itself.
  - Sized by a ResizeObserver; built and fed only while on screen (IntersectionObserver), so a
    chart scrolled away or in a hidden window costs nothing.
  - Lines are 2 px, estimates dashed (design-language.md §2). Bands: alternating limit windows,
    a window that reached its limit in the critical tint, an estimated window in grey.
  - No animation (ADR-019): uPlot draws once per change.
-->
<script lang="ts">
import { onMount } from "svelte";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { pct } from "../i18n/format";
import { hour12, locale, t } from "../i18n/i18n.svelte";
import ChartTooltip, { type TooltipRow } from "./ChartTooltip.svelte";
import {
  align,
  type ChartModel,
  latest,
  tickLabel,
  timeTicks,
  tooltipTime,
  valueAt,
} from "./model";
import { type ChartColors, chartColors } from "./theme";

interface Props {
  model: ChartModel;
  /** Plot height in px; the width follows the container. */
  height?: number;
  /** Screen reader summary; defaults to the first line's latest value. */
  label?: string;
}

let { model, height = 200, label }: Props = $props();

let host: HTMLDivElement;
let plot: uPlot | null = null;
let visible = $state(false);
let width = $state(0);
/** Bumped when the page theme changes: rebuild with the new colors. */
let themeEpoch = $state(0);
let tip = $state<{ title: string; rows: TooltipRow[]; x: number; y: number } | null>(null);

const FONT = '11px "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif';
const Y_SPLITS = [0, 25, 50, 75, 100];
const TIP_WIDTH = 180;

const aligned = $derived(align(model.lines));
const empty = $derived(aligned.xs.length === 0);
const summary = $derived.by(() => {
  if (label) return label;
  const first = model.lines[0];
  const last = first ? latest(first) : null;
  return first && last ? t("chart-alt", { name: first.label, pct: last.value }) : t("chart-empty");
});

function drawBands(u: uPlot, colors: ChartColors) {
  const { ctx } = u;
  const { top, height: h } = u.bbox;
  ctx.save();
  let alt = false;
  for (const band of model.bands) {
    const x0 = u.valToPos(Math.max(band.from, model.from), "x", true);
    const x1 = u.valToPos(Math.min(band.to, model.to), "x", true);
    if (x1 <= x0) continue;
    if (band.kind === "window") {
      alt = !alt;
      if (!alt) continue;
    }
    ctx.fillStyle =
      band.kind === "hit"
        ? colors.bandHit
        : band.kind === "estimated"
          ? colors.bandEstimated
          : colors.bandWindow;
    ctx.fillRect(x0, top, x1 - x0, h);
  }
  ctx.restore();
}

function drawThresholds(u: uPlot, colors: ChartColors) {
  const { ctx } = u;
  const { left, width: w } = u.bbox;
  ctx.save();
  ctx.lineWidth = Math.max(1, devicePixelRatio);
  ctx.setLineDash([3 * devicePixelRatio, 4 * devicePixelRatio]);
  for (const value of model.thresholds) {
    const y = Math.round(u.valToPos(value, "y", true)) + 0.5;
    ctx.strokeStyle = value >= 90 ? colors.crit : colors.warn;
    ctx.beginPath();
    ctx.moveTo(left, y);
    ctx.lineTo(left + w, y);
    ctx.stroke();
  }
  ctx.restore();
}

function valueText(value: number, estimated: boolean): string {
  return t("value-pct", { pct: value, estimate: estimated ? "yes" : "no" });
}

function updateTip(u: uPlot, colors: ChartColors) {
  const { left, top, idx } = u.cursor;
  if (idx == null || left == null || top == null || left < 0) {
    tip = null;
    return;
  }
  const at = aligned.xs[idx];
  if (at === undefined) return;
  const rows: TooltipRow[] = [];
  for (const line of model.lines) {
    const p = valueAt(line, at);
    if (!p) continue;
    rows.push({
      label: line.label,
      value: valueText(p.value, Boolean(p.estimated)),
      color: colors.tone[line.tone],
      dashed: p.estimated,
    });
  }
  if (model.bands.some((b) => b.kind === "hit" && b.from <= at && at < b.to)) {
    rows.push({ label: t("chart-hit-limit"), value: "", color: "", note: true });
  }
  if (rows.length === 0) {
    tip = null;
    return;
  }
  const plotLeft = u.bbox.left / devicePixelRatio;
  const plotTop = u.bbox.top / devicePixelRatio;
  const x = plotLeft + left + 12;
  tip = {
    title: tooltipTime(locale(), hour12(), at),
    rows,
    // Flip to the left of the cursor near the right edge.
    x: x + TIP_WIDTH > width ? plotLeft + left - TIP_WIDTH - 12 : x,
    y: plotTop + Math.max(0, top - 16),
  };
}

function build(colors: ChartColors): uPlot {
  const loc = locale();
  const h12 = hour12();
  const span = model.to - model.from;
  const axis = {
    stroke: colors.text,
    font: FONT,
    grid: { stroke: colors.grid, width: 1 },
    ticks: { show: false },
  };
  const opts: uPlot.Options = {
    width,
    height,
    ms: 1,
    pxAlign: 0,
    legend: { show: false },
    scales: {
      x: { time: true, range: () => [model.from, model.to] },
      y: { range: () => [0, model.yMax] },
    },
    axes: [
      {
        ...axis,
        splits: () => timeTicks(model.from, model.to),
        values: (_u, splits) => splits.map((s) => tickLabel(loc, h12, s, span)),
        size: 28,
      },
      {
        ...axis,
        splits: () => Y_SPLITS.filter((v) => v <= model.yMax),
        values: (_u, splits) => splits.map((v) => pct(loc, v)),
        size: 44,
      },
    ],
    series: [
      {},
      ...aligned.series.map((s) => ({
        stroke: colors.tone[s.tone],
        width: 2,
        dash: s.dashed ? [5, 4] : undefined,
        fill: s.dashed ? undefined : colors.fill[s.tone],
        points: { show: false },
        spanGaps: false,
      })),
    ],
    cursor: {
      y: false,
      drag: { x: false, y: false, setScale: false },
      points: { size: 8, width: 2, fill: (u, i) => u.series[i]?.stroke?.toString() ?? colors.text },
    },
    hooks: {
      drawClear: [(u) => drawBands(u, colors)],
      draw: [(u) => drawThresholds(u, colors)],
      setCursor: [(u) => updateTip(u, colors)],
    },
  };
  const data = [aligned.xs, ...aligned.series.map((s) => s.values)] as unknown as uPlot.AlignedData;
  return new uPlot(opts, data, host);
}

function destroy() {
  plot?.destroy();
  plot = null;
  tip = null;
}

// Build (or rebuild) whenever the inputs that shape the canvas change; only while on screen.
$effect(() => {
  void themeEpoch;
  void locale();
  void hour12();
  void aligned;
  void model;
  void height;
  if (!visible || width === 0 || empty) {
    destroy();
    return;
  }
  destroy();
  const built = build(chartColors(host));
  plot = built;
  // uPlot caches the plot's screen rect for the cursor; a chart rebuilt under a resting pointer
  // (theme change) or before layout settled would map the pointer to the wrong time.
  const frame = requestAnimationFrame(() => built.syncRect());
  return () => {
    cancelAnimationFrame(frame);
    destroy();
  };
});

// Width changes only resize: no rebuild.
$effect(() => {
  if (plot && width > 0) {
    plot.setSize({ width, height });
    plot.syncRect();
  }
});

onMount(() => {
  const sizer = new ResizeObserver(([entry]) => {
    width = Math.floor(entry?.contentRect.width ?? 0);
  });
  sizer.observe(host);
  const seen = new IntersectionObserver(([entry]) => {
    visible = entry?.isIntersecting ?? false;
  });
  seen.observe(host);
  // `data-theme` on <html> is the page's theme (lib/settings).
  const theme = new MutationObserver(() => themeEpoch++);
  theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => {
    sizer.disconnect();
    seen.disconnect();
    theme.disconnect();
  };
});
</script>

<div class="chart" style:min-height="{height}px">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="plot" bind:this={host} role="img" aria-label={summary} onpointerleave={() => (tip = null)}></div>
  {#if empty}
    <p class="empty">{t("chart-empty")}</p>
  {/if}
  {#if tip}
    <ChartTooltip {...tip} />
  {/if}
</div>
<!-- Two or more lines: a legend, so identity is never color alone (dataviz). -->
{#if model.lines.length > 1}
  <ul class="legend">
    {#each model.lines as line (line.id)}
      <li><span class="swatch tone-{line.tone}" aria-hidden="true"></span>{line.label}</li>
    {/each}
  </ul>
{/if}

<style>
  .chart {
    position: relative;
    width: 100%;
  }
  .plot {
    width: 100%;
  }
  .legend {
    display: flex;
    gap: var(--space-4);
    margin: var(--space-2) 0 0;
    padding: 0;
    list-style: none;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .swatch {
    width: 12px;
    border-top: 2px solid;
  }
  .tone-primary {
    border-color: var(--signal-ok);
  }
  .tone-secondary {
    border-color: var(--series-2);
  }
  .tone-other {
    border-color: var(--signal-other);
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    font: var(--type-body);
    color: var(--text-2);
  }
  /* uPlot's own crosshair line in the theme's muted ink. */
  .plot :global(.u-cursor-x) {
    border-right: 1px dashed var(--text-3);
  }
  .plot :global(.u-cursor-pt) {
    border-color: var(--surface-1) !important;
  }
</style>
