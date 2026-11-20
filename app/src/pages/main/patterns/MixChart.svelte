<!--
  Model mix by week (Step 17a): each local week's API-equivalent cost split by model family, as
  stacked shares (0–100 %). Colour follows the family (design-language.md), the stack order is
  the backend's (Opus at the bottom). A legend names every family; hovering a week lists its
  shares.
-->
<script lang="ts">
import { pct } from "../../../lib/i18n/format";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import type { MixTrend, ModelFamily } from "../../../lib/ipc/bindings";
import { familyColor, mixShares, nearestIndex, stackPaths } from "../windows/view";

let { trend }: { trend: MixTrend } = $props();

const W = 600;
const H = 140;
let hover = $state<number | null>(null);

const shares = $derived(mixShares(trend));
const paths = $derived(stackPaths(shares, W, H, 100));

function name(f: ModelFamily | null): string {
  return f == null ? t("session-chart-other") : f.charAt(0).toUpperCase() + f.slice(1);
}

function week(ms: number): string {
  return new Date(ms).toLocaleDateString(locale(), { day: "numeric", month: "short" });
}

function tip(i: number): string {
  const w = trend.weeks[i];
  if (!w) return "";
  const parts = trend.families
    .map((f, k) => ({ f, v: shares[k]?.[i] ?? 0 }))
    .filter((p) => p.v > 0)
    .map((p) => `${name(p.f)} ${pct(locale(), p.v)}`)
    .join(", ");
  return t("patterns-mix-tip", { when: week(w.starts_at), parts: parts || t("value-none") });
}

function onmove(e: PointerEvent) {
  const box = (e.currentTarget as SVGElement).getBoundingClientRect();
  hover = nearestIndex((e.clientX - box.left) / Math.max(1, box.width), trend.weeks.length);
}
</script>

<ul class="legend">
  {#each trend.families as f, i (f ?? `other-${i}`)}
    <li><span class="swatch" style:background={familyColor(f)}></span>{name(f)}</li>
  {/each}
</ul>
<svg
  viewBox="0 0 {W} {H}"
  preserveAspectRatio="none"
  role="img"
  aria-label={t("patterns-mix")}
  onpointermove={onmove}
  onpointerleave={() => (hover = null)}
>
  {#each paths as d, i (i)}
    <path
      {d}
      fill={familyColor(trend.families[i] ?? null)}
      stroke="var(--surface-0-solid)"
      stroke-width="2"
      vector-effect="non-scaling-stroke"
    />
  {/each}
  {#if hover != null && trend.weeks.length > 1}
    {@const x = (hover / (trend.weeks.length - 1)) * W}
    <line x1={x} x2={x} y1="0" y2={H} class="cross" vector-effect="non-scaling-stroke" />
  {/if}
</svg>
<div class="axis" aria-hidden="true">
  <span>{trend.weeks[0] ? week(trend.weeks[0].starts_at) : ""}</span>
  <span>{trend.weeks.at(-1) ? week(trend.weeks.at(-1)?.starts_at ?? 0) : ""}</span>
</div>
<p class="readout" role="status">{hover != null ? tip(hover) : ""}</p>
<p class="note">{t("patterns-mix-note")}</p>

<style>
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
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
  svg {
    display: block;
    width: 100%;
    height: 140px;
  }
  .cross {
    stroke: var(--text-1);
    stroke-width: 1;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    font: 400 10px / 14px var(--font-mono);
    color: var(--text-3);
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
