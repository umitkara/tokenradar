<!--
  The usage chart (ADR-046, ADR-048): the window's measured readings as a filled curve against
  the time line (dashed diagonal), with the 75/90 thresholds, day (or hour) gridlines and a
  "now" line. Inline SVG with the theme's tokens, so it follows light and dark by itself; no
  chart library in the panel's bundle. As wide as the time strip above it, so days line up.
-->
<script lang="ts">
import { limitName } from "../../lib/components/limit";
import { clock as wallClock } from "../../lib/i18n/format";
import { hour12, locale, t } from "../../lib/i18n/i18n.svelte";
import type { FocusView, LimitView, PaceView } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import { CHART, chartX, chartY, gridlines, position, usagePaths } from "./detail";

interface Props {
  focus: FocusView;
  limit: LimitView;
  pace: PaceView | null;
}

let { focus, limit, pace }: Props = $props();

const start = $derived(focus.window_start);
const end = $derived(focus.resets_at);
const weekly = $derived(focus.periods.length > 0);
const grid = $derived(gridlines(start, end));
const paths = $derived(usagePaths(focus.points, start, end));
const nowX = $derived(chartX(Math.min(clock.now, end), start, end));
const y75 = chartY(75);
const y90 = chartY(90);
const base = chartY(0);
const top = chartY(100);
/** The 5-hour window has no strip above it: its hours are labelled under the chart. */
const hours = $derived(
  weekly
    ? []
    : grid.map((at) => {
        const d = new Date(at);
        return {
          pos: position(at, start, end),
          text: wallClock(locale(), d.getHours(), 0, hour12()),
        };
      }),
);
const alt = $derived(
  t("detail-chart-alt", {
    name: limitName(limit),
    used: limit.percent ?? 0,
    elapsed: pace?.elapsed_percent ?? position(clock.now, start, end),
  }),
);
</script>

{#if paths.line === ""}
  <p class="empty">{t("detail-chart-empty")}</p>
{:else}
  <svg
    class={limit.severity}
    viewBox="0 0 {CHART.width} {CHART.height}"
    preserveAspectRatio="none"
    role="img"
    aria-label={alt}
  >
    {#each grid as at (at)}
      {@const x = chartX(at, start, end)}
      <line class="grid" x1={x} y1={top} x2={x} y2={base} />
    {/each}
    <line class="t75" x1="0" y1={y75} x2={CHART.width} y2={y75} />
    <line class="t90" x1="0" y1={y90} x2={CHART.width} y2={y90} />
    <line class="time" x1="0" y1={base} x2={CHART.width} y2={top} />
    <path class="area" d={paths.area} />
    <path class="use" d={paths.line} />
    <line class="now" x1={nowX} y1={top} x2={nowX} y2={base} />
  </svg>
  {#if hours.length > 0}
    <div class="hours" aria-hidden="true">
      {#each hours as h (h.pos)}
        <span style:left="{h.pos}%">{h.text}</span>
      {/each}
    </div>
  {/if}
{/if}

<style>
  svg {
    display: block;
    width: 100%;
    height: 96px;
    overflow: visible;
    color: var(--signal-ok);
  }
  svg.warn {
    color: var(--signal-warn);
  }
  svg.crit {
    color: var(--signal-crit);
  }
  line,
  path {
    vector-effect: non-scaling-stroke;
  }
  .grid {
    stroke: var(--stroke);
    stroke-width: 1;
  }
  .t75 {
    stroke: var(--signal-warn);
    stroke-width: 1;
    stroke-dasharray: 2 4;
    opacity: 0.5;
  }
  .t90 {
    stroke: var(--signal-crit);
    stroke-width: 1;
    stroke-dasharray: 2 4;
    opacity: 0.5;
  }
  .time {
    stroke: var(--text-3);
    stroke-width: 1.5;
    stroke-dasharray: 5 5;
  }
  .area {
    fill: currentColor;
    fill-opacity: 0.18;
    stroke: none;
  }
  .use {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }
  .now {
    stroke: var(--text-2);
    stroke-width: 1;
  }
  .hours {
    position: relative;
    height: 14px;
    font: var(--type-caption);
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .hours span {
    position: absolute;
    translate: -50% 0;
  }
  .empty {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  @media (forced-colors: active) {
    svg {
      forced-color-adjust: none;
      color: CanvasText;
    }
    .grid,
    .time,
    .now {
      stroke: GrayText;
    }
  }
</style>
