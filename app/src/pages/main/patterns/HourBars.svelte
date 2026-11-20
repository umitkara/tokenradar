<!--
  Time of day (Step 17a): for each hour, the average per weekday next to the average per weekend
  day, on one scale. Two series, so a legend; hovering an hour names both values.
-->
<script lang="ts">
import { clock } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { HourSplit } from "../../../lib/ipc/bindings";
import { barHeight } from "../windows/view";

interface Props {
  hours: HourSplit[];
  format: (v: number) => string;
}

let { hours, format }: Props = $props();

let hovered = $state<HourSplit | null>(null);
const max = $derived(Math.max(0, ...hours.flatMap((h) => [h.weekday ?? 0, h.weekend ?? 0])));

function onmove(e: PointerEvent) {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-hour]");
  if (el) hovered = hours.find((h) => h.hour === Number(el.dataset.hour)) ?? null;
}

function tip(h: HourSplit): string {
  return t("patterns-hour-tip", {
    hour: clock(locale(), h.hour, 0, hour12()),
    weekday: format(h.weekday ?? 0),
    weekend: format(h.weekend ?? 0),
  });
}
</script>

<ul class="legend">
  <li><span class="swatch weekday"></span>{t("patterns-weekday")}</li>
  <li><span class="swatch weekend"></span>{t("patterns-weekend")}</li>
</ul>
<div class="bars" role="img" aria-label={t("patterns-hours")} onpointermove={onmove} onpointerleave={() => (hovered = null)}>
  {#each hours as h (h.hour)}
    <span class="hour" class:on={hovered === h} data-hour={h.hour}>
      <span class="bar weekday" style:height="{barHeight(h.weekday ?? 0, max)}%"></span>
      <span class="bar weekend" style:height="{barHeight(h.weekend ?? 0, max)}%"></span>
    </span>
  {/each}
</div>
<div class="axis" aria-hidden="true">
  {#each hours as h (h.hour)}
    <span>{h.hour % 3 === 0 ? clock(locale(), h.hour, 0, hour12()) : ""}</span>
  {/each}
</div>
<p class="readout" role="status">{hovered ? tip(hovered) : ""}</p>
<p class="note">{t("patterns-hours-note")}</p>

<style>
  .legend {
    display: flex;
    gap: var(--space-3);
    margin: 0 0 var(--space-2);
    padding: 0;
    list-style: none;
    font: var(--type-caption);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  .bars,
  .axis {
    display: grid;
    grid-template-columns: repeat(24, 1fr);
    gap: 4px;
  }
  .bars {
    height: 120px;
    border-bottom: 1px solid var(--stroke);
  }
  .hour {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding: 0 1px;
    border-radius: 2px;
  }
  .hour.on {
    background: var(--surface-2);
  }
  .bar {
    flex: 1 1 0;
    border-radius: 2px 2px 0 0;
  }
  .weekday {
    background: var(--series-2);
  }
  .weekend {
    background: var(--series-3);
  }
  .axis span {
    font: 400 10px / 14px var(--font-mono);
    color: var(--text-3);
    white-space: nowrap;
  }
  .readout {
    margin: var(--space-1) 0 0;
    min-height: 18px;
    font: var(--type-caption);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
</style>
