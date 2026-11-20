<!--
  Week day × hour heat map (Step 17a): seven rows in the locale's week order, 24 hour columns,
  local time. One hue, light to dark (sequential, dataviz): five steps of the accent mixed into
  the track colour, and an empty cell stays the track. Hovering or focusing the grid names the
  cell under the pointer in the readout line under it.
-->
<script lang="ts">
import { clock, weekdayName } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { HeatCell } from "../../../lib/ipc/bindings";
import { HEAT_STEPS, heatCell, heatStep, toJsDay, weekOrder } from "../windows/view";

interface Props {
  cells: HeatCell[];
  max: number;
  first: number;
  format: (v: number) => string;
}

let { cells, max, first, format }: Props = $props();

let hovered = $state<HeatCell | null>(null);
const order = $derived(weekOrder(first));
const hours = Array.from({ length: 24 }, (_, h) => h);

function tip(c: HeatCell): string {
  return t("patterns-cell-tip", {
    day: weekdayName(locale(), toJsDay(c.weekday)),
    hour: clock(locale(), c.hour, 0, hour12()),
    total: format(c.total ?? 0),
    mean: format(c.mean ?? 0),
    days: c.active_days,
  });
}

function onmove(e: PointerEvent) {
  const i = (e.target as HTMLElement).dataset?.cell;
  if (i != null) hovered = cells[Number(i)] ?? null;
}

function shade(step: number): string {
  return step === 0
    ? "var(--surface-2)"
    : `color-mix(in oklab, var(--accent) ${Math.round((step / HEAT_STEPS) * 90 + 10)}%, var(--surface-2))`;
}
</script>

<div class="heat" role="img" aria-label={t("patterns-heat-label")} onpointermove={onmove} onpointerleave={() => (hovered = null)}>
  <span></span>
  {#each hours as h (h)}
    <span class="hour">{h % 3 === 0 ? clock(locale(), h, 0, hour12()) : ""}</span>
  {/each}
  {#each order as d (d)}
    <span class="wd">{weekdayName(locale(), toJsDay(d))}</span>
    {#each hours as h (h)}
      {@const c = heatCell(cells, d, h)}
      <span
        class="cell"
        class:on={hovered === c}
        style:background={shade(heatStep(c?.total ?? 0, max))}
        data-cell={d * 24 + h}
      ></span>
    {/each}
  {/each}
</div>
<div class="foot">
  <p class="readout" role="status">{hovered ? tip(hovered) : ""}</p>
  <span class="legend" aria-hidden="true">
    {t("patterns-heat-legend-low")}
    {#each Array.from({ length: HEAT_STEPS + 1 }, (_, i) => i) as s (s)}
      <span class="swatch" style:background={shade(s)}></span>
    {/each}
    {t("patterns-heat-legend-high")}
  </span>
</div>

<style>
  .heat {
    display: grid;
    grid-template-columns: 40px repeat(24, minmax(10px, 1fr));
    gap: 2px;
    align-items: center;
  }
  .hour {
    font: 400 10px / 14px var(--font-mono);
    color: var(--text-3);
    white-space: nowrap;
    overflow: visible;
  }
  .wd {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .cell {
    aspect-ratio: 1.4;
    border-radius: 2px;
  }
  .cell.on {
    outline: 2px solid var(--text-1);
    outline-offset: 0;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    min-height: 24px;
    margin-top: var(--space-2);
  }
  .readout {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-1);
  }
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font: var(--type-caption);
    color: var(--text-2);
    white-space: nowrap;
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }
  @media (forced-colors: active) {
    .cell,
    .swatch {
      forced-color-adjust: none;
    }
  }
</style>
