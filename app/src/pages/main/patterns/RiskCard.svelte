<!--
  Limit risk (Step 17a, ADR-070): which start hours of 5-hour windows end at the limit. Only for
  accounts with 5-hour windows; without them, or with fewer than the minimum, it says why
  instead of drawing. The sentence comes from the backend's span (or its absence); the bars show
  each start hour's windows, with the share that hit the limit in the critical colour and the
  share that reached 90 % lighter. Both carry words in the legend, never colour alone.
-->
<script lang="ts">
import EmptyState from "../../../lib/components/EmptyState.svelte";
import { clock } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { LimitRisk, RiskHour } from "../../../lib/ipc/bindings";
import { barHeight } from "../windows/view";

let { risk }: { risk: LimitRisk } = $props();

let hovered = $state<RiskHour | null>(null);
const max = $derived(Math.max(0, ...risk.hours.map((h) => h.windows)));

function hour(h: number): string {
  return clock(locale(), h % 24, 0, hour12());
}

function onmove(e: PointerEvent) {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-hour]");
  if (el) hovered = risk.hours.find((h) => h.hour === Number(el.dataset.hour)) ?? null;
}

function tip(h: RiskHour): string {
  return t("patterns-risk-tip", { hour: hour(h.hour), n: h.windows, hit: h.hit, high: h.high });
}
</script>

{#if !risk.any}
  <EmptyState title={t("patterns-risk-none-title")} body={t("patterns-risk-none-body")} />
{:else if !risk.enough}
  <p class="note">{t("patterns-risk-few", { n: risk.windows, min: risk.minimum })}</p>
{:else}
  <p class="statement">
    {#if risk.span}
      {t("patterns-risk-statement", {
        from: hour(risk.span.from_hour),
        to: hour(risk.span.to_hour),
        hit: risk.span.hit,
        n: risk.span.windows,
      })}
    {:else}
      {t("patterns-risk-flat", { hit: risk.hit, n: risk.windows })}
    {/if}
  </p>
  <ul class="legend">
    <li><span class="swatch hit"></span>{t("patterns-risk-hit")}</li>
    <li><span class="swatch high"></span>{t("patterns-risk-high")}</li>
    <li><span class="swatch all"></span>{t("patterns-risk-windows")}</li>
  </ul>
  <div class="bars" role="img" aria-label={t("patterns-risk")} onpointermove={onmove} onpointerleave={() => (hovered = null)}>
    {#each risk.hours as h (h.hour)}
      <span class="col" class:on={hovered === h} data-hour={h.hour}>
        <span class="track" style:height="{barHeight(h.windows, max)}%">
          <span class="bar-high" style:height="{h.windows > 0 ? (h.high / h.windows) * 100 : 0}%">
            <span class="bar-hit" style:height="{h.high > 0 ? (h.hit / h.high) * 100 : 0}%"></span>
          </span>
        </span>
      </span>
    {/each}
  </div>
  <div class="axis" aria-hidden="true">
    {#each risk.hours as h (h.hour)}
      <span>{h.hour % 3 === 0 ? hour(h.hour) : ""}</span>
    {/each}
  </div>
  <p class="readout" role="status">{hovered ? tip(hovered) : ""}</p>
{/if}

<style>
  .statement {
    margin: 0 0 var(--space-2);
    font: var(--type-body);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin: 0 0 var(--space-2);
    padding: 0;
    list-style: none;
    font: var(--type-caption);
  }
  .legend li {
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  .swatch.hit,
  .bar-hit {
    background: var(--signal-crit);
  }
  .swatch.high,
  .bar-high {
    background: color-mix(in srgb, var(--signal-crit) 35%, var(--surface-2));
  }
  .swatch.all {
    background: var(--stroke);
  }
  .bars,
  .axis {
    display: grid;
    grid-template-columns: repeat(24, 1fr);
    gap: 3px;
  }
  .bars {
    height: 96px;
    border-bottom: 1px solid var(--stroke);
  }
  .col {
    display: flex;
    align-items: flex-end;
    border-radius: 2px;
  }
  .col.on {
    background: var(--surface-2);
  }
  .track {
    display: flex;
    align-items: flex-end;
    width: 100%;
    background: var(--stroke);
    border-radius: 2px 2px 0 0;
  }
  .bar-high {
    display: flex;
    align-items: flex-end;
    width: 100%;
    border-radius: 2px 2px 0 0;
  }
  .bar-hit {
    width: 100%;
    border-radius: 2px 2px 0 0;
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
</style>
